//! End-to-end slice cases (tests.md §5): real adapters, synthetic installs under a temp
//! directory, one temp directory standing in for the shared provider folder.

#[cfg(test)]
mod tests {
    use crate::control::{Command, CommandError, CommandOutcome};
    use crate::removal_cycle::RemovalStore;
    use crate::*;
    use std::path::{Path, PathBuf};
    use wow_sync_adapters::fs_install::InstallArea;
    use wow_sync_adapters::localstate::LocalState;
    use wow_sync_adapters::testkit::{SyntheticInstall, TempDir};
    use wow_sync_core::authority::{decode_authority, AuthorityRecord};
    use wow_sync_core::decide::encode_authority;
    use wow_sync_core::hash::sha256;
    use wow_sync_core::ids::{ComputerId, EnrollmentId, GroupId, Sha256};
    use wow_sync_core::kinds::{EnrollmentRef, Issuer, Retirement};
    use wow_sync_core::layout;
    use wow_sync_core::ledger::{derive, read_index_chain, IndexHead, InvalidReason, Status};
    use wow_sync_core::plan::Activity;
    use wow_sync_core::records::{decode, CodeState};

    struct Pc {
        install: SyntheticInstall,
        cfg: ServiceConfig,
        svc: SliceService,
        act: ActivityHandle,
    }

    struct Lab {
        tmp: TempDir,
        shared: PathBuf,
    }

    impl Lab {
        fn new() -> Lab {
            let tmp = TempDir::new();
            let shared = tmp.join("shared");
            std::fs::create_dir(&shared).unwrap();
            Lab { tmp, shared }
        }

        fn pc(&self, name: &str) -> Pc {
            self.pc_with(name, "_forever_", "wowf")
        }

        fn pc_with(&self, name: &str, folder: &str, product: &str) -> Pc {
            let install = SyntheticInstall::with_product(self.tmp.path(), name, folder, product);
            let state = self.tmp.join(&format!("{name}-state"));
            let cfg = ServiceConfig {
                install_root: install.root.clone(),
                variant_folder: folder.to_string(),
                shared_root: self.shared.clone(),
                state_dir: state,
                display_name: name.to_string(),
                app_version: "0.0.0-e2e".into(),
                recent_ms: 0,
            };
            let (src, act) = ScriptedActivity::new(Activity::NotRunning);
            let svc = SliceService::with_activity(cfg.clone(), Box::new(src)).unwrap();
            Pc {
                install,
                cfg,
                svc,
                act,
            }
        }
    }

    impl Pc {
        fn set_build(&self, build: &str) {
            std::fs::write(
                self.install.root.join(".build.info"),
                format!(
                    "Branch!STRING:0|Active!DEC:1|Product!STRING:0|Version!STRING:0\nus|1|wowf|{build}\n"
                ),
            )
            .unwrap();
        }

        /// A tag ending in a digit N is version N+1.0 so later tags are demonstrably newer.
        fn addon(&self, name: &str, tag: &str) {
            let n = tag
                .chars()
                .last()
                .and_then(|c| c.to_digit(10))
                .map_or(1, |d| d + 1);
            self.addon_with(
                name,
                tag,
                &format!("## Interface: 11508\n## Version: {n}.0\n"),
            );
        }

        fn addon_with(&self, name: &str, tag: &str, toc: &str) {
            self.install.add_addon(
                name,
                &format!("{name}.toc"),
                toc,
                &[("main.lua", tag.as_bytes())],
            );
            self.install.set_old_mtimes();
        }

        fn read(&self, rel: &str) -> Option<String> {
            std::fs::read_to_string(self.install.addons.join(rel)).ok()
        }

        fn confirm_request(&self, join: JoinChoice) -> EnrollRequest {
            let p = self.svc.inspect();
            EnrollRequest {
                join,
                identity_confirmed: true,
                scope_confirmed: true,
                digest: p.digest,
                display_name: self.cfg.display_name.clone(),
            }
        }

        fn create(&mut self) -> GroupId {
            let req = self.confirm_request(JoinChoice::New {
                name: "Slice".into(),
            });
            self.svc.enroll(&req).unwrap()
        }

        fn join(&mut self, g: &GroupId) {
            let req = self.confirm_request(JoinChoice::Existing(g.clone()));
            self.svc.enroll(&req).unwrap();
        }

        fn journals(&self) -> usize {
            InstallArea::open(&self.install.variant_dir)
                .unwrap()
                .read_journals()
                .unwrap()
                .len()
        }

        fn restart(&mut self) {
            let (src, act) = ScriptedActivity::new(Activity::NotRunning);
            self.svc = SliceService::with_activity(self.cfg.clone(), Box::new(src)).unwrap();
            self.act = act;
        }
    }

    fn synced(pc: &mut Pc) -> SyncReport {
        pc.svc.sync_once().unwrap()
    }

    fn computer_id(pc: &Pc) -> String {
        let st = LocalState::open_at(&pc.cfg.state_dir).unwrap();
        let c = String::from_utf8(st.read("computer.json", 65536).unwrap().unwrap()).unwrap();
        c.split('"')
            .find(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .unwrap()
            .to_string()
    }

    fn provider_state_files(lab: &Lab, g: &GroupId, pc: &Pc) -> usize {
        let id = computer_id(pc);
        let dir = lab
            .shared
            .join("WoWSync")
            .join(format!("groups/{g}/members/{id}/state"));
        std::fs::read_dir(dir).map(|d| d.count()).unwrap_or(0)
    }

    fn remove_blob(lab: &Lab, g: &GroupId, content: &[u8]) {
        let rel = layout::blob(g, &sha256(content));
        let p: PathBuf = rel
            .split('/')
            .fold(lab.shared.join("WoWSync"), |a, s| a.join(s));
        std::fs::remove_file(p).unwrap();
    }

    fn texts(r: &SyncReport) -> String {
        let s = &r.status;
        format!(
            "{} | {} | {} | {}",
            s.headline,
            s.packages
                .iter()
                .map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join(" ; "),
            s.members.join(" ; "),
            s.blocks.join(" ; ")
        )
    }

    fn same_tree(a: &Path, b: &Path) -> bool {
        fn list(p: &Path, base: &Path, out: &mut Vec<(String, Vec<u8>)>) {
            for e in std::fs::read_dir(p).unwrap().flatten() {
                let q = e.path();
                if q.is_dir() {
                    list(&q, base, out);
                } else {
                    out.push((
                        q.strip_prefix(base).unwrap().to_string_lossy().into_owned(),
                        std::fs::read(&q).unwrap(),
                    ));
                }
            }
        }
        let (mut x, mut y) = (vec![], vec![]);
        list(a, a, &mut x);
        list(b, b, &mut y);
        x.sort();
        y.sort();
        x == y
    }

    #[test]
    fn two_installs_publish_and_apply_through_temp_provider() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        a.addon("Beta", "beta-v1");
        let g = a.create();
        let r = synced(&mut a);
        assert!(r.published.is_some(), "{r:?}");
        b.join(&g);
        let r = synced(&mut b);
        assert_eq!(r.applied.len(), 2, "{r:?}");
        assert!(same_tree(&a.install.addons, &b.install.addons));
        assert_eq!(b.journals(), 0);
        // Idempotent when nothing changed.
        let r = synced(&mut b);
        assert!(r.applied.is_empty(), "{r:?}");
    }

    fn other_build_update_applies(candidate_toc: &str, local_has_addon: bool) {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        b.set_build("1.0.0.2");
        a.addon_with("Alpha", "alpha-v1", candidate_toc);
        if local_has_addon {
            b.addon("Alpha", "alpha-v0");
        }
        b.addon("Beta", "beta-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        let r = synced(&mut b);
        assert_eq!(r.applied.len(), 1, "{}", texts(&r));
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
    }

    #[test]
    fn different_build_same_interface_applies() {
        other_build_update_applies("## Interface: 11508\n## Version: 2.0\n", true);
    }

    #[test]
    fn different_build_different_interface_applies() {
        other_build_update_applies("## Interface: 99999\n## Version: 2.0\n", true);
    }

    #[test]
    fn different_build_malformed_interface_applies() {
        other_build_update_applies("## Interface: 11508,garbage\n## Version: 2.0\n", true);
    }

    #[test]
    fn different_build_missing_interface_applies() {
        other_build_update_applies("## Version: 2.0\n", true);
    }

    #[test]
    fn different_build_new_addon_applies() {
        other_build_update_applies("## Interface: 99999\n## Version: 2.0\n", false);
    }
    #[test]
    fn independent_updates_converge() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        b.addon("Beta", "beta-v1");
        let g = a.create();
        b.join(&g);
        for _ in 0..3 {
            synced(&mut a);
            synced(&mut b);
        }
        assert!(same_tree(&a.install.addons, &b.install.addons));
        assert_eq!(a.read("Beta/main.lua").as_deref(), Some("beta-v1"));
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
    }

