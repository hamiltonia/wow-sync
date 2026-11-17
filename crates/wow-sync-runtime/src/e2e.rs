//! End-to-end slice cases (tests.md §5): real adapters, synthetic installs under a temp
//! directory, one temp directory standing in for the shared provider folder.

#[cfg(test)]
mod tests {
    use crate::*;
    use std::path::{Path, PathBuf};
    use wow_sync_adapters::fs_install::InstallArea;
    use wow_sync_adapters::localstate::LocalState;
    use wow_sync_adapters::testkit::{SyntheticInstall, TempDir};
    use wow_sync_core::hash::sha256;
    use wow_sync_core::ids::GroupId;
    use wow_sync_core::layout;
    use wow_sync_core::plan::Activity;

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

    #[test]
    fn different_build_same_interface_applies() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        b.set_build("1.0.0.2");
        a.addon("Alpha", "alpha-v1");
        b.addon("Alpha", "alpha-v0");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        let r = synced(&mut b);
        assert_eq!(r.applied.len(), 1, "{}", texts(&r));
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
    }

    fn other_build_update_blocked(candidate_toc: &str, local_has_addon: bool) {
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
        assert!(r.applied.is_empty(), "{}", texts(&r));
        let kept = b.read("Alpha/main.lua");
        assert_eq!(kept.as_deref(), local_has_addon.then_some("alpha-v0"));
        assert!(texts(&r).contains("another game build"), "{}", texts(&r));
    }

    #[test]
    fn different_build_different_interface_needs_decision() {
        other_build_update_blocked("## Interface: 99999\n## Version: 2.0\n", true);
    }

    #[test]
    fn different_build_malformed_interface_needs_decision() {
        other_build_update_blocked("## Interface: 11508,garbage\n## Version: 2.0\n", true);
    }

    #[test]
    fn different_build_new_addon_needs_decision() {
        other_build_update_blocked("## Interface: 11508\n## Version: 2.0\n", false);
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
            t.contains("on this computer") && t.contains("computer a"),
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
        assert!(line.contains("from computer"), "{line}");
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
}