    #[test]
    fn ambiguous_addon_stops_only_that_addon() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Amb", "from-a");
        a.addon("Good", "good-v1");
        b.addon("Amb", "from-b");
        let g = a.create();
        b.join(&g);
        let mut last = SyncReport::default();
        for _ in 0..3 {
            synced(&mut a);
            last = synced(&mut b);
        }
        assert_eq!(a.read("Amb/main.lua").as_deref(), Some("from-a"));
        assert_eq!(b.read("Amb/main.lua").as_deref(), Some("from-b"));
        assert_eq!(b.read("Good/main.lua").as_deref(), Some("good-v1"));
        assert!(
            last.status
                .packages
                .iter()
                .any(|p| p.key == "amb" && p.text.contains("cannot tell which version is newer")),
            "{}",
            texts(&last)
        );
    }

    fn suite_pair(a: &Pc) {
        a.addon("Suite", "suite-v1");
        a.addon_with(
            "SuiteData",
            "data-v1",
            "## Interface: 11508\n## Version: 1.0\n## Dependencies: Suite\n",
        );
    }

    #[test]
    fn partial_suite_on_target_is_not_mixed() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        suite_pair(&a);
        b.addon("Suite", "suite-v0");
        let g = a.create();
        synced(&mut a);
        remove_blob(&lab, &g, b"data-v1");
        b.join(&g);
        let r = synced(&mut b);
        assert_eq!(
            b.read("Suite/main.lua").as_deref(),
            Some("suite-v0"),
            "{r:?}"
        );
        assert!(b.read("SuiteData/main.lua").is_none());
    }

    #[test]
    fn empty_peer_does_not_receive_partial_suite() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        suite_pair(&a);
        let g = a.create();
        synced(&mut a);
        remove_blob(&lab, &g, b"suite-v1");
        b.join(&g);
        let r = synced(&mut b);
        assert!(b.read("Suite/main.lua").is_none(), "{r:?}");
        assert!(b.read("SuiteData/main.lua").is_none(), "{r:?}");
        assert_eq!(b.journals(), 0);
    }

    #[test]
    fn game_running_on_target_blocks_apply_then_resumes() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        b.act.set(Activity::Running { pids: vec![4242] });
        let r = synced(&mut b);
        assert!(b.read("Alpha/main.lua").is_none(), "{r:?}");
        assert!(
            r.apply_note.as_deref().unwrap_or("").contains("running"),
            "{r:?}"
        );
        b.act.set(Activity::Unknown("probe failed".into()));
        synced(&mut b);
        assert!(b.read("Alpha/main.lua").is_none());
        b.act.set(Activity::NotRunning);
        let r = synced(&mut b);
        assert_eq!(
            b.read("Alpha/main.lua").as_deref(),
            Some("alpha-v1"),
            "{r:?}"
        );
    }

    #[test]
    fn unfamiliar_variant_blocked_until_confirmed() {
        let lab = Lab::new();
        let mut t = lab.pc_with("t", "_test_", "wow_test");
        t.addon("Alpha", "alpha-v1");
        let p = t.svc.inspect();
        assert_eq!(p.kind, PreviewKind::Unfamiliar, "{p:?}");
        assert!(p.needs_guided_confirmation && p.digest.is_some());
        let mut req = t.confirm_request(JoinChoice::New { name: "T".into() });
        req.identity_confirmed = false;
        assert!(matches!(t.svc.enroll(&req), Err(ServiceError::Refused(_))));
        req.identity_confirmed = true;
        req.scope_confirmed = false;
        assert!(matches!(t.svc.enroll(&req), Err(ServiceError::Refused(_))));
        req.scope_confirmed = true;
        req.digest = Some(sha256(b"some other scope"));
        assert!(matches!(t.svc.enroll(&req), Err(ServiceError::Refused(_))));
        // Nothing was written for the refused attempts (G1: inspection and refusal are read-only).
        assert!(!lab.shared.join("WoWSync").exists());
        assert!(!t.cfg.state_dir.join("enrollment.json").exists());
        req.digest = t.svc.inspect().digest;
        t.svc.enroll(&req).unwrap();
        assert!(t.cfg.state_dir.join("enrollment.json").exists());
    }

    #[test]
    fn join_refused_for_other_product() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let g = a.create();
        let mut t = lab.pc_with("t", "_test_", "wow_test");
        let req = t.confirm_request(JoinChoice::Existing(g.clone()));
        let e = t.svc.enroll(&req).unwrap_err();
        assert!(matches!(e, ServiceError::Refused(_)), "{e}");
        let members = lab
            .shared
            .join("WoWSync")
            .join(format!("groups/{g}/members"));
        assert_eq!(std::fs::read_dir(members).unwrap().count(), 1);
        assert!(!t.cfg.state_dir.join("enrollment.json").exists());
    }

    #[test]
    fn enrolled_unfamiliar_client_keeps_syncing_after_new_folder() {
        let lab = Lab::new();
        let mut t = lab.pc_with("t", "_test_", "wow_test");
        t.addon("Alpha", "alpha-v1");
        t.create();
        synced(&mut t);
        t.addon("Beta", "beta-v1");
        let r = t
            .svc
            .sync_once()
            .expect("later folder must not stale the enrollment");
        assert!(r.published.is_some(), "{r:?}");
        assert!(t.svc.status().reconfirm.is_none());
    }

    #[test]
    fn blocked_folder_is_not_published_and_shows_not_synced() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Good", "good-v1");
        a.install.add_addon(
            "Blocked",
            "Blocked_Foreign.toc",
            "## Interface: 11508\n## Version: 9.0\n",
            &[("main.lua", b"blocked-secret")],
        );
        a.install.set_old_mtimes();
        let g = a.create();
        let r = synced(&mut a);
        assert!(r.published.is_some(), "{r:?}");
        let rel = layout::blob(&g, &sha256(b"blocked-secret"));
        let p: PathBuf = rel
            .split('/')
            .fold(lab.shared.join("WoWSync"), |x, s| x.join(s));
        assert!(!p.exists(), "blocked folder content must not be uploaded");
        let line = r
            .status
            .packages
            .iter()
            .find(|p| p.key == "blocked")
            .map(|p| p.text.clone())
            .unwrap_or_default();
        assert!(line.contains("not synced"), "{line} / {}", texts(&r));
        assert!(!line.contains("up to date"), "{line}");
    }

    #[test]
    fn enrollment_preview_discloses_scope_and_uses_canonical_label() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let p = a.svc.inspect();
        let all = p.disclosures.join("\n");
        assert!(all.contains("AddOns"), "{all}");
        assert!(all.contains("recovery"), "{all}");
        assert!(all.contains(&*a.install.root.to_string_lossy()), "{all}");
        assert!(p.identity_line.contains("Forever"), "{}", p.identity_line);
        assert!(
            !p.identity_line.contains("Classic Era"),
            "{}",
            p.identity_line
        );
        assert!(a.svc.status().who_line().is_none());
    }

    #[test]
    fn ambiguity_status_names_versions_and_computers() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Amb", "from-a");
        b.addon("Amb", "from-b");
        let g = a.create();
        b.join(&g);
        let mut last = SyncReport::default();
        for _ in 0..3 {
            synced(&mut a);
            last = synced(&mut b);
        }
        let t = texts(&last);
        assert!(
            t.contains("on this computer")
                && t.contains(" on a")
                && !t.contains("another computer"),
            "{t}"
        );
    }

    #[test]
    fn running_game_headline_and_applied_line_name_source() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        b.act.set(Activity::Running { pids: vec![4242] });
        let r = synced(&mut b);
        assert!(
            r.status.headline.contains("Waiting") && r.status.headline.contains("WoW is running"),
            "{}",
            r.status.headline
        );
        b.act.set(Activity::NotRunning);
        let r = synced(&mut b);
        let line = r
            .status
            .packages
            .iter()
            .map(|p| p.text.clone())
            .collect::<String>();
        assert!(line.contains("from a "), "{line}");
        assert!(!line.contains("from computer"), "{line}");
    }

    #[test]
    fn typed_rows_use_titles_and_names_and_never_ids() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon_with(
            "Alpha",
            "alpha-v1",
            "## Interface: 11508\n## Title: |cff00ff00Alpha Bars|r\n## Version: 3.0\n",
        );
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        b.act.set(Activity::Running { pids: vec![4242] });
        let r = synced(&mut b);
        let d = r.status.detail.expect("typed detail");
        let row = &d.packages[0];
        assert_eq!(row.title, "Alpha", "not installed yet: folder name");
        assert_eq!(row.version_target.as_deref(), Some("3.0"));
        assert_eq!(row.sources, vec!["a".to_string()]);
        let b_id = computer_id(&b);
        let a_id = computer_id(&a);
        assert!(!row.sentence.contains(&a_id) && !row.sentence.contains(&b_id));
        assert_eq!(d.members.len(), 2);
        assert!(d.members[0].is_this);
        assert_eq!(d.members[1].name, "a");
        assert_eq!(d.members[1].app_version.as_deref(), Some("0.0.0-e2e"));
        b.act.set(Activity::NotRunning);
        let r = synced(&mut b);
        let d = r.status.detail.expect("typed detail");
        assert_eq!(d.packages[0].title, "Alpha Bars");
    }

    #[test]
    fn a_sync_cycle_is_recorded_in_the_activity_log() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        let rb = synced(&mut b);
        assert_eq!(rb.applied.len(), 1, "{rb:?}");
        let log = b.svc.activity_log(20);
        assert!(
            log.iter()
                .any(|e| e.kind == "applied" && e.package.as_deref() == Some("alpha")),
            "{log:?}"
        );
        assert!(log.iter().all(|e| !e.text.contains(&computer_id(&a))));
    }

    #[test]
    fn pause_is_computer_wide_and_survives_a_restart() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v1");
        a.create();
        assert_eq!(a.svc.paused_since(), None);
        a.svc.set_paused(true).unwrap();
        a.restart();
        assert!(a.svc.paused_since().is_some());
        a.svc.set_paused(false).unwrap();
        a.restart();
        assert_eq!(a.svc.paused_since(), None);
    }

    #[test]
    fn changed_product_needs_help_without_a_doomed_reconfirm_offer() {
        let lab = Lab::new();
        let mut t = lab.pc_with("t", "_test_", "wow_test");
        t.addon("Alpha", "alpha-v1");
        t.create();
        synced(&mut t);
        let other = lab.pc_with("o", "_test_", "wow_other");
        let mut moved = t.cfg.clone();
        moved.install_root = other.install.root.clone();
        let mut svc = SliceService::new(moved).unwrap();
        let st = svc.status();
        assert!(st.reconfirm.is_none(), "{st:?}");
        assert_eq!(st.headline, "Something needs your help");
        assert!(
            st.provider_error
                .as_deref()
                .unwrap_or("")
                .contains("no longer"),
            "{st:?}"
        );
        assert!(matches!(svc.sync_once(), Err(ServiceError::VariantChanged)));
    }

    #[test]
    fn provider_problem_is_plain_english() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v1");
        a.create();
        synced(&mut a);
        std::fs::remove_dir_all(&lab.shared).unwrap();
        std::fs::write(&lab.shared, b"not a folder").unwrap();
        let st = a.svc.status();
        let msg = st.provider_error.clone().unwrap_or_default();
        assert!(!msg.is_empty(), "{st:?}");
        assert!(
            !msg.contains("NotPlainFile") && !msg.contains("NotFound"),
            "{msg}"
        );
    }

    #[test]
    fn install_without_marker_refused() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        std::fs::remove_file(a.install.root.join(".wowsync-synthetic")).unwrap();
        let p = a.svc.inspect();
        assert_eq!(p.kind, PreviewKind::Refused, "{p:?}");
        let req = EnrollRequest {
            join: JoinChoice::New { name: "X".into() },
            identity_confirmed: true,
            scope_confirmed: true,
            digest: p.digest,
            display_name: "a".into(),
        };
        assert!(matches!(a.svc.enroll(&req), Err(ServiceError::Refused(_))));
        assert!(!lab.shared.join("WoWSync").exists());
    }

    struct Crashy {
        _lab: Lab,
        _a: Pc,
        b: Pc,
    }

    /// B holds old versions of two independent add-ons, A holds new ones; both enrolled and
    /// A has published.
    fn two_package_fixture() -> Crashy {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        b.addon("Alpha", "alpha-v0");
        b.addon("Beta", "beta-v0");
        let g = a.create();
        b.join(&g);
        synced(&mut b);
        a.addon("Alpha", "alpha-v1");
        a.addon("Beta", "beta-v1");
        synced(&mut a);
        Crashy {
            _lab: lab,
            _a: a,
            b,
        }
    }

    fn assert_whole(b: &Pc) {
        for (name, old, new) in [
            ("Alpha", "alpha-v0", "alpha-v1"),
            ("Beta", "beta-v0", "beta-v1"),
        ] {
            let v = b.read(&format!("{name}/main.lua")).unwrap_or_default();
            assert!(v == old || v == new, "{name} holds {v:?}");
        }
    }

    #[test]
    fn crash_mid_apply_recovers_on_restart() {
        let mut f = two_package_fixture();
        let b = &mut f.b;
        b.svc.inject_crash_after(Some(3));
        let r = synced(b);
        assert!(r.crashed, "{r:?}");
        assert!(b.journals() > 0, "a crash mid-apply leaves a journal");
        b.restart();
        let r = synced(b);
        assert!(!r.recovered.is_empty(), "{r:?}");
        assert_whole(b);
        for _ in 0..2 {
            synced(b);
        }
        assert_eq!(b.journals(), 0);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert_eq!(b.read("Beta/main.lua").as_deref(), Some("beta-v1"));
    }

    #[test]
    fn two_package_crash_at_every_effect_recovers_with_real_adapters() {
        let mut k = 0;
        loop {
            let mut f = two_package_fixture();
            let b = &mut f.b;
            b.svc.inject_crash_after(Some(k));
            let r = synced(b);
            let done = !r.crashed;
            b.restart();
            for _ in 0..3 {
                synced(b);
                assert_whole(b);
            }
            assert_eq!(b.journals(), 0, "crash point {k}");
            assert_eq!(
                b.read("Alpha/main.lua").as_deref(),
                Some("alpha-v1"),
                "k={k}"
            );
            assert_eq!(b.read("Beta/main.lua").as_deref(), Some("beta-v1"), "k={k}");
            if done {
                break;
            }
            k += 1;
            assert!(k < 200, "the cycle never completed");
        }
        assert!(k >= 6, "expected several crash points, saw {k}");
    }

    #[test]
    fn crash_during_recovery_with_real_adapters_is_reentrant() {
        let mut j = 0;
        loop {
            let mut f = two_package_fixture();
            let b = &mut f.b;
            b.svc.inject_crash_after(Some(4));
            assert!(synced(b).crashed);
            b.restart();
            b.svc.inject_crash_after(Some(j));
            let r = synced(b);
            let done = !r.crashed;
            b.restart();
            for _ in 0..3 {
                synced(b);
                assert_whole(b);
            }
            assert_eq!(b.journals(), 0, "recovery crash point {j}");
            assert_eq!(
                b.read("Alpha/main.lua").as_deref(),
                Some("alpha-v1"),
                "j={j}"
            );
            assert_eq!(b.read("Beta/main.lua").as_deref(), Some("beta-v1"), "j={j}");
            if done {
                break;
            }
            j += 1;
            assert!(j < 200);
        }
        assert!(j >= 1, "recovery itself must have had effects to interrupt");
    }

    /// `applied` results for `pkg` across every state file B has published.
    fn published_applied(lab: &Lab, g: &GroupId, pc: &Pc, pkg: &str) -> Vec<String> {
        use wow_sync_core::records::{decode, CodeState, ReportKind};
        let dir = lab
            .shared
            .join("WoWSync")
            .join(format!("groups/{g}/members/{}/state", computer_id(pc)));
        let mut out = Vec::new();
        for e in std::fs::read_dir(dir).into_iter().flatten() {
            let bytes = std::fs::read(e.unwrap().path()).unwrap();
            let st: CodeState = decode(&bytes, 16 * 1024 * 1024).unwrap();
            for r in st.apply_report.iter().flat_map(|a| &a.results) {
                if r.package.as_str() == pkg && r.result == ReportKind::Applied {
                    out.push(format!("{}@{}", st.seq, r.package_hash.unwrap()));
                }
            }
        }
        out
    }

    /// Follow-up #9: a crash after the apply finished but before the report was published
    /// must not lose the report; it is published exactly once after restart.
    #[test]
    fn apply_report_survives_crash_before_publish() {
        let mut window_hit = false;
        let mut k = 0;
        loop {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            let mut b = lab.pc("b");
            a.addon("Alpha", "alpha-v1");
            let g = a.create();
            synced(&mut a);
            b.join(&g);
            b.svc.inject_crash_after(Some(k));
            let r = synced(&mut b);
            let done = !r.crashed;
            let applied = r.applied.iter().any(|l| l.text == "applied");
            let pending_file = b.cfg.state_dir.join("pending-report.json");
            let mut uncleared = None;
            if r.crashed && applied {
                assert!(
                    published_applied(&lab, &g, &b, "alpha").is_empty(),
                    "k={k}: crashed before the report was published"
                );
                uncleared = Some(std::fs::read(&pending_file).expect("report kept locally"));
                window_hit = true;
            }
            b.restart();
            let mut last = None;
            for _ in 0..3 {
                last = Some(synced(&mut b));
            }
            assert_eq!(b.journals(), 0, "k={k}");
            assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
            let reported = published_applied(&lab, &g, &b, "alpha");
            assert_eq!(
                reported.len(),
                1,
                "k={k}: exactly one report, got {reported:?}"
            );
            let target = alpha_package_hash(&lab, &g, &a);
            assert!(
                reported[0].ends_with(&target),
                "k={k}: {reported:?} vs {target}"
            );
            let last = last.unwrap();
            assert!(last.published.is_none(), "k={k}: republished {last:?}");
            if let Some(bytes) = uncleared {
                // A crash after the state was written but before the store was cleared:
                // the next start must not publish the same report again.
                std::fs::write(&pending_file, bytes).unwrap();
                b.restart();
                for _ in 0..2 {
                    let r = synced(&mut b);
                    assert!(r.published.is_none(), "k={k}: republished {r:?}");
                }
                assert_eq!(published_applied(&lab, &g, &b, "alpha").len(), 1, "k={k}");
            }
            let t = texts(&synced(&mut a));
            assert!(t.contains("applied and reported by"), "k={k}: {t}");
            if done {
                break;
            }
            k += 1;
            assert!(k < 200, "the cycle never completed");
        }
        assert!(
            window_hit,
            "no crash point between apply and publish was exercised"
        );
    }

    /// The package hash of A's only add-on (a single-folder package).
    fn alpha_package_hash(lab: &Lab, g: &GroupId, pc: &Pc) -> String {
        use wow_sync_core::records::{decode, CodeState};
        let dir = lab
            .shared
            .join("WoWSync")
            .join(format!("groups/{g}/members/{}/state", computer_id(pc)));
        let e = std::fs::read_dir(dir).unwrap().next().unwrap().unwrap();
        let st: CodeState = decode(&std::fs::read(e.path()).unwrap(), 16 * 1024 * 1024).unwrap();
        let f = st
            .folders
            .iter()
            .find(|f| f.key.as_str() == "alpha")
            .unwrap();
        wow_sync_core::hash::package_hash(&[(f.key.clone(), f.tree.unwrap())]).to_string()
    }

    /// Test-only fault: `path` stays readable but cannot be replaced until dropped.
    /// Windows: a handle that shares read access only (`FILE_SHARE_READ`).
    /// macOS: the owner-settable immutable flag (`chflags uchg`). This needs no root, and
    /// rename onto an immutable target fails with EPERM. The caller asserts the block works,
    /// so an ineffective block fails the test instead of passing vacuously.
    #[cfg(any(windows, target_os = "macos"))]
    struct ReplaceBlock {
        #[cfg(windows)]
        _hold: std::fs::File,
        #[cfg(target_os = "macos")]
        path: PathBuf,
    }

    #[cfg(any(windows, target_os = "macos"))]
    impl ReplaceBlock {
        #[cfg(windows)]
        fn new(path: &Path) -> Self {
            use std::os::windows::fs::OpenOptionsExt;
            let hold = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(1)
                .open(path)
                .unwrap();
            Self { _hold: hold }
        }

        #[cfg(target_os = "macos")]
        fn new(path: &Path) -> Self {
            Self::chflags("uchg", path);
            Self {
                path: path.to_path_buf(),
            }
        }

        #[cfg(target_os = "macos")]
        fn chflags(flag: &str, path: &Path) {
            let st = std::process::Command::new("/usr/bin/chflags")
                .arg(flag)
                .arg(path)
                .status()
                .unwrap();
            assert!(st.success(), "chflags {flag} failed: {st}");
        }
    }

    #[cfg(target_os = "macos")]
    impl Drop for ReplaceBlock {
        fn drop(&mut self) {
            Self::chflags("nouchg", &self.path);
        }
    }

    /// R3-M1 (R2-recheck3): a pending-report clear that keeps failing (the file stays readable
    /// but cannot be replaced) must not let an inventory-only state displace the state
    /// that carries the report, so the same `applied` result is never published twice.
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn pending_clear_failure_does_not_duplicate_report() {
        use wow_sync_core::records::{decode, encode, CodeState, PendingReport};
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        assert_eq!(published_applied(&lab, &g, &b, "alpha").len(), 1);
        let states = lab
            .shared
            .join("WoWSync")
            .join("groups")
            .join(g.as_str())
            .join("members")
            .join(computer_id(&b))
            .join("state");
        let st: CodeState = std::fs::read_dir(states)
            .unwrap()
            .map(|e| {
                decode::<CodeState>(&std::fs::read(e.unwrap().path()).unwrap(), 16 * 1024 * 1024)
                    .unwrap()
            })
            .find(|st| st.apply_report.is_some())
            .unwrap();
        let pending = PendingReport {
            group_id: g.clone(),
            enrollment_id: st.enrollment_id,
            report: st.apply_report,
        };
        let bytes = encode(&pending);
        let path = b.cfg.state_dir.join("pending-report.json");
        std::fs::write(&path, &bytes).unwrap();
        b.restart();
        // The store stays readable but cannot be replaced.
        let hold = ReplaceBlock::new(&path);
        assert!(LocalState::open_at(&b.cfg.state_dir)
            .unwrap()
            .write_atomic("pending-report.json", &bytes)
            .is_err());
        b.addon("Beta", "beta-v1");
        let first = synced(&mut b);
        assert_eq!(
            published_applied(&lab, &g, &b, "alpha").len(),
            1,
            "{first:?}"
        );
        let second = synced(&mut b);
        assert_eq!(
            published_applied(&lab, &g, &b, "alpha").len(),
            1,
            "duplicate Alpha applied report after inventory-only publication: {second:?}"
        );
        // Once the store can be reconciled, publication resumes without a duplicate.
        drop(hold);
        let third = synced(&mut b);
        let fourth = synced(&mut b);
        assert_eq!(
            published_applied(&lab, &g, &b, "alpha").len(),
            1,
            "{third:?} {fourth:?}"
        );
        let beta_seen = std::fs::read_dir(
            lab.shared
                .join("WoWSync")
                .join("groups")
                .join(g.as_str())
                .join("members")
                .join(computer_id(&b))
                .join("state"),
        )
        .unwrap()
        .filter_map(|e| {
            decode::<CodeState>(&std::fs::read(e.unwrap().path()).unwrap(), 16 * 1024 * 1024).ok()
        })
        .any(|st| st.folders.iter().any(|f| f.key.as_str() == "beta"));
        assert!(beta_seen, "Beta was never published: {third:?} {fourth:?}");
    }

    /// mi-1: scope consent is required for a recognized client too, at the service boundary.
    #[test]
    fn recognized_enroll_without_scope_consent_is_refused() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut req = a.confirm_request(JoinChoice::New {
            name: "Slice".into(),
        });
        req.identity_confirmed = false;
        req.scope_confirmed = false;
        assert!(
            matches!(a.svc.enroll(&req), Err(ServiceError::Refused(_))),
            "enrolled without scope consent"
        );
        assert!(!lab.shared.join("WoWSync").exists());
        assert!(!a.svc.status().enrolled);
        req.scope_confirmed = true;
        a.svc.enroll(&req).unwrap();
    }

    #[test]
    fn destination_updated_during_staging_is_not_replaced() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        b.addon("Alpha", "alpha-v0");
        let g = a.create();
        b.join(&g);
        synced(&mut b);
        a.addon("Alpha", "alpha-v1");
        synced(&mut a);
        let live = b.install.addons.join("Alpha").join("main.lua");
        b.svc.hooks_mut().after_stage = Some(Box::new(move || {
            std::fs::write(&live, b"user-edit").unwrap();
        }));
        let r = synced(&mut b);
        assert_eq!(
            b.read("Alpha/main.lua").as_deref(),
            Some("user-edit"),
            "{r:?}"
        );
        assert!(
            r.applied.iter().all(|l| !l.text.starts_with("applied")),
            "{r:?}"
        );
        assert_eq!(b.journals(), 0);
    }

    #[test]
    fn publish_aborts_when_source_changes() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        let live = a.install.addons.join("Alpha").join("main.lua");
        a.svc.hooks_mut().after_put = Some(Box::new(move || {
            std::fs::write(&live, b"changed while publishing").unwrap();
        }));
        let r = synced(&mut a);
        assert!(r.published.is_none(), "{r:?}");
        assert!(
            r.publish_note
                .as_deref()
                .unwrap_or("")
                .contains("not published"),
            "{r:?}"
        );
        assert_eq!(provider_state_files(&lab, &g, &a), 0);
    }

    #[test]
    fn peer_confirmation_only_after_apply() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        let b_id = computer_id(&b);
        b.act.set(Activity::Running { pids: vec![7] });
        synced(&mut b);
        let r = synced(&mut a);
        let t = texts(&r);
        assert!(!t.contains("reported by"), "published is not applied: {t}");
        assert!(
            r.status
                .last_publish
                .as_deref()
                .unwrap_or("")
                .contains("Published"),
            "{t}"
        );
        b.act.set(Activity::NotRunning);
        let rb = synced(&mut b);
        assert_eq!(rb.applied.len(), 1, "{rb:?}");
        let r = synced(&mut a);
        let t = texts(&r);
        assert!(t.contains("applied and reported by"), "{t}");
        assert!(
            t.contains("b") && !t.contains(&b_id),
            "peer named by its display name: {t}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn fake_client_blocks_real_apply() {
        use wow_sync_adapters::testkit::FakeClient;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        // Real process observation instead of the scripted source.
        b.svc = SliceService::new(b.cfg.clone()).unwrap();
        let client = FakeClient::start(&b.install.variant_dir);
        let r = synced(&mut b);
        assert!(b.read("Alpha/main.lua").is_none(), "{r:?}");
        assert!(
            r.apply_note.as_deref().unwrap_or("").contains("running"),
            "{r:?}"
        );
        drop(client);
        let r = synced(&mut b);
        assert_eq!(
            b.read("Alpha/main.lua").as_deref(),
            Some("alpha-v1"),
            "{r:?}"
        );
    }

    // ------------------------------------------------ authority plane (R4-integrate B1)

    fn pc_ids(pc: &Pc) -> (ComputerId, EnrollmentId) {
        let me = computer_id(pc);
        let r = pc
            .svc
            .plane()
            .members
            .pairs()
            .find(|p| p.computer_id.as_str() == me)
            .expect("own pair is in the membership")
            .clone();
        (r.computer_id, r.enrollment_id)
    }

    fn provider_file(lab: &Lab, rel: &str) -> PathBuf {
        rel.split('/')
            .fold(lab.shared.join("WoWSync"), |a, s| a.join(s))
    }

    fn put_authority(lab: &Lab, g: &GroupId, into: &ComputerId, r: &AuthorityRecord) -> Sha256 {
        let bytes = encode_authority(r);
        let h = sha256(&bytes);
        let p = provider_file(lab, &layout::authority_record(g, into, &h));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
        h
    }

    fn authority_files(lab: &Lab, g: &GroupId, c: &ComputerId) -> Vec<String> {
        let dir = provider_file(lab, &layout::authority_dir(g, c));
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .map(|d| {
                d.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    fn removal_hash(lab: &Lab, g: &GroupId, c: &ComputerId) -> Sha256 {
        authority_files(lab, g, c)
            .iter()
            .map(|f| Sha256::parse(f.trim_end_matches(".json")).unwrap())
            .find(|h| {
                let p = provider_file(lab, &layout::authority_record(g, c, h));
                let bytes = std::fs::read(p).unwrap();
                matches!(decode_authority(&bytes, h), Ok(AuthorityRecord::Removal(_)))
            })
            .expect("a removal record")
    }

    fn status(pc: &Pc, g: &GroupId, h: &Sha256) -> Option<Status> {
        let p = pc.svc.plane();
        derive(&p.ledger, g, &p.members).status_of(h).cloned()
    }

    /// Makes `a` see Alpha disappear for longer than the confirmation gap, then runs its cycle:
    /// a matured removal record is written.
    fn a_removes_alpha(a: &mut Pc, g: &GroupId) {
        a.addon("Alpha", "alpha-v1");
        synced(a);
        std::fs::remove_dir_all(a.install.addons.join("Alpha")).unwrap();
        let st = LocalState::open_at(&a.cfg.state_dir).unwrap();
        let store = RemovalStore::new(&st, g.clone());
        let (base, mut obs) = store.load();
        let key = base.packages.keys().next().expect("a baseline").clone();
        obs.gone.insert(key, (1, 1));
        store.save(&base, &obs).unwrap();
        let r = synced(a);
        assert!(!r.crashed, "{r:?}");
    }

    #[test]
    fn matured_removal_is_written_learned_and_survives_a_restart() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        let g = a.create();
        a_removes_alpha(&mut a, &g);
        let (ac, _) = pc_ids(&a);
        let files = authority_files(&lab, &g, &ac);
        assert_eq!(
            files.len(),
            2,
            "a removal and its own acknowledgement: {files:?}"
        );
        let h = removal_hash(&lab, &g, &ac);
        assert_eq!(status(&a, &g, &h), Some(Status::Valid));
        a.restart();
        assert_eq!(
            status(&a, &g, &h),
            Some(Status::Valid),
            "the learned record is loaded from local state before any scan"
        );
        b.join(&g);
        synced(&mut b);
        assert_eq!(status(&b, &g, &h), Some(Status::Valid));
    }

    #[test]
    fn a_removal_whose_record_could_not_be_written_is_replayed_on_the_next_cycle() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let g = a.create();
        a.addon("Alpha", "alpha-v1");
        synced(&mut a);
        let (ac, _) = pc_ids(&a);
        // The write of the record fails (a file where its folder belongs) after the
        // observation was consumed: the window of a crash between the two.
        let dir = provider_file(&lab, &layout::authority_dir(&g, &ac));
        let _ = std::fs::remove_dir(&dir);
        std::fs::write(&dir, b"in the way").unwrap();
        std::fs::remove_dir_all(a.install.addons.join("Alpha")).unwrap();
        let st = LocalState::open_at(&a.cfg.state_dir).unwrap();
        let store = RemovalStore::new(&st, g.clone());
        let (base, mut obs) = store.load();
        let key = base.packages.keys().next().expect("a baseline").clone();
        obs.gone.insert(key, (1, 1));
        store.save(&base, &obs).unwrap();
        let r = synced(&mut a);
        assert!(
            r.apply_note
                .as_deref()
                .is_some_and(|n| n.contains("removal record not written")),
            "{r:?}"
        );
        assert!(dir.is_file(), "nothing was written");
        a.restart();
        std::fs::remove_file(&dir).unwrap();
        synced(&mut a);
        let files = authority_files(&lab, &g, &ac);
        assert_eq!(files.len(), 2, "the removal and its ack: {files:?}");
        let h = removal_hash(&lab, &g, &ac);
        assert_eq!(status(&a, &g, &h), Some(Status::Valid));
        synced(&mut a);
        assert_eq!(
            authority_files(&lab, &g, &ac).len(),
            2,
            "replayed once, not again"
        );
    }

    #[test]
    fn a_pending_removal_is_dropped_when_the_folder_is_installed_again() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let g = a.create();
        a.addon("Alpha", "alpha-v1");
        synced(&mut a);
        let (ac, _) = pc_ids(&a);
        let dir = provider_file(&lab, &layout::authority_dir(&g, &ac));
        let _ = std::fs::remove_dir(&dir);
        std::fs::write(&dir, b"in the way").unwrap();
        std::fs::remove_dir_all(a.install.addons.join("Alpha")).unwrap();
        let st = LocalState::open_at(&a.cfg.state_dir).unwrap();
        let store = RemovalStore::new(&st, g.clone());
        let (base, mut obs) = store.load();
        let key = base.packages.keys().next().expect("a baseline").clone();
        obs.gone.insert(key, (1, 1));
        store.save(&base, &obs).unwrap();
        synced(&mut a);
        std::fs::remove_file(&dir).unwrap();
        a.addon("Alpha", "alpha-v1");
        synced(&mut a);
        assert!(
            authority_files(&lab, &g, &ac).is_empty(),
            "an addon that is back is not removed from the group"
        );
    }

    #[test]
    fn records_from_non_members_and_forged_copies_are_not_admitted() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        let g = a.create();
        a_removes_alpha(&mut a, &g);
        let (ac, _) = pc_ids(&a);
        let h = removal_hash(&lab, &g, &ac);
        let bytes =
            std::fs::read(provider_file(&lab, &layout::authority_record(&g, &ac, &h))).unwrap();
        let AuthorityRecord::Removal(mut forged) = decode_authority(&bytes, &h).unwrap() else {
            panic!("a removal");
        };
        forged.issuer.enrollment_id = EnrollmentId::parse(&"7".repeat(32)).unwrap();
        let fh = put_authority(&lab, &g, &ac, &AuthorityRecord::Removal(forged));
        // A copy whose bytes do not hash to its name is not a record at all.
        let bad_name = Sha256::parse(&"c".repeat(64)).unwrap();
        let bad = provider_file(&lab, &layout::authority_record(&g, &ac, &bad_name));
        std::fs::write(bad, b"{\"schema\":\"wowsync.removal/1\"}").unwrap();
        b.join(&g);
        synced(&mut b);
        assert_eq!(
            status(&b, &g, &fh),
            Some(Status::Invalid(InvalidReason::NotAMember))
        );
        assert_eq!(status(&b, &g, &h), Some(Status::Valid));
        assert_eq!(
            status(&b, &g, &bad_name),
            None,
            "a bad copy teaches nothing"
        );
    }

    fn latest_state(lab: &Lab, g: &GroupId, pc: &Pc) -> CodeState {
        let c = computer_id(pc);
        let dir = lab
            .shared
            .join("WoWSync")
            .join(format!("groups/{g}/members/{c}/state"));
        let newest = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                layout::parse_state_name(&n).map(|(_, seq)| (seq, e.path()))
            })
            .max()
            .expect("a published state");
        decode::<CodeState>(&std::fs::read(newest.1).unwrap(), 8 * 1024 * 1024).unwrap()
    }

    fn index_refs(lab: &Lab, g: &GroupId, pc: &Pc, st: &CodeState) -> Vec<Sha256> {
        let (c, e) = pc_ids(pc);
        let head = st.authority_index.expect("an authority index");
        let fetch = |h: &Sha256| {
            std::fs::read(provider_file(lab, &layout::authority_index_chunk(g, &c, h))).ok()
        };
        let read = read_index_chain(
            &IndexHead {
                head: head.head,
                total: head.total,
            },
            g,
            &e,
            &Default::default(),
            &fetch,
        );
        assert_eq!(read.error, None);
        read.refs.iter().map(|r| r.hash).collect()
    }

    #[test]
    fn own_records_are_listed_in_the_published_authority_index() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let g = a.create();
        synced(&mut a);
        assert!(latest_state(&lab, &g, &a).authority_index.is_none());
        a_removes_alpha(&mut a, &g);
        let (ac, _) = pc_ids(&a);
        let st = latest_state(&lab, &g, &a);
        let mut listed = index_refs(&lab, &g, &a, &st);
        listed.sort();
        let mut on_disk: Vec<Sha256> = authority_files(&lab, &g, &ac)
            .iter()
            .map(|f| Sha256::parse(f.trim_end_matches(".json")).unwrap())
            .collect();
        on_disk.sort();
        assert_eq!(listed, on_disk);
        assert_eq!(st.authority_index.unwrap().total, 2);
        // Listed once: a further cycle neither republishes nor grows the chain.
        let r = synced(&mut a);
        assert!(r.published.is_none(), "{r:?}");
        a.restart();
        let r = synced(&mut a);
        assert!(r.published.is_none(), "{r:?}");
        assert_eq!(latest_state(&lab, &g, &a).authority_index.unwrap().total, 2);
    }

    #[test]
    fn a_peer_mirrors_valid_records_and_only_repairs_its_own_copy() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        let g = a.create();
        a_removes_alpha(&mut a, &g);
        let (ac, _) = pc_ids(&a);
        b.join(&g);
        let bc = ComputerId::parse(&computer_id(&b)).unwrap();
        let removal = removal_hash(&lab, &g, &ac);
        let ack = authority_files(&lab, &g, &ac)
            .iter()
            .map(|f| Sha256::parse(f.trim_end_matches(".json")).unwrap())
            .find(|h| *h != removal)
            .unwrap();
        let good = std::fs::read(provider_file(
            &lab,
            &layout::authority_record(&g, &ac, &removal),
        ))
        .unwrap();
        // B's own copy of the removal is damaged before it first lists it; A's copy of the ack
        // is damaged too.
        let b_copy = provider_file(&lab, &layout::authority_record(&g, &bc, &removal));
        std::fs::create_dir_all(b_copy.parent().unwrap()).unwrap();
        std::fs::write(&b_copy, b"damaged").unwrap();
        let a_ack = provider_file(&lab, &layout::authority_record(&g, &ac, &ack));
        std::fs::write(&a_ack, b"damaged too").unwrap();
        synced(&mut b);
        assert_eq!(std::fs::read(&b_copy).unwrap(), good, "own copy repaired");
        assert_eq!(
            std::fs::read(&a_ack).unwrap(),
            b"damaged too",
            "a peer's directory is never written"
        );
        assert!(
            !authority_files(&lab, &g, &bc).contains(&format!("{ack}.json")),
            "a record with no good copy is read past, not mirrored"
        );
        assert_eq!(status(&b, &g, &removal), Some(Status::Valid));
        let st = latest_state(&lab, &g, &b);
        assert_eq!(index_refs(&lab, &g, &b, &st), vec![removal]);
        // The issuer's directory becoming unusable does not lose the record (Q-51).
        std::fs::remove_dir_all(provider_file(&lab, &layout::authority_dir(&g, &ac))).unwrap();
        let mut c = lab.pc("c");
        c.join(&g);
        synced(&mut c);
        assert_eq!(status(&c, &g, &removal), Some(Status::Valid));
    }

    #[test]
    fn a_retired_computer_publishes_and_writes_nothing() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        let (ac, ae) = pc_ids(&a);
        let (bc, be) = pc_ids(&b);
        let before = provider_state_files(&lab, &g, &b);
        // A retires B's enrollment at cutoff 0: everything B issues from now on is above it.
        let ret = AuthorityRecord::Retirement(Retirement {
            group_id: g.clone(),
            issuer: Issuer {
                computer_id: ac.clone(),
                enrollment_id: ae.clone(),
                issued_at_seq: 1000,
            },
            retired: EnrollmentRef {
                computer_id: bc.clone(),
                enrollment_id: be.clone(),
            },
            cutoff_seq: 0,
            was_settings_master: false,
            retirements_known: vec![],
            written_unix_ms: 1,
        });
        let rh = put_authority(&lab, &g, &ac, &ret);
        b.addon("Beta", "beta-v1");
        let r = synced(&mut b);
        assert_eq!(status(&b, &g, &rh), Some(Status::Valid));
        assert!(r.published.is_none(), "{r:?}");
        assert!(
            r.publish_note.as_deref().unwrap_or("").contains("retired"),
            "{r:?}"
        );
        assert_eq!(provider_state_files(&lab, &g, &b), before);
        assert!(authority_files(&lab, &g, &bc).is_empty());
        // The group still works for the computer that was not retired.
        a.addon("Gamma", "gamma-v1");
        let r = synced(&mut a);
        assert!(r.published.is_some(), "{r:?}");
    }

    // ------------------------------------------------ composed target (R4-integrate B3b)

    fn alpha_folder(lab: &Lab, g: &GroupId, pc: &Pc) -> wow_sync_core::inventory::FolderState {
        latest_state(lab, g, pc)
            .folders
            .into_iter()
            .find(|f| f.key.as_str() == "alpha")
            .expect("alpha is published")
    }

    fn issuer_of(pc: &Pc) -> Issuer {
        let (c, e) = pc_ids(pc);
        Issuer {
            computer_id: c,
            enrollment_id: e,
            issued_at_seq: 1000,
        }
    }

    /// Three computers that all hold Alpha v1, then `b` moves to v2 and publishes it.
    fn alpha_v2_published(lab: &Lab) -> (Pc, Pc, Pc, GroupId) {
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        let mut c = lab.pc("c");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        c.join(&g);
        synced(&mut c);
        synced(&mut a);
        b.addon("Alpha", "alpha-v2");
        let r = synced(&mut b);
        assert!(r.published.is_some(), "{r:?}");
        (a, b, c, g)
    }

    #[test]
    fn a_removal_published_by_a_peer_is_applied_and_an_offline_peer_cannot_resurrect_it() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        let mut c = lab.pc("c");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        c.join(&g);
        synced(&mut c);
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        a_removes_alpha(&mut a, &g);
        // C is offline from here on: its published state still lists Alpha.
        assert!(alpha_folder(&lab, &g, &c).tree.is_some());
        let r = synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua"), None, "{r:?}");
        assert!(
            !b.install.addons.join("Alpha").exists(),
            "the folder is gone, not emptied"
        );
        for _ in 0..2 {
            synced(&mut b);
            assert_eq!(
                b.read("Alpha/main.lua"),
                None,
                "removed once, stays removed"
            );
        }
        // A computer enrolled later sees only C's stale copy as a holder: it must not install it.
        let mut d = lab.pc("d");
        d.join(&g);
        for _ in 0..2 {
            let r = synced(&mut d);
            assert_eq!(d.read("Alpha/main.lua"), None, "resurrected: {r:?}");
        }
        assert!(!d.install.addons.join("Alpha").exists());
    }

    #[test]
    fn a_computer_that_lost_a_folder_waits_for_the_group_instead_of_reinstalling() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        std::fs::remove_dir_all(b.install.addons.join("Alpha")).unwrap();
        for _ in 0..2 {
            let r = synced(&mut b);
            assert_eq!(b.read("Alpha/main.lua"), None, "{r:?}");
            assert!(
                texts(&r).contains("removal history"),
                "the wait is visible: {}",
                texts(&r)
            );
        }
        let (bc, _) = pc_ids(&b);
        assert!(
            authority_files(&lab, &g, &bc).is_empty(),
            "nothing is written before the removal is confirmed"
        );
    }

    #[test]
    fn a_live_keep_pins_a_folder_against_a_newer_candidate() {
        use wow_sync_core::choice::{build_keep, ChoiceView, CopyEntry, WriteCx};
        use wow_sync_core::kinds::{ChoiceReason, ManifestFolder};
        use wow_sync_core::ledger::Ledger;
        use wow_sync_core::target::Source;
        let lab = Lab::new();
        let (mut a, _b, mut c, g) = alpha_v2_published(&lab);
        // Control: without a record a computer takes the newer release.
        synced(&mut c);
        assert_eq!(c.read("Alpha/main.lua").as_deref(), Some("alpha-v2"));
        let (ac, ae) = pc_ids(&a);
        let kept = alpha_folder(&lab, &g, &a);
        let copy = CopyEntry::new(
            vec![ManifestFolder {
                key: kept.key.clone(),
                name: kept.name.clone(),
                tree: kept.tree.unwrap(),
            }],
            vec![ae],
        );
        let ledger = Ledger::new();
        let view = ChoiceView::new(&ledger, &Default::default());
        let keep = build_keep(
            &view,
            &WriteCx {
                group: &g,
                issuer: issuer_of(&a),
                now_unix_ms: 5,
            },
            ChoiceReason::Ambiguous,
            std::slice::from_ref(&kept.key),
            &copy,
            &[],
            &Source::from_state(&latest_state(&lab, &g, &a), true),
        )
        .unwrap();
        let kh = put_authority(&lab, &g, &ac, &AuthorityRecord::CodeChoice(keep));
        for _ in 0..2 {
            let r = synced(&mut a);
            assert_eq!(status(&a, &g, &kh), Some(Status::Valid), "{r:?}");
            assert_eq!(
                a.read("Alpha/main.lua").as_deref(),
                Some("alpha-v1"),
                "the keep pins the folder: {r:?}"
            );
        }
    }

    #[test]
    fn a_rejected_release_is_never_installed() {
        use wow_sync_core::choice::ChoiceView;
        use wow_sync_core::hash::package_hash;
        use wow_sync_core::kinds::{
            RecoveredEntry, RejectedEntry, Rejection, RejectionSettings, TreeEntry,
        };
        use wow_sync_core::ledger::Ledger;
        let lab = Lab::new();
        let (mut a, b, _c, g) = alpha_v2_published(&lab);
        let (ac, _) = pc_ids(&a);
        let bad = alpha_folder(&lab, &g, &b);
        let good = alpha_folder(&lab, &g, &a);
        let entry = |f: &wow_sync_core::inventory::FolderState| TreeEntry {
            key: f.key.clone(),
            tree: f.tree.unwrap(),
        };
        let hash = |f: &wow_sync_core::inventory::FolderState| {
            package_hash(&[(f.key.clone(), f.tree.unwrap())])
        };
        let ledger = Ledger::new();
        let view = ChoiceView::new(&ledger, &Default::default());
        let rej = Rejection {
            group_id: g.clone(),
            issuer: issuer_of(&a),
            folders: vec![bad.key.clone()],
            rejected: RejectedEntry {
                package_hash: hash(&bad),
                version_label: "3.0".into(),
                folders: vec![entry(&bad)],
            },
            recovered: RecoveredEntry {
                package_hash: hash(&good),
                folders: vec![entry(&good)],
            },
            considered: {
                let (_, be) = pc_ids(&b);
                let (_, ae) = pc_ids(&a);
                let mut v = vec![
                    wow_sync_core::kinds::Considered {
                        package_hash: hash(&bad),
                        folders: vec![entry(&bad)],
                        holders: vec![be],
                    },
                    wow_sync_core::kinds::Considered {
                        package_hash: hash(&good),
                        folders: vec![entry(&good)],
                        holders: vec![ae],
                    },
                ];
                v.sort();
                v
            },
            settings: RejectionSettings::NotApplicable,
            known: view.known_for(std::slice::from_ref(&bad.key)),
            considered_parts: vec![],
            written_unix_ms: 5,
        };
        let rh = put_authority(&lab, &g, &ac, &AuthorityRecord::Rejection(rej));
        for _ in 0..3 {
            let r = synced(&mut a);
            assert_eq!(status(&a, &g, &rh), Some(Status::Valid), "{r:?}");
            assert_eq!(
                a.read("Alpha/main.lua").as_deref(),
                Some("alpha-v1"),
                "the rejected release is not installed: {r:?}"
            );
        }
    }

    #[test]
    fn a_rollback_stages_the_recovered_release_from_a_local_recovery_point() {
        use wow_sync_core::hash::package_hash;
        use wow_sync_core::records::decode_tree;
        let lab = Lab::new();
        let (mut a, b, c, g) = alpha_v2_published(&lab);
        let v1 = alpha_folder(&lab, &g, &c);
        let tree = v1.tree.unwrap();
        let v1_hash = package_hash(&[(v1.key.clone(), tree)]);
        // A follows B to v2; its recovery point now holds v1.
        synced(&mut a);
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v2"));
        // v1 leaves the shared folder: its files and its tree record.
        let rec_rel = layout::tree_record(&g, &tree);
        let rec = std::fs::read(provider_file(&lab, &rec_rel)).unwrap();
        for f in decode_tree(&rec, &tree).unwrap().files {
            std::fs::remove_file(provider_file(&lab, &layout::blob(&g, &f.sha256))).unwrap();
        }
        std::fs::remove_file(provider_file(&lab, &rec_rel)).unwrap();

        // B changed v1 to v2 by hand: no recovery point holds v1 anywhere, so it is refused.
        let mut b = b;
        synced(&mut b);
        let (bc, _) = pc_ids(&b);
        let before = authority_files(&lab, &g, &bc);
        assert_eq!(
            b.svc.command(Command::Rollback {
                package: v1.key.clone(),
                recovered: v1_hash,
            }),
            Err(CommandError::Rollback(
                crate::rollback_flow::RollbackRefusal::NotAvailable
            ))
        );
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v2"));
        assert_eq!(authority_files(&lab, &g, &bc), before, "nothing written");

        assert_eq!(
            a.svc.command(Command::Rollback {
                package: v1.key.clone(),
                recovered: v1_hash,
            }),
            Ok(CommandOutcome::Recorded)
        );
        let r = synced(&mut a);
        assert_eq!(
            a.read("Alpha/main.lua").as_deref(),
            Some("alpha-v1"),
            "{r:?}"
        );
        let (ac, _) = pc_ids(&a);
        assert!(!authority_files(&lab, &g, &ac).is_empty(), "{r:?}");
    }

    // ------------------------------------------------ control commands (R4-integrate C1)

    #[test]
    fn a_rollback_installs_the_recovered_release_and_an_offline_peer_cannot_bring_the_rejected_one_back(
    ) {
        use wow_sync_core::hash::package_hash;
        let lab = Lab::new();
        let (mut a, mut b, mut c, g) = alpha_v2_published(&lab);
        // C follows B to v2 and goes offline; A stays on v1, so v1 remains published by A.
        synced(&mut c);
        assert_eq!(c.read("Alpha/main.lua").as_deref(), Some("alpha-v2"));
        let v1 = alpha_folder(&lab, &g, &a);
        let v1_hash = package_hash(&[(v1.key.clone(), v1.tree.unwrap())]);
        let v2 = alpha_folder(&lab, &g, &b);
        let v2_hash = package_hash(&[(v2.key.clone(), v2.tree.unwrap())]);
        let alpha = v1.key.clone();
        synced(&mut b);

        assert_eq!(
            b.svc.command(Command::Rollback {
                package: alpha.clone(),
                recovered: wow_sync_core::ids::Sha256([9; 32]),
            }),
            Err(CommandError::UnknownCopy(wow_sync_core::ids::Sha256(
                [9; 32]
            ))),
            "an unknown release is refused"
        );
        assert_eq!(
            b.svc.command(Command::Rollback {
                package: alpha.clone(),
                recovered: v2_hash,
            }),
            Err(CommandError::Rollback(
                crate::rollback_flow::RollbackRefusal::SameRelease
            )),
            "rolling back to what is installed is refused"
        );
        let (bc, _) = pc_ids(&b);
        assert!(
            authority_files(&lab, &g, &bc).is_empty(),
            "nothing queued, nothing written"
        );
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v2"));

        assert_eq!(
            b.svc.command(Command::Rollback {
                package: alpha,
                recovered: v1_hash,
            }),
            Ok(CommandOutcome::Recorded)
        );
        let r = synced(&mut b);
        assert_eq!(
            b.read("Alpha/main.lua").as_deref(),
            Some("alpha-v1"),
            "{r:?}"
        );
        assert!(
            !authority_files(&lab, &g, &bc).is_empty(),
            "the rejection is written with the install"
        );
        synced(&mut b);

        // C comes back still holding v2: it is moved to v1 and never publishes v2 again.
        for _ in 0..2 {
            let r = synced(&mut c);
            assert_eq!(
                c.read("Alpha/main.lua").as_deref(),
                Some("alpha-v1"),
                "{r:?}"
            );
        }
        for pc in [&mut a, &mut b, &mut c] {
            synced(pc);
            assert_eq!(pc.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        }
    }

    fn ref_of(pc: &Pc) -> EnrollmentRef {
        let (c, e) = pc_ids(pc);
        EnrollmentRef {
            computer_id: c,
            enrollment_id: e,
        }
    }

    #[test]
    fn retire_command_writes_a_retirement_and_a_retired_computer_is_refused() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        let (ac, _) = pc_ids(&a);
        let out = a
            .svc
            .command(Command::Retire {
                subject: ref_of(&b),
            })
            .unwrap();
        let CommandOutcome::Written(h) = out else {
            panic!("{out:?}")
        };
        synced(&mut a);
        assert_eq!(status(&a, &g, &h), Some(Status::Valid));
        assert_eq!(authority_files(&lab, &g, &ac).len(), 1);
        let r = synced(&mut b);
        assert!(r.published.is_none(), "{r:?}");
        assert_eq!(
            b.svc.command(Command::Retire {
                subject: ref_of(&a)
            }),
            Err(CommandError::NotAllowed)
        );
        let (bc, _) = pc_ids(&b);
        assert!(authority_files(&lab, &g, &bc).is_empty());
    }

    #[test]
    fn a_retired_computer_cannot_publish_until_it_re_enrolls() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        let (_, old_enrollment) = pc_ids(&b);
        assert_eq!(
            a.svc.command(Command::ReEnroll),
            Ok(CommandOutcome::NothingToDo),
            "an active computer has nothing to join again"
        );
        a.svc
            .command(Command::Retire {
                subject: ref_of(&b),
            })
            .unwrap();
        synced(&mut a);
        for _ in 0..2 {
            let r = synced(&mut b);
            assert!(r.published.is_none(), "retired: {r:?}");
        }
        b.addon("Beta", "beta-v1");
        assert!(synced(&mut b).published.is_none());
        assert_eq!(
            a.read("Beta/main.lua"),
            None,
            "nothing a retired computer holds reaches the group"
        );
        assert_eq!(
            b.svc.command(Command::ReEnroll),
            Ok(CommandOutcome::ReEnrolled)
        );
        let r = synced(&mut b);
        assert!(r.published.is_some(), "re-enrolled: {r:?}");
        synced(&mut a);
        synced(&mut a);
        let bc = pc_ids(&b).0;
        let enrollments: Vec<_> = a
            .svc
            .plane()
            .members
            .pairs()
            .filter(|p| p.computer_id == bc)
            .map(|p| p.enrollment_id.clone())
            .collect();
        assert_eq!(enrollments.len(), 2, "{enrollments:?}");
        assert!(enrollments.contains(&old_enrollment), "history is kept");
        assert_eq!(
            a.read("Beta/main.lua").as_deref(),
            Some("beta-v1"),
            "the re-enrolled computer's addon reaches the group"
        );
        // The old enrollment is still retired: re-enrollment is not an undo of the record.
        assert_eq!(
            a.svc.command(Command::Retire {
                subject: wow_sync_core::kinds::EnrollmentRef {
                    computer_id: pc_ids(&b).0,
                    enrollment_id: old_enrollment,
                }
            }),
            Ok(CommandOutcome::NothingToDo)
        );
    }

    #[test]
    fn restoring_a_removed_addon_brings_it_back_and_it_is_not_removed_again() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        let mut c = lab.pc("c");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        c.join(&g);
        synced(&mut c);
        synced(&mut b);
        a_removes_alpha(&mut a, &g);
        let (ac, _) = pc_ids(&a);
        let removal = removal_hash(&lab, &g, &ac);
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua"), None, "the removal applied first");

        let ghost = sha256(b"no such removal");
        assert_eq!(
            b.svc.command(Command::RestoreRemoved { removal: ghost }),
            Err(CommandError::UnknownRemoval(ghost))
        );
        let (bc, _) = pc_ids(&b);
        assert!(authority_files(&lab, &g, &bc).iter().all(|f| !f.is_empty()));
        let before = authority_files(&lab, &g, &bc).len();
        let out = b.svc.command(Command::RestoreRemoved { removal }).unwrap();
        let CommandOutcome::Written(readd) = out else {
            panic!("{out:?}")
        };
        assert_eq!(authority_files(&lab, &g, &bc).len(), before + 1);
        for _ in 0..3 {
            synced(&mut b);
            synced(&mut c);
            synced(&mut a);
        }
        assert_eq!(status(&b, &g, &readd), Some(Status::Valid));
        let rc = synced(&mut c);
        assert!(!rc.crashed);
        for pc in [&b, &a, &c] {
            assert_eq!(
                pc.read("Alpha/main.lua").as_deref(),
                Some("alpha-v1"),
                "the code returned and stays"
            );
        }
        let removals = |pc: &Pc| {
            let p = pc.svc.plane();
            p.ledger
                .entries()
                .filter(|e| e.kind == wow_sync_core::kinds::AuthorityKind::Removal)
                .count()
        };
        for pc in [&a, &b, &c] {
            assert_eq!(
                removals(pc),
                1,
                "no new removal is written after the restore"
            );
        }
    }

    /// A settings checkpoint of one synthetic file in `pc`'s local store.
    fn stored_checkpoint(pc: &Pc, unit: &wow_sync_core::ids::SettingsUnit, bytes: &[u8]) -> Sha256 {
        use wow_sync_core::records::{encode, CheckpointFile, SettingsCheckpointManifest};
        let store = wow_sync_adapters::settings_store::SettingsStore::open(
            &pc.cfg.install_root.join(&pc.cfg.variant_folder),
        )
        .unwrap();
        let blob = store.put_blob(bytes).unwrap();
        let manifest = SettingsCheckpointManifest {
            unit: unit.clone(),
            files: vec![CheckpointFile {
                name: "Alpha.lua".into(),
                size: bytes.len() as u64,
                blob: Sha256(blob),
            }],
        };
        Sha256(store.put_manifest(&encode(&manifest), &[blob]).unwrap())
    }

    #[test]
    fn settings_policy_and_push_commands_record_explicit_intent_only() {
        use wow_sync_core::kinds::{PolicyRule, PolicyRuleKind, RevisionKind};
        use wow_sync_core::settings::SettingsWriteError as W;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        let alpha = wow_sync_core::paths::validate_folder_name("Alpha").unwrap();
        let beta = wow_sync_core::paths::validate_folder_name("Beta").unwrap();
        let unit = wow_sync_core::ids::SettingsUnit {
            folder: alpha.clone(),
            identity: wow_sync_core::ids::Identity::account("ACC").unwrap(),
        };
        let cp = stored_checkpoint(&b, &unit, b"settings one");

        // Nothing is written without a policy, a stored checkpoint or a live member.
        assert_eq!(
            b.svc.command(Command::ShareSettings {
                unit: unit.clone(),
                checkpoint: cp
            }),
            Err(CommandError::Settings(W::NoPolicy))
        );
        let ghost = sha256(b"no such checkpoint");
        assert_eq!(
            b.svc.command(Command::ShareSettings {
                unit: unit.clone(),
                checkpoint: ghost
            }),
            Err(CommandError::UnknownCheckpoint(ghost))
        );
        let nobody = EnrollmentRef {
            computer_id: ComputerId::parse(&"9".repeat(32)).unwrap(),
            enrollment_id: EnrollmentId::parse(&"8".repeat(32)).unwrap(),
        };
        assert_eq!(
            a.svc.command(Command::SetSettingsPolicy {
                master: Some(nobody),
                exceptions: vec![]
            }),
            Err(CommandError::Settings(W::NotLive))
        );
        let malformed = PolicyRule {
            folder: alpha.clone(),
            rule: PolicyRuleKind::NotShared,
            computer_id: Some(pc_ids(&b).0),
            enrollment_id: None,
        };
        assert_eq!(
            a.svc.command(Command::SetSettingsPolicy {
                master: None,
                exceptions: vec![malformed]
            }),
            Err(CommandError::Settings(W::BadRule))
        );
        let not_shared = |f: &wow_sync_core::paths::FolderKey| PolicyRule {
            folder: f.clone(),
            rule: PolicyRuleKind::NotShared,
            computer_id: None,
            enrollment_id: None,
        };
        assert_eq!(
            a.svc.command(Command::SetSettingsPolicy {
                master: None,
                exceptions: vec![not_shared(&beta), not_shared(&beta)]
            }),
            Err(CommandError::Settings(W::BadRule)),
            "one rule per folder"
        );

        // The master is chosen explicitly; a local-only exception is respected.
        let out = a
            .svc
            .command(Command::SetSettingsPolicy {
                master: Some(ref_of(&a)),
                exceptions: vec![not_shared(&beta)],
            })
            .unwrap();
        let CommandOutcome::Written(policy) = out else {
            panic!("{out:?}")
        };
        synced(&mut a);
        synced(&mut b);
        assert_eq!(status(&b, &g, &policy), Some(Status::Valid));
        assert_eq!(
            b.svc.command(Command::ShareSettings {
                unit: wow_sync_core::ids::SettingsUnit {
                    folder: beta.clone(),
                    identity: unit.identity.clone()
                },
                checkpoint: cp
            }),
            Err(CommandError::Settings(W::NotShared)),
            "a not-shared folder publishes nothing"
        );

        // b is not the master; its explicit push is a valid push, never an automatic revision.
        let out = b
            .svc
            .command(Command::ShareSettings {
                unit: unit.clone(),
                checkpoint: cp,
            })
            .unwrap();
        let CommandOutcome::Written(push) = out else {
            panic!("{out:?}")
        };
        synced(&mut b);
        synced(&mut a);
        assert_eq!(status(&a, &g, &push), Some(Status::Valid));
        let history = a.svc.settings_history(&unit);
        assert_eq!(history.len(), 1, "{history:?}");
        assert_eq!(history[0].hash, push);
        assert_eq!(history[0].kind, RevisionKind::Push);
        assert!(history[0].is_head);
        assert!(a
            .svc
            .settings_history(&wow_sync_core::ids::SettingsUnit {
                folder: beta,
                identity: unit.identity.clone()
            })
            .is_empty());

        // A later push builds on the head, which stops being one.
        let cp2 = stored_checkpoint(&b, &unit, b"settings two");
        let out = b
            .svc
            .command(Command::ShareSettings {
                unit: unit.clone(),
                checkpoint: cp2,
            })
            .unwrap();
        let CommandOutcome::Written(push2) = out else {
            panic!("{out:?}")
        };
        synced(&mut b);
        synced(&mut a);
        let history = a.svc.settings_history(&unit);
        assert_eq!(history.len(), 2);
        let heads: Vec<_> = history
            .iter()
            .filter(|e| e.is_head)
            .map(|e| e.hash)
            .collect();
        assert_eq!(heads, vec![push2]);

        // A retired computer records no settings intent.
        a.svc
            .command(Command::Retire {
                subject: ref_of(&b),
            })
            .unwrap();
        synced(&mut a);
        synced(&mut b);
        assert_eq!(
            b.svc.command(Command::SetSettingsPolicy {
                master: Some(ref_of(&b)),
                exceptions: vec![]
            }),
            Err(CommandError::NotAllowed)
        );
    }

    const SV_TOC: &str = "## Interface: 11508\n## Version: 1.0\n## SavedVariables: AlphaDB\n";

    fn wtf_file(pc: &Pc, folder: &str) -> PathBuf {
        pc.install
            .variant_dir
            .join(format!("WTF/Account/ACC/SavedVariables/{folder}.lua"))
    }

    fn write_wtf(pc: &Pc, folder: &str, text: &str) {
        let p = wtf_file(pc, folder);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
    }

    fn read_wtf(pc: &Pc, folder: &str) -> Option<String> {
        std::fs::read_to_string(wtf_file(pc, folder)).ok()
    }

    /// Two computers with the same addon that saves variables, settings consent on both, and
    /// `master` chosen (`a` is the master when true, `b` otherwise). Test cap is on for both.
    fn settings_pair(lab: &Lab, a_is_master: bool, cap: Option<u64>) -> (Pc, Pc, GroupId) {
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon_with("Alpha", "alpha-v1", SV_TOC);
        b.addon_with("Alpha", "alpha-v1", SV_TOC);
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        a.svc.set_settings_cap_for_tests(cap);
        b.svc.set_settings_cap_for_tests(cap);
        let master = if a_is_master { ref_of(&a) } else { ref_of(&b) };
        let out = a
            .svc
            .command(Command::SetSettingsPolicy {
                master: Some(master),
                exceptions: vec![],
            })
            .unwrap();
        assert!(matches!(out, CommandOutcome::Written(_)));
        for pc in [&mut a, &mut b] {
            assert_eq!(
                pc.svc.command(Command::EnableSettings {
                    included_accounts: vec!["ACC".into()]
                }),
                Ok(CommandOutcome::Recorded)
            );
        }
        synced(&mut a);
        synced(&mut b);
        (a, b, g)
    }

    #[test]
    fn the_masters_settings_change_reaches_a_peer() {
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a); // activation capture: never published
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 2 }\n")
        );
        // The next cycles settle: nothing is applied or published again.
        let r = synced(&mut b);
        assert!(!texts(&r).contains("settings"), "{}", texts(&r));
        synced(&mut a);
        assert_eq!(
            read_wtf(&a, "Alpha").as_deref(),
            Some("AlphaDB = { v = 2 }\n")
        );
    }

    #[test]
    fn with_no_approved_cap_nothing_of_settings_is_read_published_or_applied() {
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, None);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        assert_eq!(read_wtf(&b, "Alpha"), None);
        assert!(a.svc.settings_history(&alpha_unit()).is_empty());
    }

    fn alpha_unit() -> wow_sync_core::ids::SettingsUnit {
        wow_sync_core::ids::SettingsUnit {
            folder: wow_sync_core::paths::validate_folder_name("Alpha").unwrap(),
            identity: wow_sync_core::ids::Identity::account("ACC").unwrap(),
        }
    }

    #[test]
    fn a_non_master_routine_write_yields_to_the_master_and_keeps_a_recovery_copy() {
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        // b's own routine edit is not published; the master's next revision replaces it.
        synced(&mut b);
        write_wtf(&b, "Alpha", "AlphaDB = { mine = true }\n");
        synced(&mut b);
        synced(&mut a);
        assert_eq!(
            a.svc.settings_history(&alpha_unit()).len(),
            1,
            "a non-master's routine write publishes nothing"
        );
        write_wtf(&a, "Alpha", "AlphaDB = { v = 3 }\n");
        synced(&mut a);
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 3 }\n")
        );
        let recovery = b
            .install
            .variant_dir
            .join(".wowsync")
            .join("settings-recovery");
        let mut found = false;
        let mut stack = vec![recovery];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if std::fs::read_to_string(&p).is_ok_and(|t| t.contains("mine = true")) {
                    found = true;
                }
            }
        }
        assert!(found, "the replaced file is kept in settings-recovery");
    }

    #[test]
    fn a_per_addon_exception_is_respected() {
        use wow_sync_core::kinds::{PolicyRule, PolicyRuleKind};
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        let alpha = wow_sync_core::paths::validate_folder_name("Alpha").unwrap();
        let rule = PolicyRule {
            folder: alpha,
            rule: PolicyRuleKind::NotShared,
            computer_id: None,
            enrollment_id: None,
        };
        a.svc
            .command(Command::SetSettingsPolicy {
                master: Some(ref_of(&a)),
                exceptions: vec![rule],
            })
            .unwrap();
        synced(&mut a);
        synced(&mut b);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        assert!(a.svc.settings_history(&alpha_unit()).is_empty());
        assert_eq!(
            read_wtf(&b, "Alpha"),
            None,
            "a not-shared folder is never applied"
        );
    }

    fn settings_recovery_holds(pc: &Pc, text: &str) -> bool {
        let mut stack = vec![pc
            .install
            .variant_dir
            .join(".wowsync")
            .join("settings-recovery")];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if std::fs::read_to_string(&p).is_ok_and(|t| t.contains(text)) {
                    return true;
                }
            }
        }
        false
    }

    #[test]
    fn pausing_settings_holds_a_units_updates_until_resume_and_resume_never_publishes() {
        use wow_sync_core::settings::ResumeOption;
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        let alpha = alpha_unit().folder;
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 2 }\n")
        );
        let beta = wow_sync_core::paths::validate_folder_name("Beta").unwrap();
        assert_eq!(
            b.svc.command(Command::PauseSettings {
                folder: beta.clone()
            }),
            Err(CommandError::UnknownFolder(beta)),
            "only an installed folder can be paused"
        );
        assert_eq!(
            b.svc.command(Command::PauseSettings {
                folder: alpha.clone()
            }),
            Ok(CommandOutcome::Recorded)
        );
        assert_eq!(
            b.svc.command(Command::PauseSettings {
                folder: alpha.clone()
            }),
            Ok(CommandOutcome::NothingToDo)
        );
        write_wtf(&a, "Alpha", "AlphaDB = { v = 3 }\n");
        synced(&mut a);
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 2 }\n"),
            "paused: the newer copy is not applied"
        );
        // Resume never publishes, so "Share mine" is refused here and the unit stays paused.
        assert_eq!(
            b.svc.command(Command::ResumeSettings {
                folder: alpha.clone(),
                choice: ResumeOption::ShareMine
            }),
            Err(CommandError::UseShareSettings)
        );
        assert_eq!(
            b.svc.command(Command::ResumeSettings {
                folder: alpha.clone(),
                choice: ResumeOption::StayPaused
            }),
            Ok(CommandOutcome::NothingToDo)
        );
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 2 }\n")
        );
        assert_eq!(
            b.svc.command(Command::ResumeSettings {
                folder: alpha.clone(),
                choice: ResumeOption::UseShared
            }),
            Ok(CommandOutcome::Recorded)
        );
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 3 }\n")
        );
        assert_eq!(
            b.svc.settings_history(&alpha_unit()).len(),
            2,
            "resuming wrote no revision"
        );
        assert_eq!(
            b.svc.command(Command::ResumeSettings {
                folder: alpha,
                choice: ResumeOption::Resume
            }),
            Ok(CommandOutcome::NothingToDo),
            "a unit that is not paused has nothing to resume"
        );
    }

    #[test]
    fn use_shared_settings_replaces_a_local_difference_and_keeps_it_as_a_recovery_copy() {
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        synced(&mut b); // the next cycle records the applied revision as the basis
        write_wtf(&b, "Alpha", "AlphaDB = { mine = true }\n");
        synced(&mut b);
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { mine = true }\n"),
            "a local difference is never replaced by itself"
        );
        let other = wow_sync_core::ids::SettingsUnit {
            folder: wow_sync_core::paths::validate_folder_name("Beta").unwrap(),
            identity: wow_sync_core::ids::Identity::account("ACC").unwrap(),
        };
        assert_eq!(
            b.svc.command(Command::UseSharedSettings {
                unit: other.clone()
            }),
            Err(CommandError::NoSharedSettings(other)),
            "a unit with nothing shared has nothing to use"
        );
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { mine = true }\n")
        );
        assert_eq!(
            b.svc
                .command(Command::UseSharedSettings { unit: alpha_unit() }),
            Ok(CommandOutcome::Recorded)
        );
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 2 }\n")
        );
        assert!(settings_recovery_holds(&b, "mine = true"));
        write_wtf(&b, "Alpha", "AlphaDB = { again = true }\n");
        synced(&mut b);
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { again = true }\n"),
            "the request was spent by the first yield"
        );
    }

    fn two_option_item(
        key: &wow_sync_core::paths::FolderKey,
        write: AuthorityRecord,
        basis: &[u8],
    ) -> (
        wow_sync_core::decision::DecisionItem,
        wow_sync_core::decide::Plans,
    ) {
        use wow_sync_core::decide::{build_item, PlannedOption};
        use wow_sync_core::decision::*;
        let opt = |id: u32, label| DecisionOption {
            id: OptionId(id),
            label,
            holders: vec![],
            consequence: Consequence::default(),
            warning: None,
        };
        build_item(
            DecisionScope::Folders(vec![key.clone()]),
            DecisionKind::ChoicesDiffer,
            DecisionReach::Group,
            vec![
                PlannedOption {
                    option: opt(0, OptionLabel::KeepOnThisComputer),
                    effect: DecisionEffect::Local(LocalResolution::Dismiss(sha256(b"x"))),
                },
                PlannedOption {
                    option: opt(1, OptionLabel::RemoveEverywhere),
                    effect: DecisionEffect::Write(Box::new(write)),
                },
            ],
            &[sha256(basis)],
        )
    }

    #[test]
    fn decide_command_is_checked_for_freshness_and_kept_while_paused() {
        use wow_sync_core::decision::{DecideError, DecisionId, LocalResolution, OptionId};
        let lab = Lab::new();
        let (mut a, b, _c, g) = alpha_v2_published(&lab);
        let (ac, _) = pc_ids(&a);
        let key = alpha_folder(&lab, &g, &a).key;
        let write = AuthorityRecord::Retirement(Retirement {
            group_id: g.clone(),
            issuer: issuer_of(&a),
            retired: ref_of(&b),
            cutoff_seq: 0,
            was_settings_master: false,
            retirements_known: vec![],
            written_unix_ms: 1,
        });
        let (item, plans) = two_option_item(&key, write.clone(), b"basis-1");
        let id = DecisionId(item.scope.clone());
        a.svc.offer_decisions(vec![item.clone()], plans);
        let out = a
            .svc
            .command(Command::Decide {
                item: id.clone(),
                option: OptionId(0),
                shown: item.evidence,
            })
            .unwrap();
        assert_eq!(
            out,
            CommandOutcome::Local(LocalResolution::Dismiss(sha256(b"x")))
        );
        // The evidence changed after the owner looked: nothing is written.
        let (newer, plans) = two_option_item(&key, write, b"basis-2");
        assert_ne!(newer.evidence, item.evidence);
        a.svc.offer_decisions(vec![newer.clone()], plans);
        let stale = Command::Decide {
            item: id.clone(),
            option: OptionId(1),
            shown: item.evidence,
        };
        assert_eq!(
            a.svc.command(stale),
            Err(CommandError::Decide(DecideError::Changed))
        );
        assert_eq!(
            a.svc.command(Command::Decide {
                item: id.clone(),
                option: OptionId(9),
                shown: newer.evidence,
            }),
            Err(CommandError::Decide(DecideError::UnknownOption))
        );
        assert!(authority_files(&lab, &g, &ac).is_empty());
        // Paused: the answer is kept, then written once syncing resumes.
        a.svc.set_paused(true).unwrap();
        let fresh = Command::Decide {
            item: id,
            option: OptionId(1),
            shown: newer.evidence,
        };
        assert_eq!(a.svc.command(fresh), Ok(CommandOutcome::Kept));
        assert!(authority_files(&lab, &g, &ac).is_empty());
        a.svc.set_paused(false).unwrap();
        synced(&mut a);
        let files = authority_files(&lab, &g, &ac);
        assert_eq!(files.len(), 1, "{files:?}");
    }

    #[test]
    fn allow_again_lifts_only_an_active_rejection() {
        use wow_sync_core::hash::package_hash;
        use wow_sync_core::kinds::{
            Considered, RecoveredEntry, RejectedEntry, Rejection, RejectionSettings, TreeEntry,
        };
        let lab = Lab::new();
        let (mut a, b, _c, g) = alpha_v2_published(&lab);
        let (ac, _) = pc_ids(&a);
        let bad = alpha_folder(&lab, &g, &b);
        let good = alpha_folder(&lab, &g, &a);
        let entry = |f: &wow_sync_core::inventory::FolderState| TreeEntry {
            key: f.key.clone(),
            tree: f.tree.unwrap(),
        };
        let hash = |f: &wow_sync_core::inventory::FolderState| {
            package_hash(&[(f.key.clone(), f.tree.unwrap())])
        };
        let mut considered = vec![
            Considered {
                package_hash: hash(&bad),
                folders: vec![entry(&bad)],
                holders: vec![pc_ids(&b).1],
            },
            Considered {
                package_hash: hash(&good),
                folders: vec![entry(&good)],
                holders: vec![pc_ids(&a).1],
            },
        ];
        considered.sort();
        let rej = Rejection {
            group_id: g.clone(),
            issuer: issuer_of(&a),
            folders: vec![bad.key.clone()],
            rejected: RejectedEntry {
                package_hash: hash(&bad),
                version_label: "3.0".into(),
                folders: vec![entry(&bad)],
            },
            recovered: RecoveredEntry {
                package_hash: hash(&good),
                folders: vec![entry(&good)],
            },
            considered,
            settings: RejectionSettings::NotApplicable,
            known: Default::default(),
            considered_parts: vec![],
            written_unix_ms: 5,
        };
        let rh = put_authority(&lab, &g, &ac, &AuthorityRecord::Rejection(rej));
        synced(&mut a);
        let unknown = sha256(b"no such rejection");
        assert_eq!(
            a.svc.command(Command::AllowAgain {
                rejections: vec![unknown]
            }),
            Err(CommandError::UnknownRejection(unknown))
        );
        assert_eq!(
            a.svc.command(Command::AllowAgain { rejections: vec![] }),
            Err(CommandError::EmptySelection)
        );
        let out = a
            .svc
            .command(Command::AllowAgain {
                rejections: vec![rh],
            })
            .unwrap();
        let CommandOutcome::Written(lift) = out else {
            panic!("{out:?}")
        };
        synced(&mut a);
        assert_eq!(status(&a, &g, &lift), Some(Status::Valid));
        // The rejection is no longer active, so it cannot be lifted twice.
        assert_eq!(
            a.svc.command(Command::AllowAgain {
                rejections: vec![rh]
            }),
            Err(CommandError::UnknownRejection(rh))
        );
    }

    /// A keep of `owner`'s own Alpha release, written into `owner`'s authority folder.
    fn put_keep_of_alpha(lab: &Lab, g: &GroupId, owner: &Pc) -> Sha256 {
        use wow_sync_core::choice::{build_keep, ChoiceView, CopyEntry, WriteCx};
        use wow_sync_core::kinds::{ChoiceReason, ManifestFolder};
        use wow_sync_core::ledger::Ledger;
        use wow_sync_core::target::Source;
        let (oc, oe) = pc_ids(owner);
        let kept = alpha_folder(lab, g, owner);
        let copy = CopyEntry::new(
            vec![ManifestFolder {
                key: kept.key.clone(),
                name: kept.name.clone(),
                tree: kept.tree.unwrap(),
            }],
            vec![oe],
        );
        let ledger = Ledger::new();
        let view = ChoiceView::new(&ledger, &Default::default());
        let keep = build_keep(
            &view,
            &WriteCx {
                group: g,
                issuer: issuer_of(owner),
                now_unix_ms: 5,
            },
            ChoiceReason::Ambiguous,
            std::slice::from_ref(&kept.key),
            &copy,
            &[],
            &Source::from_state(&latest_state(lab, g, owner), true),
        )
        .unwrap();
        put_authority(lab, g, &oc, &AuthorityRecord::CodeChoice(keep))
    }

    fn offered_kinds(pc: &Pc) -> Vec<wow_sync_core::decision::DecisionKind> {
        pc.svc
            .offered_decisions()
            .iter()
            .map(|i| i.reason)
            .collect()
    }

    #[test]
    fn a_newer_version_after_a_keep_is_offered_while_the_kept_version_stays() {
        use wow_sync_core::decision::DecisionKind;
        let lab = Lab::new();
        let (mut a, _b, _c, _g) = alpha_v2_published(&lab);
        // Control: with no keep a plain newer release is simply taken, nothing is asked.
        synced(&mut a);
        assert!(offered_kinds(&a).is_empty(), "{:?}", offered_kinds(&a));
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v2"));

        let lab = Lab::new();
        let (mut a, _b, _c, g2) = alpha_v2_published(&lab);
        let kh = put_keep_of_alpha(&lab, &g2, &a);
        for _ in 0..2 {
            synced(&mut a);
            assert_eq!(status(&a, &g2, &kh), Some(Status::Valid));
            assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
            assert_eq!(
                offered_kinds(&a),
                vec![DecisionKind::NewVersionAfterDecision]
            );
        }
    }

    #[test]
    fn keep_and_resume_commands_pin_and_release_a_typed_scope() {
        use wow_sync_core::choice::WriteError;
        use wow_sync_core::hash::package_hash;
        use wow_sync_core::kinds::ChoiceReason;
        let lab = Lab::new();
        let (mut a, b, _c, g) = alpha_v2_published(&lab);
        let key = alpha_folder(&lab, &g, &a).key;
        let v1 = alpha_folder(&lab, &g, &a).tree.unwrap();
        let v2 = package_hash(&[(key.clone(), alpha_folder(&lab, &g, &b).tree.unwrap())]);
        let scope = vec![key.clone()];
        let why = ChoiceReason::Ambiguous;
        // Negative cases write nothing.
        let ghost = sha256(b"no such release");
        assert_eq!(
            a.svc.command(Command::Keep {
                scope: scope.clone(),
                copy: ghost,
                why
            }),
            Err(CommandError::UnknownCopy(ghost))
        );
        assert_eq!(
            a.svc.command(Command::Keep {
                scope: vec![],
                copy: ghost,
                why
            }),
            Err(CommandError::UnknownCopy(ghost))
        );
        assert_eq!(
            a.svc.command(Command::Resume {
                scope: scope.clone(),
                allow_again: vec![ghost],
                why
            }),
            Err(CommandError::Write(WriteError::AllowAgainNotInherited(
                ghost
            )))
        );
        assert_eq!(
            a.svc.command(Command::Resume {
                scope: vec![],
                allow_again: vec![],
                why
            }),
            Err(CommandError::Write(WriteError::EmptyScope))
        );
        let (ac, _) = pc_ids(&a);
        assert!(authority_files(&lab, &g, &ac).is_empty());

        let kept = package_hash(&[(key.clone(), v1)]);
        let out = a
            .svc
            .command(Command::Keep {
                scope: scope.clone(),
                copy: kept,
                why,
            })
            .unwrap();
        let CommandOutcome::Written(kh) = out else {
            panic!("{out:?}")
        };
        for _ in 0..2 {
            synced(&mut a);
            assert_eq!(status(&a, &g, &kh), Some(Status::Valid));
            assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        }
        assert!(
            offered_kinds(&a).is_empty(),
            "the newer release was shown when the keep was made, so it is not new"
        );
        let out = a
            .svc
            .command(Command::Resume {
                scope: scope.clone(),
                allow_again: vec![],
                why,
            })
            .unwrap();
        let CommandOutcome::Written(rh) = out else {
            panic!("{out:?}")
        };
        for _ in 0..2 {
            synced(&mut a);
        }
        assert_eq!(status(&a, &g, &rh), Some(Status::Valid));
        assert_eq!(
            a.read("Alpha/main.lua").as_deref(),
            Some("alpha-v1"),
            "a resume inherits the exclusion of the release the keep passed over (Q-50)"
        );
        let out = a
            .svc
            .command(Command::Resume {
                scope,
                allow_again: vec![v2],
                why,
            })
            .unwrap();
        let CommandOutcome::Written(ah) = out else {
            panic!("{out:?}")
        };
        for _ in 0..2 {
            synced(&mut a);
        }
        assert_eq!(status(&a, &g, &ah), Some(Status::Valid));
        assert_eq!(
            a.read("Alpha/main.lua").as_deref(),
            Some("alpha-v2"),
            "allowing the excluded release again lets the group's newer release in"
        );
    }

    #[test]
    fn two_different_concurrent_keeps_raise_a_choices_differ_item() {
        use wow_sync_core::decision::DecisionKind;
        let lab = Lab::new();
        let (mut a, b, _c, g) = alpha_v2_published(&lab);
        put_keep_of_alpha(&lab, &g, &a);
        put_keep_of_alpha(&lab, &g, &b);
        for _ in 0..2 {
            let r = synced(&mut a);
            assert_eq!(
                a.read("Alpha/main.lua").as_deref(),
                Some("alpha-v1"),
                "{r:?}"
            );
            assert!(
                offered_kinds(&a).contains(&DecisionKind::ChoicesDiffer),
                "{:?}",
                offered_kinds(&a)
            );
        }
        let item = a
            .svc
            .offered_decisions()
            .iter()
            .find(|i| i.reason == DecisionKind::ChoicesDiffer)
            .unwrap()
            .clone();
        assert!(item.options.len() >= 2, "a choice is offered, none made");
    }
}
