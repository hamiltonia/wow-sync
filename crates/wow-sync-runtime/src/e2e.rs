//! End-to-end slice cases (tests.md §5): real adapters, synthetic installs under a temp
//! directory, one temp directory standing in for the shared provider folder.

#[cfg(test)]
mod tests {
    use crate::control::{AppToggles, Command, CommandError, CommandOutcome};
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

        /// A folder with no `.toc` at all: unrecognised, synced by content (R-05, D-29).
        fn bare(&self, name: &str, content: &str) {
            let dir = self.install.addons.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("main.lua"), content).unwrap();
            self.install.set_old_mtimes();
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

    fn many_addons(pc: &Pc, n: usize) {
        for i in 0..n {
            pc.addon(&format!("Many{i}"), "m-v1");
        }
    }

    /// Runs `work` on this thread while another sets the cancel flag after `after`, and returns
    /// how long `work` kept going after the flag was set.
    fn cancelled_after<R>(
        flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
        after: std::time::Duration,
        work: impl FnOnce() -> R,
    ) -> (R, std::time::Duration) {
        let set_at = std::sync::Arc::new(std::sync::Mutex::new(None));
        let (f, s) = (flag.clone(), set_at.clone());
        let t = std::thread::spawn(move || {
            std::thread::sleep(after);
            *s.lock().unwrap() = Some(std::time::Instant::now());
            f.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        let r = work();
        let ended = std::time::Instant::now();
        t.join().unwrap();
        let set = set_at.lock().unwrap().expect("the flag was set");
        (r, ended.saturating_duration_since(set))
    }

    #[test]
    fn quit_abandons_a_slow_sync_status_and_enroll_scan_within_a_bound() {
        use std::time::Duration;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        many_addons(&a, 40);
        a.create();
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let slow = |flag: &std::sync::Arc<std::sync::atomic::AtomicBool>| ScanCtl {
            cancel: flag.clone(),
            ..ScanCtl::slowed(25)
        };
        // One scan takes about a second; a whole cycle needs several.
        a.svc.set_read_ctl(slow(&flag));
        let (res, late) = cancelled_after(&flag, Duration::from_millis(150), || a.svc.sync_once());
        assert!(matches!(res, Err(ServiceError::Cancelled)), "{res:?}");
        assert!(
            late < Duration::from_millis(500),
            "sync kept going {late:?}"
        );
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        a.svc.set_read_ctl(slow(&flag));
        let (st, late) = cancelled_after(&flag, Duration::from_millis(150), || a.svc.status());
        assert!(st.enrolled && st.provider_error.is_none(), "{st:?}");
        assert!(
            late < Duration::from_millis(500),
            "status kept going {late:?}"
        );
        // Nothing was lost: with the flag clear the cycle completes normally.
        a.svc.set_read_ctl(ScanCtl::default());
        assert!(a.svc.sync_once().is_ok());

        let mut b = lab.pc("b");
        many_addons(&b, 40);
        let req = b.confirm_request(JoinChoice::New { name: "Two".into() });
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        b.svc.set_read_ctl(slow(&flag));
        let (res, late) = cancelled_after(&flag, Duration::from_millis(150), || b.svc.enroll(&req));
        assert!(matches!(res, Err(ServiceError::Cancelled)), "{res:?}");
        assert!(
            late < Duration::from_millis(500),
            "enroll kept going {late:?}"
        );
        assert!(!b.svc.is_enrolled(), "an abandoned enroll leaves no record");
    }

    struct QuitWhenWriting {
        flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
        target: PathBuf,
    }

    impl ActivitySource for QuitWhenWriting {
        // The apply step asks for the game's activity just before it writes.
        fn observe(&mut self, _: &Path) -> Activity {
            if !self.target.exists() {
                self.flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Activity::NotRunning
        }
    }

    #[test]
    fn a_write_in_progress_finishes_whole_even_when_quit_is_requested() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let src = QuitWhenWriting {
            flag: flag.clone(),
            target: b.install.addons.join("Alpha").join("main.lua"),
        };
        b.svc = SliceService::with_activity(b.cfg.clone(), Box::new(src)).unwrap();
        b.svc.set_read_ctl(ScanCtl {
            cancel: flag.clone(),
            ..ScanCtl::default()
        });
        let r = synced(&mut b);
        assert!(
            flag.load(std::sync::atomic::Ordering::Relaxed),
            "quit came before the write"
        );
        assert_eq!(
            b.read("Alpha/main.lua").as_deref(),
            Some("alpha-v1"),
            "{r:?}"
        );
        assert_eq!(
            b.journals(),
            0,
            "the write was completed, not left half done"
        );
        assert!(r.published.is_some() || !r.applied.is_empty(), "{r:?}");
        assert!(matches!(b.svc.sync_once(), Err(ServiceError::Cancelled)));
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
    fn unrecognised_folder_is_published_by_content_and_says_so() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Good", "good-v1");
        a.install.add_addon(
            "Odd",
            "Odd_Foreign.toc",
            "## Interface: 11508\n## Version: 9.0\n",
            &[("main.lua", b"odd-content")],
        );
        a.install.set_old_mtimes();
        let g = a.create();
        let r = synced(&mut a);
        assert!(r.published.is_some(), "{r:?}");
        let rel = layout::blob(&g, &sha256(b"odd-content"));
        let p: PathBuf = rel
            .split('/')
            .fold(lab.shared.join("WoWSync"), |x, s| x.join(s));
        assert!(p.exists(), "unrecognised content is uploaded");
        let line = r
            .status
            .packages
            .iter()
            .find(|p| p.key == "odd")
            .map(|p| p.text.clone())
            .unwrap_or_default();
        assert!(
            line.contains("Unrecognised: synced by content") && !line.contains("not synced"),
            "{line} / {}",
            texts(&r)
        );
        let good = r.status.packages.iter().find(|p| p.key == "good").unwrap();
        assert!(!good.text.contains("Unrecognised"), "{}", good.text);
    }

    fn converge(a: &mut Pc, b: &mut Pc) {
        for _ in 0..4 {
            synced(a);
            synced(b);
        }
    }

    #[test]
    fn unrecognised_folder_only_on_a_installs_on_b_and_later_edits_follow() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.bare("Bare", "v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        converge(&mut a, &mut b);
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("v1"));
        a.bare("Bare", "v2");
        converge(&mut a, &mut b);
        assert_eq!(a.read("Bare/main.lua").as_deref(), Some("v2"));
        assert_eq!(
            b.read("Bare/main.lua").as_deref(),
            Some("v2"),
            "the copy changed since the shared release wins without a question"
        );
        assert!(offered_kinds(&a).is_empty() && offered_kinds(&b).is_empty());
        assert!(
            recovery_holds(&b, "v1"),
            "the replaced copy is kept for recovery like any replacement"
        );
    }

    fn recovery_holds(pc: &Pc, content: &str) -> bool {
        has_file(
            &pc.install.variant_dir.join(".wowsync").join("recovery"),
            "main.lua",
            content.as_bytes(),
        )
    }

    fn decide_first_copy(pc: &mut Pc) {
        use wow_sync_core::decision::{DecisionId, OptionLabel};
        let item = pc.svc.offered_decisions()[0].clone();
        let option = item
            .options
            .iter()
            .find(|o| matches!(o.label, OptionLabel::Release { .. }))
            .expect("a copy option")
            .id;
        pc.svc
            .command(Command::Decide {
                item: DecisionId(item.scope.clone()),
                option,
                shown: item.evidence,
            })
            .unwrap();
    }

    fn three_at_v1(lab: &Lab) -> (Pc, Pc, Pc) {
        let (mut a, mut b, mut c) = (lab.pc("a"), lab.pc("b"), lab.pc("c"));
        a.bare("Bare", "v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        c.join(&g);
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
            synced(&mut c);
        }
        for pc in [&a, &b, &c] {
            assert_eq!(pc.read("Bare/main.lua").as_deref(), Some("v1"));
        }
        (a, b, c)
    }

    #[test]
    fn a_peer_that_was_offline_while_the_group_moved_on_is_replaced_not_obeyed() {
        let lab = Lab::new();
        let (mut a, mut b, mut c) = three_at_v1(&lab);
        a.bare("Bare", "v2");
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("v2"));
        // c has not synced: its v1 state is still published. The group must not go back to it.
        for _ in 0..3 {
            synced(&mut a);
            synced(&mut b);
            assert_eq!(a.read("Bare/main.lua").as_deref(), Some("v2"));
            assert_eq!(b.read("Bare/main.lua").as_deref(), Some("v2"));
            assert!(offered_kinds(&a).is_empty() && offered_kinds(&b).is_empty());
        }
        for _ in 0..4 {
            synced(&mut c);
            synced(&mut a);
            synced(&mut b);
        }
        for pc in [&a, &b, &c] {
            assert_eq!(pc.read("Bare/main.lua").as_deref(), Some("v2"));
            assert!(offered_kinds(pc).is_empty(), "{:?}", offered_kinds(pc));
        }
        assert!(recovery_holds(&c, "v1"), "c's replaced copy is kept");
    }

    #[test]
    fn a_returning_peer_with_a_changed_copy_gets_one_decision_and_nothing_is_overwritten() {
        let lab = Lab::new();
        let (mut a, mut b, mut c) = three_at_v1(&lab);
        a.bare("Bare", "v2");
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
        }
        c.bare("Bare", "c-local");
        for _ in 0..4 {
            synced(&mut c);
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(c.read("Bare/main.lua").as_deref(), Some("c-local"));
        assert_eq!(a.read("Bare/main.lua").as_deref(), Some("v2"));
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("v2"));
        for pc in [&a, &b, &c] {
            assert_eq!(
                offered_kinds(pc),
                vec![wow_sync_core::decision::DecisionKind::Ambiguous]
            );
        }
    }

    #[test]
    fn a_copy_that_lost_a_decision_cannot_come_back_from_an_offline_peer() {
        let lab = Lab::new();
        let (mut a, mut b, mut c) = three_at_v1(&lab);
        a.bare("Bare", "from-a");
        b.bare("Bare", "from-b");
        for _ in 0..3 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(
            offered_kinds(&a),
            vec![wow_sync_core::decision::DecisionKind::Ambiguous]
        );
        decide_first_copy(&mut a);
        // b goes offline with its copy still published; c comes back at v1.
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut c);
        }
        let winner = a.read("Bare/main.lua").unwrap();
        assert_eq!(c.read("Bare/main.lua").as_deref(), Some(winner.as_str()));
        assert!(offered_kinds(&c).is_empty(), "{:?}", offered_kinds(&c));
        for _ in 0..4 {
            synced(&mut b);
            synced(&mut a);
            synced(&mut c);
        }
        let loser = if winner == "from-a" {
            "from-b"
        } else {
            "from-a"
        };
        for pc in [&a, &b, &c] {
            assert_eq!(pc.read("Bare/main.lua").as_deref(), Some(winner.as_str()));
            assert!(offered_kinds(pc).is_empty(), "{:?}", offered_kinds(pc));
        }
        let holder = if loser == "from-b" { &b } else { &a };
        assert!(recovery_holds(holder, loser), "the losing copy is kept");
    }

    #[test]
    fn an_author_whose_peers_are_all_offline_publishes_the_change_without_a_decision() {
        let lab = Lab::new();
        let (mut a, mut b, mut c) = three_at_v1(&lab);
        a.bare("Bare", "v2");
        for _ in 0..4 {
            synced(&mut a);
            assert_eq!(a.read("Bare/main.lua").as_deref(), Some("v2"));
            assert!(offered_kinds(&a).is_empty(), "{:?}", offered_kinds(&a));
        }
        for _ in 0..3 {
            synced(&mut b);
            synced(&mut c);
            synced(&mut a);
        }
        for pc in [&a, &b, &c] {
            assert_eq!(pc.read("Bare/main.lua").as_deref(), Some("v2"));
            assert!(offered_kinds(pc).is_empty(), "{:?}", offered_kinds(pc));
        }
    }

    #[test]
    fn a_revert_to_an_earlier_copy_is_not_undone_silently_and_asks_once() {
        let lab = Lab::new();
        let (mut a, mut b, mut c) = three_at_v1(&lab);
        a.bare("Bare", "v2");
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
            synced(&mut c);
        }
        a.bare("Bare", "v1");
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
            synced(&mut c);
        }
        assert_eq!(a.read("Bare/main.lua").as_deref(), Some("v1"));
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("v2"));
        assert_eq!(
            offered_kinds(&a),
            vec![wow_sync_core::decision::DecisionKind::Ambiguous]
        );
    }

    #[test]
    fn more_changes_than_the_lineage_cap_while_a_peer_is_offline_do_not_stall_the_rest() {
        let lab = Lab::new();
        let (mut a, mut b, mut c) = three_at_v1(&lab);
        for i in 1..=36 {
            a.bare("Bare", &format!("e{i}"));
            synced(&mut a);
            synced(&mut b);
            synced(&mut b);
            assert_eq!(b.read("Bare/main.lua"), Some(format!("e{i}")), "change {i}");
            assert!(offered_kinds(&a).is_empty() && offered_kinds(&b).is_empty());
        }
        for _ in 0..4 {
            synced(&mut c);
            synced(&mut a);
            synced(&mut b);
        }
        for pc in [&a, &b, &c] {
            assert_eq!(pc.read("Bare/main.lua").as_deref(), Some("e36"));
            assert!(offered_kinds(pc).is_empty());
        }
    }

    fn write_live(pc: &Pc, content: &str) {
        std::fs::write(pc.install.addons.join("Bare").join("main.lua"), content).unwrap();
    }

    fn with_recent_window(pc: &mut Pc) {
        pc.cfg.recent_ms = 3_600_000;
        pc.restart();
    }

    #[test]
    fn a_live_edit_that_publishes_an_unstable_observation_still_converges_without_a_decision() {
        let lab = Lab::new();
        let (mut a, mut b, mut c) = three_at_v1(&lab);
        with_recent_window(&mut a);
        write_live(&a, "v2");
        synced(&mut a);
        synced(&mut b);
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("v1"));
        a.install.set_old_mtimes();
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
            synced(&mut c);
        }
        for pc in [&a, &b, &c] {
            assert_eq!(pc.read("Bare/main.lua").as_deref(), Some("v2"));
            assert!(offered_kinds(pc).is_empty(), "{:?}", offered_kinds(pc));
        }
    }

    #[test]
    fn no_decision_is_offered_at_any_step_of_a_live_edit_while_it_is_unstable() {
        let lab = Lab::new();
        let (mut a, mut b, _c) = three_at_v1(&lab);
        with_recent_window(&mut a);
        write_live(&a, "v2");
        for _ in 0..3 {
            synced(&mut a);
            assert!(offered_kinds(&a).is_empty(), "A: {:?}", offered_kinds(&a));
            synced(&mut b);
            assert!(offered_kinds(&b).is_empty(), "B: {:?}", offered_kinds(&b));
        }
        a.install.set_old_mtimes();
        for _ in 0..4 {
            synced(&mut a);
            assert!(offered_kinds(&a).is_empty(), "A: {:?}", offered_kinds(&a));
            synced(&mut b);
            assert!(offered_kinds(&b).is_empty(), "B: {:?}", offered_kinds(&b));
        }
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("v2"));
    }

    #[test]
    fn a_live_revert_through_an_unstable_observation_is_not_undone_and_asks_once() {
        let lab = Lab::new();
        let (mut a, mut b, mut c) = three_at_v1(&lab);
        a.bare("Bare", "v2");
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
            synced(&mut c);
        }
        with_recent_window(&mut a);
        write_live(&a, "v1");
        synced(&mut a);
        a.install.set_old_mtimes();
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
            synced(&mut c);
        }
        assert_eq!(a.read("Bare/main.lua").as_deref(), Some("v1"));
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("v2"));
        assert_eq!(
            offered_kinds(&a),
            vec![wow_sync_core::decision::DecisionKind::Ambiguous]
        );
    }

    #[test]
    fn unrecognised_folder_changed_on_both_is_never_overwritten_and_asks() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.bare("Bare", "v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        converge(&mut a, &mut b);
        a.bare("Bare", "from-a");
        b.bare("Bare", "from-b");
        converge(&mut a, &mut b);
        assert_eq!(a.read("Bare/main.lua").as_deref(), Some("from-a"));
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("from-b"));
        for pc in [&a, &b] {
            assert_eq!(
                offered_kinds(pc),
                vec![wow_sync_core::decision::DecisionKind::Ambiguous]
            );
        }
        assert!(!recovery_holds(&a, "from-b") && !recovery_holds(&b, "from-a"));
        let r = synced(&mut b);
        assert!(
            r.status
                .packages
                .iter()
                .any(|p| p.key == "bare" && p.text.contains("Unrecognised: synced by content")),
            "{}",
            texts(&r)
        );
    }

    #[test]
    fn unrecognised_folders_that_differ_at_first_join_are_not_overwritten() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.bare("Bare", "from-a");
        b.bare("Bare", "from-b");
        let g = a.create();
        b.join(&g);
        converge(&mut a, &mut b);
        assert_eq!(a.read("Bare/main.lua").as_deref(), Some("from-a"));
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("from-b"));
        assert_eq!(
            offered_kinds(&b),
            vec![wow_sync_core::decision::DecisionKind::Ambiguous]
        );
        assert!(!recovery_holds(&a, "from-b") && !recovery_holds(&b, "from-a"));
    }

    #[test]
    fn unrecognised_folder_removal_and_restore_round_trip() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.bare("Bare", "v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        converge(&mut a, &mut b);
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("v1"));
        std::fs::remove_dir_all(a.install.addons.join("Bare")).unwrap();
        let st = LocalState::open_at(&a.cfg.state_dir).unwrap();
        let store = RemovalStore::new(&st, g.clone());
        let (base, mut obs) = store.load();
        let key = base.packages.keys().next().expect("a baseline").clone();
        obs.gone.insert(key, (1, 1));
        store.save(&base, &obs).unwrap();
        synced(&mut a);
        let (ac, _) = pc_ids(&a);
        let removal = removal_hash(&lab, &g, &ac);
        converge(&mut a, &mut b);
        assert_eq!(b.read("Bare/main.lua"), None, "the removal propagated");
        let out = a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        assert!(matches!(out, CommandOutcome::Recorded), "{out:?}");
        for _ in 0..8 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(a.read("Bare/main.lua").as_deref(), Some("v1"));
        assert_eq!(b.read("Bare/main.lua").as_deref(), Some("v1"));
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
    fn install_without_marker_enrolls_and_syncs_by_default() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        std::fs::remove_file(a.install.root.join(".wowsync-synthetic")).unwrap();
        a.addon("Alpha", "alpha-v1");
        assert!(!a.svc.status().test_mode);
        assert_eq!(a.svc.inspect().kind, PreviewKind::Recognized);
        let gid = a.create();
        synced(&mut a);
        let mut b = lab.pc("b");
        std::fs::remove_file(b.install.root.join(".wowsync-synthetic")).unwrap();
        b.join(&gid);
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
    }

    #[test]
    fn test_mode_persists_and_blocks_unmarked_installs_both_ways() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v1");
        a.create();
        synced(&mut a);
        assert_eq!(
            a.svc.command(Command::SetTestMode(true)),
            Ok(CommandOutcome::Recorded)
        );
        assert!(a.svc.status().test_mode);
        a.restart();
        assert!(a.svc.status().test_mode, "persisted across restarts");
        synced(&mut a);

        std::fs::remove_file(a.install.root.join(".wowsync-synthetic")).unwrap();
        a.addon("Beta", "beta-v1");
        assert!(matches!(
            a.svc.sync_once(),
            Err(ServiceError::TestModeBlocked)
        ));
        let st = a.svc.status();
        assert!(st.provider_error.is_some() && st.test_mode, "{st:?}");
        assert!(st.test_mode_blocked, "typed flag, not error text: {st:?}");
        assert!(st.enrolled && !st.computer_name.is_empty(), "{st:?}");
        assert!(
            !st.group_name.is_empty() && !st.identity_line.is_empty(),
            "{st:?}"
        );

        assert_eq!(
            a.svc.command(Command::SetTestMode(false)),
            Ok(CommandOutcome::Recorded)
        );
        assert!(!a.svc.status().test_mode);
        assert!(!a.svc.status().test_mode_blocked);
        a.restart();
        synced(&mut a);
        assert_eq!(a.read("Beta/main.lua").as_deref(), Some("beta-v1"));
    }

    #[test]
    fn test_mode_refuses_an_unmarked_install_before_enrollment() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        assert_eq!(
            a.svc.command(Command::SetTestMode(true)),
            Ok(CommandOutcome::Recorded)
        );
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

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            let (src, dst) = (e.path(), to.join(e.file_name()));
            if src.is_dir() {
                copy_dir(&src, &dst);
            } else {
                std::fs::copy(&src, &dst).unwrap();
            }
        }
    }

    #[test]
    fn first_run_join_of_a_populated_install_loses_nothing() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v2");
        a.addon("Gamma", "gamma-v1");
        let eps = "## Interface: 11508\n## Version: 2.0\n";
        a.addon_with("Eps", "eps-a", eps);
        let g = a.create();
        synced(&mut a);

        let mut b = lab.pc("b");
        b.addon("Alpha", "alpha-v1");
        b.addon("Delta", "delta-v1");
        b.addon_with("Eps", "eps-b", eps);
        let before = lab.tmp.join("b-before");
        copy_dir(&b.install.addons, &before);
        b.join(&g);
        for _ in 0..2 {
            synced(&mut b);
            synced(&mut a);
        }
        synced(&mut b);

        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v2"));
        assert_eq!(b.read("Gamma/main.lua").as_deref(), Some("gamma-v1"));
        assert_eq!(b.read("Delta/main.lua").as_deref(), Some("delta-v1"));
        assert_eq!(a.read("Delta/main.lua").as_deref(), Some("delta-v1"));
        assert_eq!(b.read("Eps/main.lua").as_deref(), Some("eps-b"));
        assert_eq!(a.read("Eps/main.lua").as_deref(), Some("eps-a"));

        let st = b.svc.status();
        let row = st
            .recovery_points
            .iter()
            .find(|r| r.package.as_str() == "alpha")
            .expect("the replaced older Alpha is kept for the GUI");
        assert_eq!(
            b.svc.command(Command::Rollback {
                package: row.package.clone(),
                recovered: row.release,
            }),
            Ok(CommandOutcome::Recorded)
        );
        synced(&mut b);
        assert!(
            same_tree(&b.install.addons.join("Alpha"), &before.join("Alpha")),
            "restored byte for byte"
        );
        for kept in ["Delta", "Eps"] {
            assert!(same_tree(&b.install.addons.join(kept), &before.join(kept)));
        }
    }

    /// A and B converge on alpha-v1 after B rolled back the group-rejected alpha-v2; then A's
    /// owner edits Alpha into a newer, never-considered copy.
    fn rolled_back_then_edited_newer(lab: &Lab) -> (Pc, Pc) {
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v2");
        let g = a.create();
        synced(&mut a);
        let mut b = lab.pc("b");
        b.addon("Alpha", "alpha-v1");
        b.join(&g);
        for _ in 0..2 {
            synced(&mut b);
            synced(&mut a);
        }
        let row = b
            .svc
            .status()
            .recovery_points
            .into_iter()
            .find(|r| r.package.as_str() == "alpha")
            .expect("the replaced older Alpha is kept");
        assert_eq!(
            b.svc.command(Command::Rollback {
                package: row.package.clone(),
                recovered: row.release,
            }),
            Ok(CommandOutcome::Recorded)
        );
        for _ in 0..3 {
            synced(&mut b);
            synced(&mut a);
        }
        for pc in [&a, &b] {
            assert_eq!(pc.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        }
        a.addon("Alpha", "alpha-newer-v3");
        (a, b)
    }

    fn answer_rollback_notice(a: &mut Pc, use_newer: bool) {
        use wow_sync_core::decision::{DecisionId, OptionLabel};
        let item = a
            .svc
            .offered_decisions()
            .iter()
            .find(|i| i.reason == wow_sync_core::decision::DecisionKind::NewerAfterRollback)
            .expect("the newer-after-rollback notice is offered")
            .clone();
        let option = item
            .options
            .iter()
            .find(|o| matches!(o.label, OptionLabel::Release { .. }) == use_newer)
            .unwrap()
            .id;
        a.svc
            .command(Command::Decide {
                item: DecisionId(item.scope.clone()),
                option,
                shown: item.evidence,
            })
            .unwrap();
    }

    #[test]
    fn a_newer_local_edit_after_a_rollback_is_held_and_offered_not_replaced() {
        use wow_sync_core::decision::DecisionKind;
        let lab = Lab::new();
        let (mut a, mut b) = rolled_back_then_edited_newer(&lab);
        for _ in 0..3 {
            synced(&mut a);
            assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-newer-v3"));
            assert_eq!(offered_kinds(&a), vec![DecisionKind::NewerAfterRollback]);
            assert_eq!(a.svc.status().local_only_versions.len(), 1);
            synced(&mut b);
            assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        }
    }

    #[test]
    fn using_the_newer_version_after_a_rollback_ends_it_and_converges_the_group() {
        let lab = Lab::new();
        let (mut a, mut b) = rolled_back_then_edited_newer(&lab);
        synced(&mut a);
        answer_rollback_notice(&mut a, true);
        for _ in 0..3 {
            synced(&mut a);
            synced(&mut b);
        }
        for pc in [&a, &b] {
            assert_eq!(pc.read("Alpha/main.lua").as_deref(), Some("alpha-newer-v3"));
        }
        assert!(offered_kinds(&a).is_empty(), "{:?}", offered_kinds(&a));
        assert!(a.svc.status().local_only_versions.is_empty());
    }

    #[test]
    fn staying_after_a_rollback_keeps_the_rollback_and_recovers_the_newer_copy() {
        let lab = Lab::new();
        let (mut a, mut b) = rolled_back_then_edited_newer(&lab);
        synced(&mut a);
        answer_rollback_notice(&mut a, false);
        for _ in 0..3 {
            synced(&mut a);
            synced(&mut b);
        }
        for pc in [&a, &b] {
            assert_eq!(pc.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        }
        assert!(offered_kinds(&a).is_empty(), "{:?}", offered_kinds(&a));
        let rec = a.install.variant_dir.join(".wowsync").join("recovery");
        assert!(
            has_file(&rec, "main.lua", b"alpha-newer-v3"),
            "the newer copy is kept for recovery"
        );
    }

    #[test]
    fn first_run_join_holding_a_group_rejected_release_does_not_resurrect_it() {
        use wow_sync_core::hash::package_hash;
        let lab = Lab::new();
        let (mut a, mut b, mut c, g) = alpha_v2_published(&lab);
        let v1 = alpha_folder(&lab, &g, &a);
        let v1_hash = package_hash(&[(v1.key.clone(), v1.tree.unwrap())]);
        synced(&mut b);
        assert_eq!(
            b.svc.command(Command::Rollback {
                package: v1.key.clone(),
                recovered: v1_hash,
            }),
            Ok(CommandOutcome::Recorded)
        );
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));

        let mut d = lab.pc("d");
        d.addon("Alpha", "alpha-v2");
        d.addon("Delta", "delta-v1");
        d.join(&g);
        for _ in 0..3 {
            synced(&mut d);
            synced(&mut a);
            synced(&mut b);
            synced(&mut c);
        }
        for pc in [&a, &b, &c, &d] {
            assert_eq!(pc.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        }
        assert_eq!(a.read("Delta/main.lua").as_deref(), Some("delta-v1"));
        let rec = d.install.variant_dir.join(".wowsync").join("recovery");
        let kept = std::fs::read_dir(&rec)
            .map(|r| r.flatten().map(|e| e.path()).collect::<Vec<_>>())
            .unwrap_or_default();
        assert!(
            kept.iter().any(|p| has_file(p, "main.lua", b"alpha-v2")),
            "the joiner's rejected copy is kept for recovery: {kept:?}"
        );
    }

    #[test]
    fn unreadable_unrelated_toc_does_not_freeze_rollback_or_later_release() {
        let lab = Lab::new();
        let big = format!(
            "## Interface: 11508\n## Version: 1.0\n## X-Pad: {}\n",
            "a".repeat(300 * 1024)
        );
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v2");
        a.addon_with("Big", "big", &big);
        let g = a.create();
        synced(&mut a);
        let mut b = lab.pc("b");
        b.addon("Alpha", "alpha-v1");
        b.addon_with("Big", "big", &big);
        let before = lab.tmp.join("b-before");
        copy_dir(&b.install.addons, &before);
        b.join(&g);
        for _ in 0..2 {
            synced(&mut b);
            synced(&mut a);
        }
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v2"));

        let row = b
            .svc
            .status()
            .recovery_points
            .into_iter()
            .find(|r| r.package.as_str() == "alpha")
            .expect("the replaced older Alpha is kept");
        assert_eq!(
            b.svc.command(Command::Rollback {
                package: row.package.clone(),
                recovered: row.release,
            }),
            Ok(CommandOutcome::Recorded)
        );
        for _ in 0..3 {
            synced(&mut b);
            synced(&mut a);
        }
        for pc in [&a, &b] {
            assert_eq!(pc.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        }

        a.addon("Fishing", "fish-v1");
        let r = synced(&mut a);
        assert!(r.published.is_some(), "{r:?} / {}", texts(&r));
        for _ in 0..2 {
            synced(&mut b);
            synced(&mut a);
        }
        assert_eq!(b.read("Fishing/main.lua").as_deref(), Some("fish-v1"));
        for pc in [&a, &b] {
            assert_eq!(pc.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        }
        assert!(same_tree(
            &b.install.addons.join("Big"),
            &before.join("Big")
        ));
    }

    fn has_file(dir: &Path, name: &str, content: &[u8]) -> bool {
        std::fs::read_dir(dir).is_ok_and(|r| {
            r.flatten().any(|e| {
                let p = e.path();
                if p.is_dir() {
                    has_file(&p, name, content)
                } else {
                    p.file_name().is_some_and(|n| n == name)
                        && std::fs::read(&p).is_ok_and(|b| b == content)
                }
            })
        })
    }

    #[test]
    fn first_run_create_from_a_populated_install_changes_nothing_locally() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v1");
        a.addon("Beta", "beta-v1");
        let before = lab.tmp.join("a-before");
        copy_dir(&a.install.addons, &before);
        let g = a.create();
        for _ in 0..3 {
            let r = synced(&mut a);
            assert!(r.applied.is_empty(), "{r:?}");
        }
        assert!(same_tree(&a.install.addons, &before));
        assert!(a.svc.status().recovery_points.is_empty());
        let mut b = lab.pc("b");
        b.join(&g);
        synced(&mut b);
        assert!(same_tree(&a.install.addons, &b.install.addons));
    }

    #[test]
    fn first_run_join_with_a_group_removed_addon_keeps_it_restorable() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let g = a.create();
        a_removes_alpha(&mut a, &g);
        let (ac, _) = pc_ids(&a);
        let removal = removal_hash(&lab, &g, &ac);

        let mut b = lab.pc("b");
        b.addon("Alpha", "alpha-v1");
        b.addon("Delta", "delta-v1");
        let before = lab.tmp.join("b-before");
        copy_dir(&b.install.addons, &before);
        b.join(&g);
        for _ in 0..3 {
            synced(&mut b);
            synced(&mut a);
        }
        assert!(same_tree(
            &b.install.addons.join("Delta"),
            &before.join("Delta")
        ));
        assert_eq!(a.read("Delta/main.lua").as_deref(), Some("delta-v1"));
        // A copy the joiner already had is not deleted by a removal that predates it.
        assert!(same_tree(
            &b.install.addons.join("Alpha"),
            &before.join("Alpha")
        ));
        assert_eq!(b.svc.status().removal_hashes, vec![removal]);
    }

    #[test]
    fn forever_suffix_folders_sync_by_the_forever_rule_and_unrecognised_ones_by_content() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let toc = |v: &str| format!("## Interface: 11508\n## Version: {v}\n");
        a.install.add_addon(
            "Suf",
            "Suf.toc",
            &toc("1.0"),
            &[
                ("Suf_Forever.toc", toc("1.0").as_bytes()),
                ("Suf_Vanilla.toc", toc("9.0").as_bytes()),
                ("main.lua", b"suf"),
            ],
        );
        a.install.add_addon(
            "OnlyVanilla",
            "OnlyVanilla_Vanilla.toc",
            &toc("1.0"),
            &[("main.lua", b"vanilla-secret")],
        );
        a.install.set_old_mtimes();
        let g = a.create();
        synced(&mut a);
        let mut b = lab.pc("b");
        b.join(&g);
        synced(&mut b);
        assert!(same_tree(
            &a.install.addons.join("Suf"),
            &b.install.addons.join("Suf")
        ));
        assert!(
            same_tree(
                &a.install.addons.join("OnlyVanilla"),
                &b.install.addons.join("OnlyVanilla")
            ),
            "a folder with only other-version .toc files is unrecognised and syncs by content"
        );
        let rel = layout::blob(&g, &sha256(b"vanilla-secret"));
        let p: PathBuf = rel
            .split('/')
            .fold(lab.shared.join("WoWSync"), |x, s| x.join(s));
        assert!(p.exists(), "its content is uploaded (D-29)");
    }

    #[test]
    fn enrolled_forever_with_the_old_plain_mapping_is_reconfirmed_and_loses_nothing() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v1");
        a.create();
        synced(&mut a);
        let st = LocalState::open_at(&a.cfg.state_dir).unwrap();
        let bytes = st.read("enrollment.json", 65536).unwrap().unwrap();
        let mut le: wow_sync_core::records::LocalEnrollment = decode(&bytes, 65536).unwrap();
        le.validation.mapping_id = "validated-plain/1".into();
        st.write_atomic("enrollment.json", &wow_sync_core::records::encode(&le))
            .unwrap();
        a.restart();
        assert!(matches!(
            a.svc.sync_once(),
            Err(ServiceError::NeedsReconfirmation)
        ));
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        let offered = a.svc.status().reconfirm.expect("reconfirmation is offered");
        assert!(a
            .svc
            .reconfirm(&ReconfirmRequest {
                identity_confirmed: false,
                scope_confirmed: true,
                digest: offered.digest,
            })
            .is_err());
        a.svc
            .reconfirm(&ReconfirmRequest {
                identity_confirmed: true,
                scope_confirmed: true,
                digest: offered.digest,
            })
            .unwrap();
        a.addon("Beta", "beta-v1");
        let r = synced(&mut a);
        assert!(r.published.is_some(), "{r:?}");
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
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

    fn folder_of(
        lab: &Lab,
        g: &GroupId,
        pc: &Pc,
        key: &str,
    ) -> wow_sync_core::inventory::FolderState {
        latest_state(lab, g, pc)
            .folders
            .into_iter()
            .find(|f| f.key.as_str() == key)
            .expect("the folder is published")
    }

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

    #[test]
    fn a_rollback_whose_files_vanish_after_it_is_queued_reports_not_available() {
        use wow_sync_core::hash::package_hash;
        use wow_sync_core::records::decode_tree;
        let lab = Lab::new();
        let (_a, mut b, c, g) = alpha_v2_published(&lab);
        let v1 = alpha_folder(&lab, &g, &c);
        let tree = v1.tree.unwrap();
        let v1_hash = package_hash(&[(v1.key.clone(), tree)]);
        let rec = std::fs::read(provider_file(&lab, &layout::tree_record(&g, &tree))).unwrap();
        let files = decode_tree(&rec, &tree).unwrap().files;
        synced(&mut b);
        assert_eq!(
            b.svc.command(Command::Rollback {
                package: v1.key.clone(),
                recovered: v1_hash,
            }),
            Ok(CommandOutcome::Recorded)
        );
        for f in &files {
            std::fs::remove_file(provider_file(&lab, &layout::blob(&g, &f.sha256))).unwrap();
        }
        synced(&mut b);
        assert_eq!(
            b.read("Alpha/main.lua").as_deref(),
            Some("alpha-v2"),
            "nothing changed"
        );
        let out = b.svc.status().rollback_outcome.expect("a visible outcome");
        assert_eq!(
            out.kind,
            crate::rollback_flow::RollbackOutcomeKind::NotAvailable
        );
        assert_eq!(out.recovered, v1_hash);
        // Asking again clears the old outcome.
        assert!(b
            .svc
            .command(Command::Rollback {
                package: v1.key.clone(),
                recovered: v1_hash,
            })
            .is_err());
    }

    #[test]
    fn a_rollback_with_its_files_present_applies_and_leaves_no_outcome() {
        use wow_sync_core::hash::package_hash;
        let lab = Lab::new();
        let (_a, mut b, c, g) = alpha_v2_published(&lab);
        let v1 = alpha_folder(&lab, &g, &c);
        let v1_hash = package_hash(&[(v1.key.clone(), v1.tree.unwrap())]);
        synced(&mut b);
        assert_eq!(
            b.svc.command(Command::Rollback {
                package: v1.key.clone(),
                recovered: v1_hash,
            }),
            Ok(CommandOutcome::Recorded)
        );
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert_eq!(b.svc.status().rollback_outcome, None);
    }

    #[test]
    fn a_rollback_whose_files_left_the_shared_folder_is_refused_not_silently_dropped() {
        use wow_sync_core::hash::package_hash;
        use wow_sync_core::records::decode_tree;
        let lab = Lab::new();
        let (_a, mut b, c, g) = alpha_v2_published(&lab);
        let v1 = alpha_folder(&lab, &g, &c);
        let tree = v1.tree.unwrap();
        let v1_hash = package_hash(&[(v1.key.clone(), tree)]);
        let rec = std::fs::read(provider_file(&lab, &layout::tree_record(&g, &tree))).unwrap();
        let files = decode_tree(&rec, &tree).unwrap().files;
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v2"));
        for f in &files {
            std::fs::remove_file(provider_file(&lab, &layout::blob(&g, &f.sha256))).unwrap();
        }
        // The files are already gone: refused at once, nothing queued.
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
        assert_eq!(b.svc.status().rollback_outcome, None);
    }

    #[test]
    fn a_release_only_a_local_recovery_point_holds_is_listed_and_can_be_restored() {
        use wow_sync_core::hash::package_hash;
        use wow_sync_core::records::decode_tree;
        let lab = Lab::new();
        let (mut a, mut b, mut c, g) = alpha_v2_published(&lab);
        let v1 = alpha_folder(&lab, &g, &c);
        let tree = v1.tree.unwrap();
        let v1_hash = package_hash(&[(v1.key.clone(), tree)]);
        // Everyone follows B to v2, so no computer publishes v1 any more; A and C hold it only
        // in a local recovery point.
        for _ in 0..2 {
            synced(&mut a);
            synced(&mut c);
        }
        synced(&mut b);
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v2"));
        let rec_rel = layout::tree_record(&g, &tree);
        let rec = std::fs::read(provider_file(&lab, &rec_rel)).unwrap();
        for f in decode_tree(&rec, &tree).unwrap().files {
            std::fs::remove_file(provider_file(&lab, &layout::blob(&g, &f.sha256))).unwrap();
        }
        std::fs::remove_file(provider_file(&lab, &rec_rel)).unwrap();

        let st = a.svc.status();
        let row = st
            .recovery_points
            .iter()
            .find(|r| r.release == v1_hash)
            .expect("the displaced v1 is listed");
        assert_eq!(row.package, v1.key);
        assert!(row.set_aside_unix_ms > 0 && !row.reason.is_empty() && !row.id.is_empty());
        assert_eq!(
            st.held_release_dates.get(&v1_hash),
            Some(&row.set_aside_unix_ms)
        );

        // Negative: B changed to v2 by hand, so it holds nothing and lists nothing.
        assert!(b.svc.status().recovery_points.is_empty());
        assert_eq!(
            b.svc.command(Command::Rollback {
                package: v1.key.clone(),
                recovered: v1_hash,
            }),
            Err(CommandError::UnknownCopy(v1_hash))
        );

        assert_eq!(
            a.svc.command(Command::Rollback {
                package: v1.key.clone(),
                recovered: row.release,
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
        let snap = b.svc.status();
        assert_eq!(snap.removed.len(), 1);
        assert_eq!(
            snap.removal_hashes,
            vec![removal],
            "the snapshot carries the hash RestoreRemoved takes"
        );

        let ghost = sha256(b"no such removal");
        assert_eq!(
            b.svc.command(Command::RestoreRemoved { removal: ghost }),
            Err(CommandError::UnknownRemoval(ghost))
        );
        let (bc, _) = pc_ids(&b);
        assert!(authority_files(&lab, &g, &bc).iter().all(|f| !f.is_empty()));
        let before_files = authority_files(&lab, &g, &bc);
        let out = b.svc.command(Command::RestoreRemoved { removal }).unwrap();
        assert!(matches!(out, CommandOutcome::Recorded), "{out:?}");
        assert_eq!(
            authority_files(&lab, &g, &bc),
            before_files,
            "the re-add is written only after the code is back"
        );
        for _ in 0..3 {
            synced(&mut b);
            synced(&mut c);
            synced(&mut a);
        }
        let after_files = authority_files(&lab, &g, &bc);
        let new: Vec<&String> = after_files
            .iter()
            .filter(|f| !before_files.contains(f))
            .filter(|f| {
                let h = Sha256::parse(f.trim_end_matches(".json")).unwrap();
                let bytes =
                    std::fs::read(provider_file(&lab, &layout::authority_record(&g, &bc, &h)))
                        .unwrap();
                matches!(decode_authority(&bytes, &h), Ok(AuthorityRecord::Readd(_)))
            })
            .collect();
        assert_eq!(new.len(), 1, "{new:?}");
        let readd = Sha256::parse(new[0].trim_end_matches(".json")).unwrap();
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

    const SV_TOC: &str = "## Interface: 11508\n## Version: 1.0\n## SavedVariables: AlphaDB\n## SavedVariablesPerCharacter: AlphaChar\n";

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
        // WoW creates the account folder at first login; the app never does.
        for pc in [&a, &b] {
            std::fs::create_dir_all(pc.install.variant_dir.join("WTF/Account/ACC")).unwrap();
        }
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

    #[test]
    fn a_character_missing_here_waits_and_is_never_created_and_a_big_unit_says_so() {
        use crate::settings_cycle::SettingsUnitStatus as S;
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        let rel = "WTF/Account/ACC/Realm1/CharA1/SavedVariables/Alpha.lua";
        let on_a = a.install.variant_dir.join(rel);
        std::fs::create_dir_all(on_a.parent().unwrap()).unwrap();
        std::fs::write(&on_a, "AlphaChar = { v = 1 }\n").unwrap();
        synced(&mut a);
        std::fs::write(&on_a, "AlphaChar = { v = 2 }\n").unwrap();
        synced(&mut a);
        synced(&mut b);
        synced(&mut b);
        let realm = b.install.variant_dir.join("WTF/Account/ACC/Realm1");
        assert!(!realm.exists(), "a missing character is never created");
        let waiting = b
            .svc
            .status()
            .settings_units
            .into_iter()
            .find(|r| r.status == S::WaitingForIdentity)
            .expect("waiting row");
        assert!(waiting.waiting_for.is_some());

        // Over the cap: the unit is named with its size and nothing is published.
        a.svc.set_settings_cap_for_tests(Some(8));
        write_wtf(&a, "Alpha", "AlphaDB = { a_long_value = 1 }\n");
        synced(&mut a);
        let big = a
            .svc
            .status()
            .settings_units
            .into_iter()
            .find(|r| r.status == S::TooLarge)
            .expect("too large row");
        assert!(big.size_bytes.is_some());
    }

    #[test]
    fn with_the_game_running_settings_wait_and_nothing_is_applied() {
        use crate::settings_cycle::SettingsUnitStatus as S;
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        b.act.set(Activity::Running { pids: vec![4242] });
        synced(&mut b);
        assert_eq!(read_wtf(&b, "Alpha"), None);
        assert!(
            b.svc
                .status()
                .settings_units
                .iter()
                .all(|r| r.status == S::WaitingForWow),
            "{:?}",
            b.svc.status().settings_units
        );
        b.act.set(Activity::NotRunning);
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 2 }\n")
        );
    }

    #[test]
    fn a_variable_declared_by_two_addons_is_owned_by_several_and_publishes_nothing() {
        use crate::settings_cycle::SettingsUnitStatus as S;
        let lab = Lab::new();
        let (mut a, _b, _g) = settings_pair(&lab, true, Some(1 << 20));
        a.addon_with("Gamma", "gamma-v1", SV_TOC);
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        write_wtf(&a, "Gamma", "AlphaDB = { v = 9 }\n");
        synced(&mut a);
        synced(&mut a);
        let rows = a.svc.status().settings_units;
        let several: Vec<_> = rows
            .iter()
            .filter(|r| r.status == S::OwnedBySeveral)
            .collect();
        assert!(!several.is_empty(), "{rows:?}");
        assert!(
            several
                .iter()
                .any(|r| r.shared_with.iter().any(|n| n == "Gamma")),
            "{several:?}"
        );
        assert!(a.svc.settings_history(&alpha_unit()).is_empty());
    }

    #[test]
    fn settings_progress_counts_units_names_the_addon_and_the_origin_when_applying() {
        use crate::history::Phase;
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        let rx_a = a.svc.progress_hub().subscribe();
        let rx_b = b.svc.progress_hub().subscribe();
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        let on_a: Vec<_> = rx_a
            .try_iter()
            .filter(|e| e.phase == Phase::Settings && e.completed.is_none())
            .collect();
        assert!(on_a.iter().all(|e| e.source.is_none()), "{on_a:?}");
        let on_b: Vec<_> = rx_b
            .try_iter()
            .filter(|e| e.phase == Phase::Settings)
            .collect();
        assert!(
            on_b.iter().any(|e| e.current.as_deref() == Some("Alpha")
                && e.source == Some(ChangeSource::Computer("a".into()))),
            "{on_b:?}"
        );
    }

    fn alpha_unit() -> wow_sync_core::ids::SettingsUnit {
        wow_sync_core::ids::SettingsUnit {
            folder: wow_sync_core::paths::validate_folder_name("Alpha").unwrap(),
            identity: wow_sync_core::ids::Identity::account("ACC").unwrap(),
        }
    }

    // >>> R5-remove: uninstall and restore of settings
    fn uninstall_alpha_on(a: &mut Pc, g: &GroupId) {
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

    fn settings_removal_ready(cap: u64) -> (Lab, Pc, Pc, GroupId) {
        let lab = Lab::new();
        let (mut a, b, g) = settings_pair(&lab, true, Some(cap));
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        (lab, a, b, g)
    }

    #[test]
    fn an_uninstall_removes_the_settings_with_the_code_and_a_restore_returns_both() {
        let (lab, mut a, mut b, g) = settings_removal_ready(1 << 20);
        write_wtf(&b, "Alpha", "AlphaDB = { peer = true }\n");
        synced(&mut b);
        uninstall_alpha_on(&mut a, &g);
        assert_eq!(
            read_wtf(&a, "Alpha"),
            None,
            "the settings left with the code"
        );
        let (ac, _) = pc_ids(&a);
        let removal = removal_hash(&lab, &g, &ac);
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua"), None);
        assert_eq!(read_wtf(&b, "Alpha"), None, "the peer's settings left too");

        let out = a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        assert!(matches!(out, CommandOutcome::Recorded), "{out:?}");
        for _ in 0..8 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert_eq!(
            read_wtf(&a, "Alpha").as_deref(),
            Some("AlphaDB = { v = 1 }\n"),
            "the captured settings came back byte for byte"
        );
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 1 }\n"),
            "the peer restores what it held at the uninstall: the master's shared copy, which \
             replaced its own routine edit"
        );
        synced(&mut a);
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert!(read_wtf(&a, "Alpha").is_some(), "not removed again");
    }

    /// H2: the add-on's file was created by the add-on, not by this app, and settings sync was
    /// never turned on. The uninstall still removes the owned file everywhere and a restore
    /// returns code and settings together.
    #[test]
    fn uninstall_removes_and_restores_settings_without_settings_sync() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon_with("Alpha", "alpha-v1", SV_TOC);
        b.addon_with("Alpha", "alpha-v1", SV_TOC);
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        let (a_text, b_text) = ("AlphaDB = { a = 1 }\n", "AlphaDB = { b = 2 }\n");
        write_wtf(&a, "Alpha", a_text);
        write_wtf(&b, "Alpha", b_text);
        let own_other = wtf_file(&a, "Unrelated");
        std::fs::write(&own_other, "UnrelatedDB = {}\n").unwrap();
        synced(&mut a);
        synced(&mut b);
        assert_eq!(read_wtf(&a, "Alpha").as_deref(), Some(a_text));

        uninstall_alpha_on(&mut a, &g);
        assert_eq!(read_wtf(&a, "Alpha"), None, "the originating computer");
        let (ac, _) = pc_ids(&a);
        let removal = removal_hash(&lab, &g, &ac);
        for _ in 0..3 {
            synced(&mut b);
            synced(&mut a);
        }
        assert_eq!(b.read("Alpha/main.lua"), None);
        assert_eq!(read_wtf(&b, "Alpha"), None, "the receiving computer");
        assert!(own_other.exists(), "an unowned file is never removed");

        let out = a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        assert!(matches!(out, CommandOutcome::Recorded), "{out:?}");
        for _ in 0..8 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert_eq!(read_wtf(&a, "Alpha").as_deref(), Some(a_text));
        assert_eq!(read_wtf(&b, "Alpha").as_deref(), Some(b_text));
    }

    #[test]
    fn a_settings_file_over_the_cap_is_not_moved_by_an_uninstall() {
        let big = "AlphaDB = { v = 1 }\n";
        let (_lab, mut a, _b, g) = settings_removal_ready(big.len() as u64 - 1);
        uninstall_alpha_on(&mut a, &g);
        assert_eq!(
            read_wtf(&a, "Alpha").as_deref(),
            Some(big),
            "a file the cap refuses stays where it is"
        );
    }

    #[test]
    fn a_file_exactly_at_the_cap_is_captured_by_an_uninstall() {
        let text = "AlphaDB = { v = 1 }\n";
        let (_lab, mut a, _b, g) = settings_removal_ready(text.len() as u64);
        uninstall_alpha_on(&mut a, &g);
        assert_eq!(read_wtf(&a, "Alpha"), None);
    }
    fn removed_manifest(pc: &Pc, removal: &Sha256) -> Vec<wow_sync_core::kinds::ManifestFolder> {
        match pc.svc.plane().ledger.record(removal) {
            Some(AuthorityRecord::Removal(r)) => r.folders.clone(),
            other => panic!("not a removal: {other:?}"),
        }
    }

    fn delete_removed_blobs(lab: &Lab, g: &GroupId, m: &[wow_sync_core::kinds::ManifestFolder]) {
        for f in m {
            let p = provider_file(lab, &layout::tree_record(g, &f.tree));
            let rec =
                wow_sync_core::records::decode_tree(&std::fs::read(p).unwrap(), &f.tree).unwrap();
            for file in rec.files {
                let _ = std::fs::remove_file(provider_file(lab, &layout::blob(g, &file.sha256)));
            }
        }
    }

    /// An uninstalled Alpha on both peers with no computer holding the code any more.
    fn uninstalled_everywhere() -> (Lab, Pc, Pc, GroupId, Sha256) {
        let (lab, mut a, mut b, g) = settings_removal_ready(1 << 20);
        write_wtf(&b, "Alpha", "AlphaDB = { peer = true }\n");
        synced(&mut b);
        uninstall_alpha_on(&mut a, &g);
        let (ac, _) = pc_ids(&a);
        let removal = removal_hash(&lab, &g, &ac);
        for _ in 0..3 {
            synced(&mut b);
            synced(&mut a);
        }
        assert_eq!(b.read("Alpha/main.lua"), None);
        (lab, a, b, g, removal)
    }

    #[test]
    fn a_restore_without_a_recovery_point_installs_the_code_from_the_shared_folder() {
        let (_lab, mut a, _b, _g, removal) = uninstalled_everywhere();
        // a deleted the folder by hand: nothing local holds the removed release
        assert!(
            crate::effects::local_points(&a.cfg.install_root.join(&a.cfg.variant_folder))
                .is_empty(),
            "no recovery point on the originating computer"
        );
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        synced(&mut a);
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
    }

    #[test]
    fn a_restore_with_a_missing_blob_is_unavailable_and_leaves_nothing_partial() {
        let (lab, mut a, _b, g, removal) = uninstalled_everywhere();
        delete_removed_blobs(&lab, &g, &removed_manifest(&a, &removal));
        let err = a
            .svc
            .command(Command::RestoreRemoved { removal })
            .unwrap_err();
        assert!(
            matches!(&err, CommandError::Service(t) if t.contains("unavailable")),
            "{err:?}"
        );
        for _ in 0..3 {
            synced(&mut a);
        }
        assert_eq!(a.read("Alpha/main.lua"), None);
        assert!(!a.install.addons.join("Alpha").exists(), "nothing partial");
        assert_eq!(a.journals(), 0);
    }

    #[test]
    fn a_restore_waits_while_wow_runs_and_installs_when_it_closes() {
        let (_lab, mut a, _b, _g, removal) = uninstalled_everywhere();
        a.act.set(Activity::Running { pids: vec![4242] });
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        for _ in 0..2 {
            synced(&mut a);
        }
        assert!(!a.install.addons.join("Alpha").exists(), "blocked by WoW");
        a.act.set(Activity::NotRunning);
        synced(&mut a);
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
    }

    #[test]
    fn a_restore_of_a_rejected_version_is_refused() {
        use wow_sync_core::choice::ChoiceView;
        use wow_sync_core::hash::package_hash;
        use wow_sync_core::kinds::{
            RecoveredEntry, RejectedEntry, Rejection, RejectionSettings, TreeEntry,
        };
        use wow_sync_core::ledger::Ledger;
        let (lab, mut a, _b, g, removal) = uninstalled_everywhere();
        let manifest = removed_manifest(&a, &removal);
        let entries: Vec<TreeEntry> = manifest
            .iter()
            .map(|m| TreeEntry {
                key: m.key.clone(),
                tree: m.tree,
            })
            .collect();
        let other = vec![TreeEntry {
            key: manifest[0].key.clone(),
            tree: sha256(b"another release"),
        }];
        let pairs =
            |e: &[TreeEntry]| -> Vec<_> { e.iter().map(|t| (t.key.clone(), t.tree)).collect() };
        let ledger = Ledger::new();
        let view = ChoiceView::new(&ledger, &Default::default());
        let (ac, ae) = pc_ids(&a);
        let rej = Rejection {
            group_id: g.clone(),
            issuer: issuer_of(&a),
            folders: vec![manifest[0].key.clone()],
            rejected: RejectedEntry {
                package_hash: package_hash(&pairs(&entries)),
                version_label: "1.0".into(),
                folders: entries.clone(),
            },
            recovered: RecoveredEntry {
                package_hash: package_hash(&pairs(&other)),
                folders: other.clone(),
            },
            considered: {
                let mut v = vec![
                    wow_sync_core::kinds::Considered {
                        package_hash: package_hash(&pairs(&entries)),
                        folders: entries.clone(),
                        holders: vec![ae.clone()],
                    },
                    wow_sync_core::kinds::Considered {
                        package_hash: package_hash(&pairs(&other)),
                        folders: other,
                        holders: vec![ae],
                    },
                ];
                v.sort();
                v
            },
            settings: RejectionSettings::NotApplicable,
            known: view.known_for(std::slice::from_ref(&manifest[0].key)),
            considered_parts: vec![],
            written_unix_ms: 5,
        };
        let rh = put_authority(&lab, &g, &ac, &AuthorityRecord::Rejection(rej));
        synced(&mut a);
        assert_eq!(status(&a, &g, &rh), Some(Status::Valid));
        let err = a
            .svc
            .command(Command::RestoreRemoved { removal })
            .unwrap_err();
        assert!(
            matches!(&err, CommandError::Service(t) if t.contains("rejected")),
            "{err:?}"
        );
        synced(&mut a);
        assert_eq!(a.read("Alpha/main.lua"), None);
    }

    #[test]
    fn a_restore_with_no_settings_copy_warns_that_settings_were_not_restored() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        uninstall_alpha_on(&mut a, &g);
        let (ac, _) = pc_ids(&a);
        let removal = removal_hash(&lab, &g, &ac);
        for _ in 0..3 {
            synced(&mut b);
            synced(&mut a);
        }
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        let r = synced(&mut a);
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert_eq!(
            r.apply_note.as_deref(),
            Some("Settings were not restored: no saved copy from before the removal")
        );
    }

    #[test]
    fn a_restore_from_the_local_recovery_point_works_without_the_shared_files() {
        let (lab, mut a, mut b, g, removal) = uninstalled_everywhere();
        delete_removed_blobs(&lab, &g, &removed_manifest(&b, &removal));
        b.svc.command(Command::RestoreRemoved { removal }).unwrap();
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        let _ = &mut a;
    }

    fn restores_of(pc: &mut Pc) -> Vec<crate::restore_flow::RestoreProgress> {
        pc.svc.status().restores
    }

    #[test]
    fn a_queued_restore_is_shown_as_waiting_for_wow_then_for_the_next_sync_then_vanishes() {
        use crate::restore_flow::{RestoreState, RestoreWait};
        let (_lab, mut a, _b, _g, removal) = uninstalled_everywhere();
        assert!(restores_of(&mut a).is_empty());
        a.act.set(Activity::Running { pids: vec![4242] });
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        let shown = restores_of(&mut a);
        assert_eq!(shown.len(), 1);
        assert_eq!(
            shown[0].state,
            RestoreState::Waiting {
                reason: RestoreWait::WowRunning
            }
        );
        assert_eq!(
            shown[0].text,
            "Restoring alpha: waiting for World of Warcraft to close."
        );
        a.act.set(Activity::NotRunning);
        assert_eq!(
            restores_of(&mut a)[0].state,
            RestoreState::Waiting {
                reason: RestoreWait::NextCycle
            }
        );
        synced(&mut a);
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert!(
            restores_of(&mut a).is_empty(),
            "gone once the re-add is written"
        );
    }

    #[test]
    fn a_queued_restore_is_shown_as_waiting_while_syncing_is_paused() {
        use crate::restore_flow::{RestoreState, RestoreWait};
        let (_lab, mut a, _b, _g, removal) = uninstalled_everywhere();
        a.act.set(Activity::Running { pids: vec![4242] });
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        a.act.set(Activity::NotRunning);
        a.svc.set_paused(true).unwrap();
        let shown = restores_of(&mut a);
        assert_eq!(
            shown[0].state,
            RestoreState::Waiting {
                reason: RestoreWait::Paused
            }
        );
        assert_eq!(
            shown[0].text,
            "Restoring alpha: waiting for syncing to be resumed."
        );
    }

    #[test]
    fn a_restore_refused_at_cycle_time_stays_listed_until_dismissed_or_retried() {
        use crate::restore_flow::RestoreState;
        let (lab, mut a, _b, g, removal) = uninstalled_everywhere();
        a.act.set(Activity::Running { pids: vec![4242] });
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        delete_removed_blobs(&lab, &g, &removed_manifest(&a, &removal));
        a.act.set(Activity::NotRunning);
        synced(&mut a);
        let shown = restores_of(&mut a);
        assert_eq!(shown.len(), 1);
        assert!(matches!(shown[0].state, RestoreState::Unavailable { .. }));
        assert!(
            shown[0].text.starts_with("alpha can't be restored: ")
                && shown[0].text.ends_with(". Nothing was changed."),
            "{}",
            shown[0].text
        );
        synced(&mut a);
        assert_eq!(restores_of(&mut a).len(), 1, "stays across cycles");
        assert!(!a.install.addons.join("Alpha").exists());
        let package = shown[0].package.clone();
        assert_eq!(
            a.svc
                .command(Command::DismissRestoreIssue {
                    package: package.clone()
                })
                .unwrap(),
            CommandOutcome::Recorded
        );
        assert!(restores_of(&mut a).is_empty());
        assert_eq!(
            a.svc
                .command(Command::DismissRestoreIssue { package })
                .unwrap(),
            CommandOutcome::NothingToDo
        );
    }

    #[test]
    fn a_retried_restore_replaces_the_shown_refusal() {
        let (lab, mut a, _b, g, removal) = uninstalled_everywhere();
        a.act.set(Activity::Running { pids: vec![4242] });
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        delete_removed_blobs(&lab, &g, &removed_manifest(&a, &removal));
        a.act.set(Activity::NotRunning);
        synced(&mut a);
        assert_eq!(restores_of(&mut a).len(), 1);
        assert!(a.svc.command(Command::RestoreRemoved { removal }).is_err());
        let shown = restores_of(&mut a);
        assert_eq!(shown.len(), 1, "a failed retry keeps the entry: {shown:?}");
        assert!(matches!(
            shown[0].state,
            crate::restore_flow::RestoreState::Unavailable { .. }
        ));
    }

    #[test]
    fn a_restore_refused_when_asked_for_is_kept_as_unavailable_and_a_good_retry_clears_it() {
        use crate::restore_flow::RestoreState;
        let (lab, mut a, _b, g, removal) = uninstalled_everywhere();
        let manifest = removed_manifest(&a, &removal);
        let mut saved = Vec::new();
        for f in &manifest {
            let rec = wow_sync_core::records::decode_tree(
                &std::fs::read(provider_file(&lab, &layout::tree_record(&g, &f.tree))).unwrap(),
                &f.tree,
            )
            .unwrap();
            for file in rec.files {
                let p = provider_file(&lab, &layout::blob(&g, &file.sha256));
                saved.push((p.clone(), std::fs::read(p).unwrap()));
            }
        }
        delete_removed_blobs(&lab, &g, &manifest);
        let err = a
            .svc
            .command(Command::RestoreRemoved { removal })
            .unwrap_err();
        assert!(
            matches!(&err, CommandError::Service(t) if t.contains("unavailable")),
            "{err:?}"
        );
        let shown = restores_of(&mut a);
        assert_eq!(
            shown.len(),
            1,
            "refusal at command time is listed: {shown:?}"
        );
        assert!(matches!(shown[0].state, RestoreState::Unavailable { .. }));
        synced(&mut a);
        assert_eq!(restores_of(&mut a).len(), 1, "it stays across cycles");
        for (p, bytes) in saved {
            std::fs::write(p, bytes).unwrap();
        }
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        assert!(
            restores_of(&mut a)
                .iter()
                .all(|p| matches!(p.state, RestoreState::Waiting { .. })),
            "a good retry clears the refusal"
        );
        synced(&mut a);
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert!(restores_of(&mut a).is_empty());
    }

    fn corrupt_removed_blobs(lab: &Lab, g: &GroupId, m: &[wow_sync_core::kinds::ManifestFolder]) {
        for f in m {
            let p = provider_file(lab, &layout::tree_record(g, &f.tree));
            let rec =
                wow_sync_core::records::decode_tree(&std::fs::read(p).unwrap(), &f.tree).unwrap();
            for file in rec.files {
                let p = provider_file(lab, &layout::blob(g, &file.sha256));
                let mut bytes = std::fs::read(&p).unwrap();
                bytes[0] ^= 0xff;
                std::fs::write(&p, bytes).unwrap();
            }
        }
    }

    #[test]
    fn a_restore_whose_shared_copy_is_corrupt_installs_nothing_and_says_so_without_retrying() {
        use crate::restore_flow::RestoreState;
        let (lab, mut a, _b, g, removal) = uninstalled_everywhere();
        corrupt_removed_blobs(&lab, &g, &removed_manifest(&a, &removal));
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        let first = synced(&mut a);
        assert_eq!(a.read("Alpha/main.lua"), None);
        assert!(
            !a.install.addons.join("Alpha").exists(),
            "nothing installed"
        );
        assert_eq!(a.journals(), 0);
        assert_ne!(
            first.apply_note.as_deref(),
            Some("Settings are restored with the code"),
            "no settings note before the restore verified"
        );
        let shown = restores_of(&mut a);
        assert_eq!(shown.len(), 1, "{shown:?}");
        assert!(matches!(shown[0].state, RestoreState::Unavailable { .. }));
        assert!(
            shown[0].text.contains("can't be restored"),
            "{}",
            shown[0].text
        );
        let again = synced(&mut a);
        assert!(
            again.applied.is_empty(),
            "not retried every cycle: {:?}",
            again.applied
        );
        assert_eq!(restores_of(&mut a).len(), 1);
        // Try again re-plans
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        assert!(restores_of(&mut a)
            .iter()
            .all(|p| matches!(p.state, RestoreState::Waiting { .. })));
    }

    #[test]
    fn a_corrupt_shared_copy_is_replaced_by_the_intact_local_recovery_copy() {
        let (lab, _a, mut b, g, removal) = uninstalled_everywhere();
        corrupt_removed_blobs(&lab, &g, &removed_manifest(&b, &removal));
        b.svc.command(Command::RestoreRemoved { removal }).unwrap();
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert!(restores_of(&mut b).is_empty());
    }

    #[test]
    fn an_uninstall_and_restore_keep_each_account_and_character_file_to_its_own_identity() {
        let (lab, mut a, mut b, g) = settings_removal_ready(1 << 20);
        let svars = |pc: &Pc, rel: &str| pc.install.variant_dir.join(format!("WTF/{rel}"));
        let put = |pc: &Pc, rel: &str, text: &str| {
            let p = svars(pc, rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        let get = |pc: &Pc, rel: &str| std::fs::read_to_string(svars(pc, rel)).ok();
        let chr = "Account/ACC/Realm/Thrall/SavedVariables/Alpha.lua";
        let other = "Account/OTHER/SavedVariables/Alpha.lua";
        put(&a, chr, "AlphaDB = { who = \"thrall\" }\n");
        put(&a, other, "AlphaDB = { who = \"excluded account\" }\n");
        write_wtf(&b, "Alpha", "AlphaDB = { peer = true }\n");
        for _ in 0..2 {
            synced(&mut a);
            synced(&mut b);
        }
        uninstall_alpha_on(&mut a, &g);
        assert_eq!(read_wtf(&a, "Alpha"), None);
        assert_eq!(get(&a, chr), None, "the character file left with the code");
        assert_eq!(
            get(&a, other).as_deref(),
            Some("AlphaDB = { who = \"excluded account\" }\n"),
            "an account outside the sharing is never moved"
        );
        let (ac, _) = pc_ids(&a);
        let removal = removal_hash(&lab, &g, &ac);
        a.svc.command(Command::RestoreRemoved { removal }).unwrap();
        for _ in 0..8 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert_eq!(
            read_wtf(&a, "Alpha").as_deref(),
            Some("AlphaDB = { v = 1 }\n")
        );
        assert_eq!(
            get(&a, chr).as_deref(),
            Some("AlphaDB = { who = \"thrall\" }\n"),
            "the character file returns to its own character"
        );
        assert_eq!(
            get(&a, other).as_deref(),
            Some("AlphaDB = { who = \"excluded account\" }\n")
        );
        assert_eq!(get(&b, chr), None, "no file crosses to another identity");
        assert_eq!(get(&b, other), None);
    }

    #[test]
    fn the_settings_note_tells_code_only_partial_and_paired_restores_apart() {
        let code_only = crate::restore_flow::settings_note(2, 0);
        assert_eq!(code_only, crate::restore_flow::CODE_ONLY_WARNING);
        let partial = crate::restore_flow::settings_note(2, 1);
        assert!(
            partial.contains("partly") && partial.contains("1 of 2"),
            "{partial}"
        );
        let paired = crate::restore_flow::settings_note(2, 2);
        assert!(
            !paired.contains("partly") && paired != code_only,
            "{paired}"
        );
    }
    // <<< R5-remove

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
            2,
            "a non-master's routine write publishes nothing (the master's two revisions only)"
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
    fn the_snapshot_reports_settings_master_accounts_and_what_a_paused_folder_would_meet() {
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        let alpha = alpha_unit().folder;
        let (sa, sb) = (a.svc.status().settings, b.svc.status().settings);
        assert!(sa.enabled && sb.enabled);
        assert_eq!(sb.accounts, vec!["ACC".to_string()]);
        assert_eq!(sb.master, Some(ref_of(&a)));
        assert_eq!(sb.master_name, Some(a.svc.status().computer_name));
        assert!(sb.paused.is_empty(), "nothing is paused yet");

        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        synced(&mut b);
        b.svc
            .command(Command::PauseSettings {
                folder: alpha.clone(),
            })
            .unwrap();
        let s = b.svc.status().settings;
        assert_eq!(s.paused.len(), 1);
        assert!(
            !s.paused[0].here_changed && !s.paused[0].shared_newer,
            "{s:?}"
        );

        write_wtf(&a, "Alpha", "AlphaDB = { v = 3 }\n");
        synced(&mut a);
        synced(&mut b);
        let s = b.svc.status().settings;
        assert!(
            s.paused[0].shared_newer && !s.paused[0].here_changed,
            "{s:?}"
        );

        write_wtf(&b, "Alpha", "AlphaDB = { mine = true }\n");
        synced(&mut b);
        let s = b.svc.status().settings;
        assert!(
            s.paused[0].shared_newer && s.paused[0].here_changed,
            "{s:?}"
        );
        assert_eq!(s.paused[0].folder, alpha);
    }

    #[test]
    fn the_snapshot_settings_stay_disabled_and_unpaused_without_consent_or_cap() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon_with("Alpha", "alpha-v1", SV_TOC);
        a.create();
        synced(&mut a);
        let s = a.svc.status().settings;
        assert!(!s.enabled && s.accounts.is_empty() && s.master.is_none());
        assert!(s.paused.is_empty() && !s.policy_conflict);
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
            3,
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

    #[test]
    fn competing_settings_choices_are_resolved_by_one_choice_that_names_every_head() {
        use wow_sync_core::kinds::RevisionKind;
        use wow_sync_core::settings::SettingsWriteError as W;
        let lab = Lab::new();
        let (mut a, mut b, g) = settings_pair(&lab, true, Some(1 << 20));
        let unit = alpha_unit();
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        synced(&mut b);
        // b shares its own copy while the master's next revision is not yet visible to it:
        // b's record is held back until the master has published, so neither knew of the other.
        let mine = stored_checkpoint(&b, &unit, b"AlphaDB = { mine = true }\n");
        let out = b
            .svc
            .command(Command::ShareSettings {
                unit: unit.clone(),
                checkpoint: mine,
            })
            .unwrap();
        let CommandOutcome::Written(push) = out else {
            panic!("{out:?}")
        };
        let (bc, _) = pc_ids(&b);
        let held = provider_file(&lab, &layout::authority_record(&g, &bc, &push));
        let held_bytes = std::fs::read(&held).unwrap();
        std::fs::remove_file(&held).unwrap();
        write_wtf(&a, "Alpha", "AlphaDB = { v = 3 }\n");
        synced(&mut a);
        synced(&mut a);
        std::fs::write(&held, held_bytes).unwrap();
        synced(&mut b);
        let heads: Vec<_> = b
            .svc
            .settings_history(&unit)
            .into_iter()
            .filter(|e| e.is_head)
            .collect();
        assert_eq!(heads.len(), 2, "two intentional choices neither knew about");
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 2 }\n"),
            "nothing is applied while the choice is open"
        );
        let hashes: Vec<Sha256> = heads.iter().map(|e| e.hash).collect();
        let masters = heads
            .iter()
            .find(|e| e.kind == RevisionKind::Auto)
            .expect("the master's revision")
            .checkpoint;
        let choose = |shown: &[Sha256], cp: Sha256, u: &wow_sync_core::ids::SettingsUnit| {
            Command::ChooseSettings {
                unit: u.clone(),
                checkpoint: cp,
                shown_heads: shown.to_vec(),
            }
        };
        assert_eq!(
            b.svc.command(choose(&hashes[..1], masters, &unit)),
            Err(CommandError::Settings(W::Changed)),
            "a head the owner was not shown"
        );
        let stranger = sha256(b"not a head state");
        assert_eq!(
            b.svc.command(choose(&hashes, stranger, &unit)),
            Err(CommandError::Settings(W::NotAHead))
        );
        let beta = wow_sync_core::ids::SettingsUnit {
            folder: wow_sync_core::paths::validate_folder_name("Beta").unwrap(),
            identity: wow_sync_core::ids::Identity::account("ACC").unwrap(),
        };
        assert_eq!(
            b.svc.command(choose(&hashes, masters, &beta)),
            Err(CommandError::Settings(W::NotCompeting))
        );
        assert_eq!(
            b.svc.settings_history(&unit).len(),
            4,
            "refusals wrote nothing"
        );
        assert!(matches!(
            b.svc.command(choose(&hashes, masters, &unit)),
            Ok(CommandOutcome::Written(_))
        ));
        synced(&mut b);
        synced(&mut a);
        synced(&mut b);
        let history = b.svc.settings_history(&unit);
        let head: Vec<_> = history.iter().filter(|e| e.is_head).collect();
        assert_eq!(head.len(), 1);
        assert_eq!(head[0].kind, RevisionKind::Choice);
        assert_eq!(head[0].checkpoint, masters);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 3 }\n"),
            "the chosen copy is applied"
        );
    }

    #[test]
    fn the_snapshot_units_carry_the_heads_choose_takes_and_the_history_restore_takes() {
        use crate::settings_cycle::SettingsUnitStatus as S;
        let lab = Lab::new();
        let (mut a, mut b, g) = settings_pair(&lab, true, Some(1 << 20));
        let unit = alpha_unit();
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        synced(&mut b);
        let calm = b.svc.status().settings_units;
        assert_eq!(calm.len(), 1, "{calm:?}");
        assert_eq!(calm[0].status, S::Active);
        assert!(calm[0].live && calm[0].competing.is_empty() && calm[0].shown_heads.is_empty());
        assert!(calm[0].can_share && calm[0].latest_capture.is_some());
        assert_eq!(calm[0].history.len(), 2);

        let mine = stored_checkpoint(&b, &unit, b"AlphaDB = { mine = true }\n");
        let out = b
            .svc
            .command(Command::ShareSettings {
                unit: unit.clone(),
                checkpoint: mine,
            })
            .unwrap();
        let CommandOutcome::Written(push) = out else {
            panic!("{out:?}")
        };
        let (bc, _) = pc_ids(&b);
        let held = provider_file(&lab, &layout::authority_record(&g, &bc, &push));
        let held_bytes = std::fs::read(&held).unwrap();
        std::fs::remove_file(&held).unwrap();
        write_wtf(&a, "Alpha", "AlphaDB = { v = 3 }\n");
        synced(&mut a);
        synced(&mut a);
        std::fs::write(&held, held_bytes).unwrap();
        synced(&mut b);

        let row = b.svc.status().settings_units.remove(0);
        assert_eq!(row.status, S::NeedsChoice);
        assert!(!row.live);
        assert_eq!(row.competing.len(), 2, "{row:?}");
        assert_eq!(row.shown_heads.len(), 2);
        assert!(row.history.len() >= 3);
        let pick = row.competing[0].checkpoint;
        assert!(matches!(
            b.svc.command(Command::ChooseSettings {
                unit: unit.clone(),
                checkpoint: pick,
                shown_heads: row.shown_heads.clone(),
            }),
            Ok(CommandOutcome::Written(_))
        ));
        synced(&mut b);
        synced(&mut a);
        synced(&mut b);
        let after = b.svc.status().settings_units.remove(0);
        assert_eq!(after.status, S::Active, "{after:?}");
        assert!(after.competing.is_empty());

        let earlier = after.history[0].checkpoint;
        assert!(matches!(
            b.svc.command(Command::RestoreSettings {
                unit: unit.clone(),
                checkpoint: earlier,
            }),
            Ok(CommandOutcome::Written(_))
        ));
        synced(&mut b);
        let restored = b.svc.status().settings_units.remove(0);
        assert!(restored.history.len() > after.history.len());
    }

    #[test]
    fn the_snapshot_lists_the_account_folders_consent_can_include() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        // Negative: no WTF folder at all gives an empty list.
        assert!(a.svc.status().settings.available_accounts.is_empty());

        let account = |pc: &Pc, name: &str| pc.install.variant_dir.join("WTF/Account").join(name);
        std::fs::create_dir_all(account(&a, "ZED")).unwrap();
        std::fs::create_dir_all(account(&a, "ACC")).unwrap();
        std::fs::write(account(&a, "stray.txt"), b"not an account").unwrap();
        let st = a.svc.status().settings;
        assert_eq!(st.available_accounts, vec!["ACC", "ZED"]);
        assert!(!st.enabled);
        // The listed names are exactly what consent takes.
        a.svc
            .command(Command::EnableSettings {
                included_accounts: st.available_accounts[..1].to_vec(),
            })
            .unwrap();
        let st = a.svc.status().settings;
        assert!(st.enabled);
        assert_eq!(st.accounts, vec!["ACC"]);
        assert_eq!(st.available_accounts, vec!["ACC", "ZED"]);

        // Negative: another computer's accounts are not listed here, and a retired one has none.
        assert!(b.svc.status().settings.available_accounts.is_empty());
        std::fs::create_dir_all(account(&b, "BEE")).unwrap();
        a.svc
            .command(Command::Retire {
                subject: ref_of(&b),
            })
            .unwrap();
        synced(&mut a);
        synced(&mut b);
        assert!(b.svc.status().settings.available_accounts.is_empty());
    }

    #[test]
    fn the_snapshot_units_offer_no_actionable_heads_when_disabled_or_retired() {
        use crate::settings_cycle::SettingsUnitStatus as S;
        let lab = Lab::new();
        // With no size cap (tests only) nothing is actionable.
        let (mut a, mut b, _g) = settings_pair(&lab, true, None);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        synced(&mut b);
        for row in b.svc.status().settings_units {
            assert_eq!(row.status, S::Disabled);
            assert!(!row.live && !row.can_share && !row.restore_possible);
            assert!(row.competing.is_empty() && row.shown_heads.is_empty());
        }

        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        a.svc
            .command(Command::Retire {
                subject: ref_of(&b),
            })
            .unwrap();
        synced(&mut a);
        synced(&mut b);
        let rows = b.svc.status().settings_units;
        assert!(!rows.is_empty());
        for row in rows {
            assert_eq!(row.status, S::Retired);
            assert!(!row.live && !row.can_share && !row.restore_possible);
            assert!(row.competing.is_empty());
        }
    }

    #[test]
    fn restore_settings_writes_a_restore_revision_and_the_unit_returns_to_that_copy() {
        use wow_sync_core::kinds::RevisionKind;
        let lab = Lab::new();
        let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
        let unit = alpha_unit();
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        synced(&mut b);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 3 }\n");
        synced(&mut a);
        synced(&mut b);
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 3 }\n")
        );
        let history = b.svc.settings_history(&unit);
        assert_eq!(history.len(), 3);
        let earlier = history[1].checkpoint;
        let ghost = sha256(b"no such copy");
        assert_eq!(
            b.svc.command(Command::RestoreSettings {
                unit: unit.clone(),
                checkpoint: ghost
            }),
            Err(CommandError::UnknownCheckpoint(ghost))
        );
        assert_eq!(b.svc.settings_history(&unit).len(), 3);
        assert!(matches!(
            b.svc.command(Command::RestoreSettings {
                unit: unit.clone(),
                checkpoint: earlier
            }),
            Ok(CommandOutcome::Written(_))
        ));
        for _ in 0..2 {
            synced(&mut b);
            synced(&mut a);
        }
        let history = b.svc.settings_history(&unit);
        let head: Vec<_> = history.iter().filter(|e| e.is_head).collect();
        assert_eq!(head.len(), 1);
        assert_eq!(head[0].kind, RevisionKind::Restore);
        assert_eq!(head[0].checkpoint, earlier);
        for pc in [&a, &b] {
            assert_eq!(
                read_wtf(pc, "Alpha").as_deref(),
                Some("AlphaDB = { v = 2 }\n"),
                "both computers returned to the restored copy"
            );
        }
    }

    #[test]
    fn confirming_settings_here_needs_an_account_and_nothing_is_applied_before_it() {
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon_with("Alpha", "alpha-v1", SV_TOC);
        b.addon_with("Alpha", "alpha-v1", SV_TOC);
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        a.svc.set_settings_cap_for_tests(Some(1 << 20));
        b.svc.set_settings_cap_for_tests(Some(1 << 20));
        let master = ref_of(&a);
        std::fs::create_dir_all(b.install.variant_dir.join("WTF/Account/ACC")).unwrap();
        a.svc
            .command(Command::SetSettingsPolicy {
                master: Some(master),
                exceptions: vec![],
            })
            .unwrap();
        assert_eq!(
            a.svc.command(Command::EnableSettings {
                included_accounts: vec!["ACC".into()]
            }),
            Ok(CommandOutcome::Recorded)
        );
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha"),
            None,
            "an unconfirmed computer applies nothing"
        );
        assert_eq!(
            b.svc.command(Command::EnableSettings {
                included_accounts: vec![]
            }),
            Err(CommandError::EmptySelection),
            "confirming with no account included is not a confirmation"
        );
        synced(&mut b);
        assert_eq!(read_wtf(&b, "Alpha"), None);
        assert_eq!(
            b.svc.command(Command::EnableSettings {
                included_accounts: vec!["ACC".into()]
            }),
            Ok(CommandOutcome::Recorded)
        );
        synced(&mut b);
        synced(&mut b);
        assert_eq!(
            read_wtf(&b, "Alpha").as_deref(),
            Some("AlphaDB = { v = 2 }\n")
        );
    }

    #[test]
    fn undoing_a_removal_restores_the_removed_units_settings_to_that_unit_only() {
        use wow_sync_core::kinds::{RemovalSettings, RevisionKind};
        use wow_sync_core::records::{Location, RootState, SettingsRoot};
        use wow_sync_core::settings::SettingsWriteError as W;
        let lab = Lab::new();
        let (mut a, mut b, g) = settings_pair(&lab, true, Some(1 << 20));
        let unit = alpha_unit();
        a_removes_alpha(&mut a, &g);
        let (ac, _) = pc_ids(&a);
        let real = removal_hash(&lab, &g, &ac);
        let kept = stored_checkpoint(&a, &unit, b"AlphaDB = { kept = true }\n");
        // The removal record that carries the captured settings of Alpha's unit.
        let p = provider_file(&lab, &layout::authority_record(&g, &ac, &real));
        let Ok(AuthorityRecord::Removal(mut rm)) =
            decode_authority(&std::fs::read(p).unwrap(), &real)
        else {
            panic!("a removal")
        };
        rm.issuer.issued_at_seq = 1000;
        rm.settings = RemovalSettings::Checkpoints {
            roots: vec![SettingsRoot {
                unit: unit.clone(),
                state: RootState::Captured {
                    checkpoint: kept,
                    location: Location::Local,
                },
            }],
        };
        let with_settings = put_authority(&lab, &g, &ac, &AuthorityRecord::Removal(rm));
        for _ in 0..2 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(status(&a, &g, &with_settings), Some(Status::Valid));
        let beta = wow_sync_core::ids::SettingsUnit {
            folder: wow_sync_core::paths::validate_folder_name("Beta").unwrap(),
            identity: wow_sync_core::ids::Identity::account("ACC").unwrap(),
        };
        let ghost = sha256(b"no such removal");
        let restore = |removal: Sha256, unit: &wow_sync_core::ids::SettingsUnit| {
            Command::RestoreRemovedSettings {
                removal,
                unit: unit.clone(),
            }
        };
        assert_eq!(
            a.svc.command(restore(ghost, &unit)),
            Err(CommandError::UnknownRemoval(ghost))
        );
        assert_eq!(
            a.svc.command(restore(with_settings, &beta)),
            Err(CommandError::Settings(W::NotRemovedUnit)),
            "settings return to the same unit only"
        );
        assert_eq!(
            a.svc.command(restore(real, &unit)),
            Err(CommandError::Settings(W::NotRemovedUnit)),
            "a removal that captured no settings restores none"
        );
        assert!(a.svc.settings_history(&unit).is_empty());
        assert!(matches!(
            a.svc.command(restore(with_settings, &unit)),
            Ok(CommandOutcome::Written(_))
        ));
        let history = a.svc.settings_history(&unit);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].kind, RevisionKind::Restore);
        assert_eq!(history[0].checkpoint, kept);
        assert!(
            provider_file(&lab, &layout::settings_manifest(&kept)).is_file(),
            "the restored copy is reachable by the other computers"
        );
        // Settings go back through their own command; the code restore is a separate command.
        assert_eq!(a.svc.settings_history(&unit).len(), 1);
    }

    #[test]
    fn a_policy_with_thousands_of_exceptions_is_written_in_parts_and_read_back_whole() {
        use wow_sync_core::kinds::{PolicyRule, PolicyRuleKind};
        let lab = Lab::new();
        let (mut a, mut b, g) = settings_pair(&lab, true, Some(1 << 20));
        let (ac, _) = pc_ids(&a);
        let before = authority_files(&lab, &g, &ac);
        let not_shared = |name: &str| PolicyRule {
            folder: wow_sync_core::paths::validate_folder_name(name).unwrap(),
            rule: PolicyRuleKind::NotShared,
            computer_id: None,
            enrollment_id: None,
        };
        // "Alpha" sorts after every "A0…" rule, so its rule is in the last part, not inline.
        let mut rules: Vec<_> = (0..3000).map(|n| not_shared(&format!("A{n:05}"))).collect();
        rules.push(not_shared("Alpha"));
        let out = a
            .svc
            .command(Command::SetSettingsPolicy {
                master: Some(ref_of(&a)),
                exceptions: rules,
            })
            .unwrap();
        let CommandOutcome::Written(policy) = out else {
            panic!("{out:?}")
        };
        let after = authority_files(&lab, &g, &ac);
        let parts: Vec<String> = after
            .iter()
            .filter(|f| !before.contains(f) && **f != format!("{policy}.json"))
            .cloned()
            .collect();
        assert!(
            parts.len() >= 2,
            "the rules were split into parts: {parts:?}"
        );
        for f in after.iter().filter(|f| !before.contains(f)) {
            let len = std::fs::metadata(provider_file(
                &lab,
                &layout::authority_record(
                    &g,
                    &ac,
                    &Sha256::parse(f.trim_end_matches(".json")).unwrap(),
                ),
            ))
            .unwrap()
            .len();
            assert!(len <= 64 * 1024, "{f} is {len} bytes");
        }
        synced(&mut a);

        // A part that has not arrived yet leaves the policy pending: it acts on nothing.
        let part_file = |f: &str| {
            provider_file(
                &lab,
                &layout::authority_record(
                    &g,
                    &ac,
                    &Sha256::parse(f.trim_end_matches(".json")).unwrap(),
                ),
            )
        };
        let missing = part_file(&parts[0]);
        let missing_bytes = std::fs::read(&missing).unwrap();
        std::fs::remove_file(&missing).unwrap();
        synced(&mut b);
        assert_ne!(status(&b, &g, &policy), Some(Status::Valid));

        std::fs::write(&missing, missing_bytes).unwrap();
        synced(&mut b);
        assert_eq!(status(&b, &g, &policy), Some(Status::Valid));

        // The rule that lives in a part is honoured: Alpha's settings are not shared.
        write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
        synced(&mut a);
        write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
        synced(&mut a);
        synced(&mut b);
        synced(&mut b);
        assert_eq!(read_wtf(&b, "Alpha"), None);
    }

    /// Both computers hold a keep of their own copy and have synced until each offers the conflict.
    fn conflicted_pair(
        lab: &Lab,
    ) -> (
        Pc,
        Pc,
        GroupId,
        wow_sync_core::decision::DecisionItem,
        Vec<wow_sync_core::decision::OptionId>,
    ) {
        use wow_sync_core::decision::{DecisionKind, OptionLabel};
        let (mut a, mut b, _c, g) = alpha_v2_published(lab);
        put_mutual_keep_of_alpha(lab, &g, &a, &b);
        put_mutual_keep_of_alpha(lab, &g, &b, &a);
        for _ in 0..3 {
            synced(&mut a);
            synced(&mut b);
        }
        let item = b
            .svc
            .offered_decisions()
            .iter()
            .find(|i| i.reason == DecisionKind::ChoicesDiffer)
            .expect("the conflict is offered")
            .clone();
        let copies = item
            .options
            .iter()
            .filter(|o| matches!(o.label, OptionLabel::Release { .. }))
            .map(|o| o.id)
            .collect();
        (a, b, g, item, copies)
    }

    #[test]
    fn a_stale_answer_after_a_peer_settled_the_conflict_writes_nothing() {
        // Run 009 round 3 (RV9-01): B answered an old view after A had already decided.
        use wow_sync_core::decision::{DecideError, DecisionId, DecisionKind};
        let lab = Lab::new();
        let (mut a, mut b, g, item, copies) = conflicted_pair(&lab);
        let answer = |option| Command::Decide {
            item: DecisionId(item.scope.clone()),
            option,
            shown: item.evidence,
        };
        let a_item = a
            .svc
            .offered_decisions()
            .iter()
            .find(|i| i.reason == DecisionKind::ChoicesDiffer)
            .expect("A is offered the conflict too")
            .clone();
        a.svc
            .command(Command::Decide {
                item: DecisionId(a_item.scope.clone()),
                option: a_item.options[0].id,
                shown: a_item.evidence,
            })
            .unwrap();
        synced(&mut a);
        let before = authority_files(&lab, &g, &pc_ids(&b).0).len();
        assert_eq!(
            b.svc.command(answer(copies[1])),
            Err(CommandError::Decide(DecideError::Changed))
        );
        assert_eq!(authority_files(&lab, &g, &pc_ids(&b).0).len(), before);
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(a.read("Alpha/main.lua"), b.read("Alpha/main.lua"));
        for pc in [&a, &b] {
            assert!(!offered_kinds(pc).contains(&DecisionKind::ChoicesDiffer));
        }
    }

    #[test]
    fn an_answer_kept_while_paused_is_written_once_and_not_offered_again() {
        // Run 009 round 3 (RV9-01): the paused answer must not leave the item offered.
        use wow_sync_core::decision::{DecideError, DecisionId, DecisionKind, OptionId};
        let lab = Lab::new();
        let (mut a, mut b, g, item, copies) = conflicted_pair(&lab);
        let answer = |option| Command::Decide {
            item: DecisionId(item.scope.clone()),
            option,
            shown: item.evidence,
        };
        assert_eq!(
            b.svc.command(answer(OptionId(99))),
            Err(CommandError::Decide(DecideError::UnknownOption))
        );
        let stale = Command::Decide {
            item: DecisionId(item.scope.clone()),
            option: copies[0],
            shown: wow_sync_core::decision::EvidenceToken(sha256(b"older view")),
        };
        assert_eq!(
            b.svc.command(stale),
            Err(CommandError::Decide(DecideError::Changed))
        );
        let before = authority_files(&lab, &g, &pc_ids(&b).0).len();
        b.svc.set_paused(true).unwrap();
        assert_eq!(b.svc.command(answer(copies[0])), Ok(CommandOutcome::Kept));
        assert_eq!(authority_files(&lab, &g, &pc_ids(&b).0).len(), before);
        b.svc.set_paused(false).unwrap();
        synced(&mut b);
        synced(&mut b);
        assert_eq!(authority_files(&lab, &g, &pc_ids(&b).0).len(), before + 1);
        assert!(!offered_kinds(&b).contains(&DecisionKind::ChoicesDiffer));
        assert_eq!(
            b.svc.command(answer(copies[1])),
            Err(CommandError::Decide(DecideError::Changed))
        );
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(a.read("Alpha/main.lua"), b.read("Alpha/main.lua"));
        assert_eq!(authority_files(&lab, &g, &pc_ids(&b).0).len(), before + 1);
    }

    #[test]
    fn a_paused_answer_stays_hidden_after_a_pass_and_is_not_offered_after_the_resume_pass() {
        // Run 009 round 4 (RV9-12, RV9-11): fails when `hide_answered` is disabled.
        use wow_sync_core::decision::{DecideError, DecisionId, DecisionKind};
        let lab = Lab::new();
        let (mut a, mut b, g, item, copies) = conflicted_pair(&lab);
        let answer = |option| Command::Decide {
            item: DecisionId(item.scope.clone()),
            option,
            shown: item.evidence,
        };
        let before = authority_files(&lab, &g, &pc_ids(&b).0).len();
        b.svc.set_paused(true).unwrap();
        assert_eq!(b.svc.command(answer(copies[0])), Ok(CommandOutcome::Kept));
        assert!(!offered_kinds(&b).contains(&DecisionKind::ChoicesDiffer));
        synced(&mut b);
        assert!(
            !offered_kinds(&b).contains(&DecisionKind::ChoicesDiffer),
            "a pass while paused re-offered the answered item"
        );
        for option in [copies[1], copies[0]] {
            assert_eq!(
                b.svc.command(answer(option)),
                Err(CommandError::Decide(DecideError::Changed))
            );
        }
        b.svc.set_paused(false).unwrap();
        synced(&mut b);
        assert!(
            !offered_kinds(&b).contains(&DecisionKind::ChoicesDiffer),
            "the pass that wrote the answer listed it once more"
        );
        assert_eq!(authority_files(&lab, &g, &pc_ids(&b).0).len(), before + 1);
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
        }
        for pc in [&a, &b] {
            assert!(!offered_kinds(pc).contains(&DecisionKind::ChoicesDiffer));
        }
        assert_eq!(a.read("Alpha/main.lua"), b.read("Alpha/main.lua"));
    }

    #[test]
    fn a_command_reads_through_the_hash_cache_and_leaves_a_pending_full_check_for_the_pass() {
        // Run 009 round 5 (G1): command dispatch must not re-hash unchanged files, and must not
        // consume a pending "check all files" request.
        use std::sync::atomic::Ordering::Relaxed;
        use wow_sync_core::decision::DecisionId;
        let lab = Lab::new();
        let (_a, mut b, _g, item, copies) = conflicted_pair(&lab);
        let cache = crate::HashCache::open(&b.cfg.state_dir, "round5");
        b.svc.set_read_ctl(ScanCtl {
            cache: Some(cache.clone()),
            ..ScanCtl::default()
        });
        synced(&mut b);
        synced(&mut b);
        assert!(!cache.is_empty(), "the pass filled the cache");
        let (h0, m0) = (cache.hits.load(Relaxed), cache.misses.load(Relaxed));
        b.svc.status();
        assert_eq!(cache.misses.load(Relaxed), m0, "status re-hashed a file");
        assert!(cache.hits.load(Relaxed) > h0);
        cache.request_full();
        let m1 = cache.misses.load(Relaxed);
        b.svc.status();
        b.svc
            .command(Command::Decide {
                item: DecisionId(item.scope.clone()),
                option: copies[0],
                shown: item.evidence,
            })
            .unwrap();
        assert_eq!(cache.misses.load(Relaxed), m1, "a command re-hashed a file");
        assert!(cache.full_requested(), "a command consumed the full check");
        synced(&mut b);
        assert!(!cache.full_requested(), "the next pass did the full check");
        assert!(cache.misses.load(Relaxed) > m1);
    }

    #[test]
    fn a_decision_looks_again_only_at_its_own_folders() {
        // Run 009 round 7 (G1): a large unrelated folder whose mtimes changed is not re-hashed
        // by Decide, while a change inside the decided folder still makes the answer stale.
        use std::sync::atomic::Ordering::Relaxed;
        use wow_sync_core::decision::{DecideError, DecisionId};
        let lab = Lab::new();
        let (_a, mut b, _g, item, copies) = conflicted_pair(&lab);
        let cache = crate::HashCache::open(&b.cfg.state_dir, "round7");
        b.svc.set_read_ctl(ScanCtl {
            cache: Some(cache.clone()),
            ..ScanCtl::default()
        });
        let big = b.install.addons.join("Unrelated");
        std::fs::create_dir_all(&big).unwrap();
        std::fs::write(big.join("Unrelated.toc"), "## T\nf0.lua\n").unwrap();
        for i in 0..60 {
            std::fs::write(big.join(format!("f{i}.lua")), vec![b'x'; 4096]).unwrap();
        }
        b.install.set_old_mtimes();
        synced(&mut b);
        synced(&mut b);
        let answer = |item: &wow_sync_core::decision::DecisionItem| Command::Decide {
            item: DecisionId(item.scope.clone()),
            option: copies[0],
            shown: item.evidence,
        };
        let item = b
            .svc
            .offered_decisions()
            .iter()
            .find(|i| i.scope == item.scope)
            .expect("still offered")
            .clone();
        // Same bytes, new mtimes: only a re-read would notice.
        for i in 0..60 {
            std::fs::write(big.join(format!("f{i}.lua")), vec![b'x'; 4096]).unwrap();
        }
        let (h0, m0) = (cache.hits.load(Relaxed), cache.misses.load(Relaxed));
        let mut stale = item.clone();
        stale.evidence = wow_sync_core::decision::EvidenceToken(sha256(b"older view"));
        assert_eq!(
            b.svc.command(answer(&stale)),
            Err(CommandError::Decide(DecideError::Changed))
        );
        assert!(
            cache.misses.load(Relaxed) < m0 + 60,
            "the unrelated folder was re-hashed: {} new misses",
            cache.misses.load(Relaxed) - m0
        );
        assert!(
            cache.hits.load(Relaxed) > h0,
            "the decided folder is read through the cache"
        );
        assert!(b.svc.command(answer(&item)).is_ok());
        assert!(
            cache.misses.load(Relaxed) < m0 + 60,
            "deciding re-hashed the unrelated folder"
        );
    }

    #[test]
    fn a_change_inside_the_decided_folder_still_makes_the_answer_stale() {
        use wow_sync_core::decision::{DecideError, DecisionId};
        let lab = Lab::new();
        let (_a, mut b, _g, item, copies) = conflicted_pair(&lab);
        b.addon("Alpha", "alpha-v9");
        let r = b.svc.command(Command::Decide {
            item: DecisionId(item.scope.clone()),
            option: copies[0],
            shown: item.evidence,
        });
        assert_eq!(r, Err(CommandError::Decide(DecideError::Changed)), "{r:?}");
    }

    #[test]
    fn diagnostics_and_ledger_only_commands_answer_from_the_last_scan() {
        // Run 009 round 5 (G1): no local read at all, not even a cached one.
        use std::sync::atomic::Ordering::Relaxed;
        use wow_sync_core::diagnostics::ExportOptions;
        let lab = Lab::new();
        let (_a, mut b, _g, _item, _copies) = conflicted_pair(&lab);
        let cache = crate::HashCache::open(&b.cfg.state_dir, "round5b");
        b.svc.set_read_ctl(ScanCtl {
            cache: Some(cache.clone()),
            ..ScanCtl::default()
        });
        synced(&mut b);
        synced(&mut b);
        let looked = |c: &crate::HashCache| c.hits.load(Relaxed) + c.misses.load(Relaxed);
        let before = looked(&cache);
        let out = b.svc.command(Command::ExportDiagnostics {
            options: ExportOptions {
                include_fingerprints: true,
                ..Default::default()
            },
        });
        assert!(out.is_ok(), "{out:?}");
        assert_eq!(looked(&cache), before, "diagnostics read the install again");
        let _ = b.svc.command(Command::AllowAgain { rejections: vec![] });
        assert_eq!(
            looked(&cache),
            before,
            "a ledger-only command read the install"
        );
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
        let snap = a.svc.status();
        assert_eq!(
            snap.rejected_releases.len(),
            1,
            "{:?}",
            snap.rejected_releases
        );
        let row = &snap.rejected_releases[0];
        assert_eq!(row.rejections, vec![rh]);
        assert_eq!(row.label, "3.0");
        assert_eq!(row.release, hash(&bad));
        assert_eq!(row.rejected_at_ms, 5);
        assert_eq!(snap.older_peer, None, "every test slice honours rollback");
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
        assert!(
            a.svc.status().rejected_releases.is_empty(),
            "a lifted rejection leaves the allow-again list"
        );
        // The rejection is no longer active, so it cannot be lifted twice.
        assert_eq!(
            a.svc.command(Command::AllowAgain {
                rejections: vec![rh]
            }),
            Err(CommandError::UnknownRejection(rh))
        );
    }

    #[test]
    fn restoring_a_copy_a_keep_displaced_converges_the_group_on_it() {
        use wow_sync_core::hash::package_hash;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Amb", "from-a");
        b.addon("Amb", "from-b");
        let g = a.create();
        b.join(&g);
        for _ in 0..3 {
            synced(&mut a);
            synced(&mut b);
        }
        let hash_of = |pc: &Pc| {
            let f = folder_of(&lab, &g, pc, "amb");
            (f.key.clone(), package_hash(&[(f.key, f.tree.unwrap())]))
        };
        let (key, a_rel) = hash_of(&a);
        let (_, b_rel) = hash_of(&b);
        assert_ne!(a_rel, b_rel);
        assert!(b
            .svc
            .command(Command::Keep {
                scope: vec![key.clone()],
                copy: b_rel,
                why: wow_sync_core::kinds::ChoiceReason::Ambiguous,
            })
            .is_ok());
        for _ in 0..4 {
            synced(&mut b);
            synced(&mut a);
        }
        assert_eq!(a.read("Amb/main.lua").as_deref(), Some("from-b"));
        assert!(offered_kinds(&a).is_empty() && offered_kinds(&b).is_empty());

        let go = Command::Rollback {
            package: key.clone(),
            recovered: a_rel,
        };
        assert_eq!(a.svc.command(go), Ok(CommandOutcome::Recorded));
        let mut lb = SyncReport::default();
        for _ in 0..5 {
            synced(&mut a);
            lb = synced(&mut b);
        }
        let (ac, _) = pc_ids(&a);
        assert!(
            !authority_files(&lab, &g, &ac).is_empty(),
            "rejection written"
        );
        assert_eq!(a.read("Amb/main.lua").as_deref(), Some("from-a"));
        assert_eq!(
            b.read("Amb/main.lua").as_deref(),
            Some("from-a"),
            "the group converges on the restored copy: {:?} {:?} {}",
            offered_kinds(&a),
            offered_kinds(&b),
            texts(&lb)
        );
        assert!(offered_kinds(&a).is_empty() && offered_kinds(&b).is_empty());
    }

    #[test]
    fn an_uninstall_after_a_group_restore_converges_on_every_computer() {
        use wow_sync_core::hash::package_hash;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Amb", "from-a");
        b.addon("Amb", "from-b");
        let g = a.create();
        b.join(&g);
        for _ in 0..3 {
            synced(&mut a);
            synced(&mut b);
        }
        let hash_of = |pc: &Pc| {
            let f = folder_of(&lab, &g, pc, "amb");
            (f.key.clone(), package_hash(&[(f.key, f.tree.unwrap())]))
        };
        let (key, a_rel) = hash_of(&a);
        let (_, b_rel) = hash_of(&b);
        b.svc
            .command(Command::Keep {
                scope: vec![key.clone()],
                copy: b_rel,
                why: wow_sync_core::kinds::ChoiceReason::Ambiguous,
            })
            .unwrap();
        for _ in 0..4 {
            synced(&mut b);
            synced(&mut a);
        }
        a.svc
            .command(Command::Rollback {
                package: key.clone(),
                recovered: a_rel,
            })
            .unwrap();
        for _ in 0..5 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(b.read("Amb/main.lua").as_deref(), Some("from-a"));

        std::fs::remove_dir_all(a.install.addons.join("Amb")).unwrap();
        for _ in 0..3 {
            synced(&mut a);
        }
        let st = LocalState::open_at(&a.cfg.state_dir).unwrap();
        let store = RemovalStore::new(&st, g.clone());
        let (base, mut obs) = store.load();
        obs.gone
            .insert(base.packages.keys().next().unwrap().clone(), (1, 1));
        store.save(&base, &obs).unwrap();
        for _ in 0..6 {
            synced(&mut a);
            synced(&mut b);
        }
        assert_eq!(
            a.read("Amb/main.lua"),
            None,
            "the originating computer is not given the code back"
        );
        assert_eq!(b.read("Amb/main.lua"), None);
        assert_eq!(a.svc.status().removed.len(), 1, "it is offered in Removed");
    }

    #[test]
    fn a_rollback_of_a_copy_the_group_never_displaced_is_still_refused() {
        use wow_sync_core::hash::package_hash;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Amb", "from-a");
        a.addon("Other", "unrelated");
        let g = a.create();
        for _ in 0..3 {
            synced(&mut a);
        }
        let f = folder_of(&lab, &g, &a, "amb");
        let other = folder_of(&lab, &g, &a, "other");
        let other_release = package_hash(&[(other.key.clone(), other.tree.unwrap())]);
        let (ac, _) = pc_ids(&a);
        let before = authority_files(&lab, &g, &ac);
        // A real, held release, but of another add-on: it is not a copy of Amb.
        assert_eq!(
            a.svc.command(Command::Rollback {
                package: f.key,
                recovered: other_release
            }),
            Err(CommandError::UnknownCopy(other_release))
        );
        for _ in 0..3 {
            synced(&mut a);
        }
        assert_eq!(
            authority_files(&lab, &g, &ac),
            before,
            "a refused rollback writes no rejection"
        );
        assert_eq!(a.read("Amb/main.lua").as_deref(), Some("from-a"));
    }

    #[test]
    fn a_previewed_save_writes_the_previewed_bytes_even_after_the_state_changed() {
        use crate::diag::ExportError;
        use wow_sync_core::decision::DecideError;
        use wow_sync_core::diagnostics::ExportOptions;
        let lab = Lab::new();
        let mut a = lab.pc("workstation-one");
        a.addon("Alpha", "alpha-v1");
        a.create();
        synced(&mut a);
        let options = ExportOptions::default();
        let Ok(CommandOutcome::Exported(preview)) =
            a.svc.command(Command::ExportDiagnostics { options })
        else {
            panic!("preview refused");
        };
        // The state changes after the preview: a new add-on is picked up and published.
        a.addon("Beta", "beta-v1");
        for _ in 0..3 {
            synced(&mut a);
        }
        let Ok(CommandOutcome::Exported(fresh)) =
            a.svc.command(Command::ExportDiagnostics { options })
        else {
            panic!("refused");
        };
        assert_ne!(fresh.text, preview.text, "the export really did change");
        // The older preview is no longer the latest one, so it is refused and nothing is written.
        let out = lab.tmp.join("saved");
        std::fs::create_dir(&out).unwrap();
        let dest = out.join("d.txt");
        assert_eq!(
            a.svc.command(Command::SavePreviewedDiagnostics {
                export: preview.clone(),
                dest: dest.clone()
            }),
            Err(CommandError::Decide(DecideError::Changed))
        );
        assert!(!dest.exists());
        // Text the service never produced is refused too.
        let mut forged = fresh.clone();
        forged.text.push_str("extra");
        assert!(a
            .svc
            .command(Command::SavePreviewedDiagnostics {
                export: forged,
                dest: dest.clone()
            })
            .is_err());
        assert!(!dest.exists());
        // The state changes again after the latest preview; the save still writes those bytes.
        a.addon("Gamma", "gamma-v1");
        for _ in 0..3 {
            synced(&mut a);
        }
        assert_eq!(
            a.svc.command(Command::SavePreviewedDiagnostics {
                export: fresh.clone(),
                dest: dest.clone()
            }),
            Ok(CommandOutcome::Exported(fresh.clone()))
        );
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), fresh.text);
        assert_ne!(
            std::fs::read_to_string(&dest).unwrap(),
            {
                let Ok(CommandOutcome::Exported(now)) =
                    a.svc.command(Command::ExportDiagnostics { options })
                else {
                    panic!("refused");
                };
                now.text
            },
            "a rebuild would have differed"
        );
        // The forbidden folders are still refused for a previewed save.
        let Ok(CommandOutcome::Exported(p)) = a.svc.command(Command::ExportDiagnostics { options })
        else {
            panic!("refused");
        };
        let bad = lab.shared.join("d.txt");
        assert!(matches!(
            a.svc.command(Command::SavePreviewedDiagnostics {
                export: p,
                dest: bad.clone()
            }),
            Err(CommandError::Export(ExportError::Save(_)))
        ));
        assert!(!bad.exists());
    }

    #[test]
    fn the_redaction_context_knows_wtf_names_and_the_provider_root() {
        let lab = Lab::new();
        let mut a = lab.pc("workstation-one");
        a.addon("Alpha", "alpha-v1");
        a.create();
        synced(&mut a);
        // Negative: no WTF folder gives no WTF names.
        let snap = a.svc.status();
        let ctx = a.svc.redaction_context(&snap);
        assert!(
            ctx.wtf_account_names.is_empty()
                && ctx.wtf_realm_names.is_empty()
                && ctx.wtf_character_names.is_empty()
        );
        let wtf = a.install.variant_dir.join("WTF/Account");
        std::fs::create_dir_all(wtf.join("PLANTEDACCT/Plantedrealm/Plantedchar")).unwrap();
        std::fs::create_dir_all(wtf.join("PLANTEDACCT/SavedVariables")).unwrap();
        let ctx = a.svc.redaction_context(&snap);
        assert_eq!(ctx.wtf_account_names, vec!["PLANTEDACCT"]);
        assert_eq!(ctx.wtf_realm_names, vec!["Plantedrealm"]);
        assert_eq!(ctx.wtf_character_names, vec!["Plantedchar"]);
        assert!(ctx.shared_root.contains(&*lab.shared.to_string_lossy()));
        assert_eq!(ctx.dropbox_root, ctx.shared_root);
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

    // >>> R5-decide: Q-88 (a) and the DataAtRisk producer
    /// A, B and C hold alpha-v1; A keeps v1 and then edits it into a newer local-only copy.
    fn kept_then_edited_locally(lab: &Lab) -> (Pc, Pc, GroupId, Sha256, Sha256) {
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        let published = alpha_folder(lab, &g, &a).tree.unwrap();
        let kh = put_keep_of_alpha(lab, &g, &a);
        a.addon("Alpha", "alpha-v3");
        (a, b, g, kh, published)
    }

    fn alpha_tree_published_by(lab: &Lab, g: &GroupId, pc: &Pc) -> Sha256 {
        alpha_folder(lab, g, pc).tree.unwrap()
    }

    #[test]
    fn a_newer_local_only_version_after_a_keep_stays_installed_unshared_and_is_asked_every_cycle() {
        use wow_sync_core::decision::DecisionKind;
        let lab = Lab::new();
        let (mut a, mut b, g, kh, v1_tree) = kept_then_edited_locally(&lab);
        for _ in 0..3 {
            synced(&mut a);
            assert_eq!(status(&a, &g, &kh), Some(Status::Valid));
            assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v3"));
            assert_eq!(
                offered_kinds(&a),
                vec![DecisionKind::NewVersionAfterDecision]
            );
            let snap = a.svc.status();
            assert_eq!(snap.local_only_versions.len(), 1, "{snap:?}");
            assert_eq!(snap.local_only_versions[0].package.as_str(), "alpha");
            let published = latest_state(&lab, &g, &a)
                .folders
                .into_iter()
                .find(|f| f.key.as_str() == "alpha")
                .and_then(|f| f.tree);
            assert!(
                published.is_none() || published == Some(v1_tree),
                "{published:?}"
            );
            synced(&mut b);
            assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        }
    }

    #[test]
    fn answering_a_local_only_version_decision_either_shares_it_or_returns_to_the_kept_version() {
        use wow_sync_core::decision::{OptionId, OptionLabel};
        let lab = Lab::new();
        let (mut a, mut b, g, _kh, v1_tree) = kept_then_edited_locally(&lab);
        synced(&mut a);
        let item = a.svc.offered_decisions()[0].clone();
        let pick = |want: fn(&OptionLabel) -> bool| -> OptionId {
            item.options.iter().find(|o| want(&o.label)).unwrap().id
        };
        let release = pick(|l| matches!(l, OptionLabel::Release { .. }));
        let own = item.options.iter().find(|o| o.id == release).unwrap();
        assert_eq!(own.holders.len(), 1, "{own:?}");
        a.svc
            .command(Command::Decide {
                item: wow_sync_core::decision::DecisionId(item.scope.clone()),
                option: release,
                shown: item.evidence,
            })
            .unwrap();
        for _ in 0..2 {
            synced(&mut a);
        }
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v3"));
        assert!(a.svc.status().local_only_versions.is_empty());
        assert_ne!(alpha_tree_published_by(&lab, &g, &a), v1_tree);
        synced(&mut b);
        assert_eq!(b.read("Alpha/main.lua").as_deref(), Some("alpha-v3"));

        let lab = Lab::new();
        let (mut a, _b, g, _kh, v1_tree) = kept_then_edited_locally(&lab);
        synced(&mut a);
        let item = a.svc.offered_decisions()[0].clone();
        let keep = item
            .options
            .iter()
            .find(|o| o.label == OptionLabel::KeepCurrent)
            .unwrap()
            .id;
        a.svc
            .command(Command::Decide {
                item: wow_sync_core::decision::DecisionId(item.scope.clone()),
                option: keep,
                shown: item.evidence,
            })
            .unwrap();
        for _ in 0..2 {
            synced(&mut a);
        }
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
        assert!(a.svc.offered_decisions().is_empty());
        assert_eq!(alpha_tree_published_by(&lab, &g, &a), v1_tree);
    }

    #[test]
    fn unreadable_local_state_is_a_data_at_risk_condition_in_the_snapshot_and_a_notice() {
        use wow_sync_adapters::notify::RecordingNotifier;
        use wow_sync_core::notice::NoticePolicy;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v1");
        a.create();
        let rec = std::sync::Arc::new(RecordingNotifier::default());
        a.svc
            .attach_notifier(rec.clone(), NoticePolicy::default())
            .unwrap();
        synced(&mut a);
        assert!(a.svc.status().data_at_risk.is_empty());
        std::fs::write(
            a.cfg.state_dir.join("pending-report.json"),
            b"{ not a report",
        )
        .unwrap();
        for _ in 0..3 {
            synced(&mut a);
            let risks = a.svc.status().data_at_risk;
            assert_eq!(risks.len(), 1, "{risks:?}");
            assert_eq!(risks[0].kind, crate::RiskKind::LocalStateUnreadable);
        }
        let sent = rec.sent();
        assert!(
            sent.iter().any(|n| n.urgent),
            "an urgent data-at-risk notice was sent: {sent:?}"
        );
    }

    #[test]
    fn a_recovery_copy_that_cannot_be_written_is_a_data_at_risk_condition() {
        let lab = Lab::new();
        let (mut a, _b, _c, _g) = alpha_v2_published(&lab);
        // A file stands where the recovery folder of alpha must be created.
        let recovery = a.install.variant_dir.join(".wowsync").join("recovery");
        std::fs::create_dir_all(&recovery).unwrap();
        std::fs::write(recovery.join("alpha"), b"in the way").unwrap();
        let r = synced(&mut a);
        let risks = a.svc.status().data_at_risk;
        assert!(
            risks
                .iter()
                .any(|x| x.kind == crate::RiskKind::RecoveryCopyFailed),
            "{risks:?} {r:?}"
        );
        assert_eq!(a.read("Alpha/main.lua").as_deref(), Some("alpha-v1"));
    }
    #[test]
    fn diagnostics_carry_real_file_fingerprints_only_when_asked() {
        use wow_sync_core::diagnostics::ExportOptions;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v1");
        a.create();
        synced(&mut a);
        fn export(a: &mut Pc, include_fingerprints: bool) -> String {
            match a
                .svc
                .command(Command::ExportDiagnostics {
                    options: ExportOptions {
                        include_fingerprints,
                        ..ExportOptions::default()
                    },
                })
                .unwrap()
            {
                CommandOutcome::Exported(e) => e.text,
                other => panic!("{other:?}"),
            }
        }
        assert!(!export(&mut a, false).contains("fingerprint="));
        let on = export(&mut a, true);
        let fp = on
            .split("fingerprint=")
            .nth(1)
            .expect("a fingerprint line")
            .split(|c: char| !c.is_ascii_hexdigit())
            .next()
            .unwrap();
        assert_eq!(fp.len(), 12, "{on}");
        a.addon("Alpha", "alpha-v2");
        synced(&mut a);
        assert_ne!(export(&mut a, true), on);
    }

    // <<< R5-decide

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

    /// A keep of `owner`'s Alpha that lists `rival`'s Alpha as considered and excluded, written
    /// without knowing any other record: the shape of the owner's real conflict (run 009).
    fn put_mutual_keep_of_alpha(lab: &Lab, g: &GroupId, owner: &Pc, rival: &Pc) -> Sha256 {
        use wow_sync_core::choice::{build_keep, ChoiceView, CopyEntry, WriteCx};
        use wow_sync_core::kinds::{ChoiceReason, ManifestFolder};
        use wow_sync_core::ledger::Ledger;
        use wow_sync_core::target::Source;
        let (oc, _) = pc_ids(owner);
        let entry = |pc: &Pc| {
            let f = alpha_folder(lab, g, pc);
            CopyEntry::new(
                vec![ManifestFolder {
                    key: f.key.clone(),
                    name: f.name.clone(),
                    tree: f.tree.unwrap(),
                }],
                vec![pc_ids(pc).1],
            )
        };
        let (mine, theirs) = (entry(owner), entry(rival));
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
            &[alpha_folder(lab, g, owner).key],
            &mine,
            &[mine.clone(), theirs],
            &Source::from_state(&latest_state(lab, g, owner), true),
        )
        .unwrap();
        put_authority(lab, g, &oc, &AuthorityRecord::CodeChoice(keep))
    }

    #[test]
    fn mutually_excluding_keeps_are_decided_once_and_a_second_click_is_refused() {
        // Run 009 R9-decide: each keep excluded the other's copy, so no option was offered;
        // and a second click on the old view must never write a record unaware of the first.
        use wow_sync_core::decision::{DecideError, DecisionId, DecisionKind, OptionLabel};
        let lab = Lab::new();
        let (mut a, mut b, _c, g) = alpha_v2_published(&lab);
        put_mutual_keep_of_alpha(&lab, &g, &a, &b);
        put_mutual_keep_of_alpha(&lab, &g, &b, &a);
        for _ in 0..3 {
            synced(&mut a);
            synced(&mut b);
        }
        let item = a
            .svc
            .offered_decisions()
            .iter()
            .find(|i| i.reason == DecisionKind::ChoicesDiffer)
            .expect("the conflict offers a decision")
            .clone();
        let copies: Vec<_> = item
            .options
            .iter()
            .filter(|o| matches!(o.label, OptionLabel::Release { .. }))
            .map(|o| o.id)
            .collect();
        assert_eq!(copies.len(), 2, "a keep of each copy: {:?}", item.options);
        let answer = |option| Command::Decide {
            item: DecisionId(item.scope.clone()),
            option,
            shown: item.evidence,
        };
        let before = authority_files(&lab, &g, &pc_ids(&a).0).len();
        a.svc.command(answer(copies[0])).unwrap();
        assert_eq!(
            a.svc.command(answer(copies[1])),
            Err(CommandError::Decide(DecideError::Changed)),
            "the first answer already settled this item"
        );
        assert_eq!(authority_files(&lab, &g, &pc_ids(&a).0).len(), before + 1);
        for _ in 0..4 {
            synced(&mut a);
            synced(&mut b);
        }
        let (ra, rb) = (a.read("Alpha/main.lua"), b.read("Alpha/main.lua"));
        assert_eq!(ra, rb, "both computers converge on the chosen copy");
        for pc in [&a, &b] {
            assert!(
                !offered_kinds(pc).contains(&DecisionKind::ChoicesDiffer),
                "{:?}",
                offered_kinds(pc)
            );
        }
    }
    #[test]
    fn the_holder_of_one_mutually_excluded_copy_syncs_offers_both_and_converges_when_it_decides() {
        // Run 009 round 2: RYZENFISH held one copy, LENOVOFISH the other; each keep excluded the
        // other copy and the kept copy was absent from the list, which panicked every pass.
        use wow_sync_core::decision::{DecisionId, DecisionKind, OptionLabel};
        let lab = Lab::new();
        let (mut a, mut b, mut c, g) = alpha_v2_published(&lab);
        put_mutual_keep_of_alpha(&lab, &g, &a, &b);
        put_mutual_keep_of_alpha(&lab, &g, &b, &a);
        for _ in 0..4 {
            synced(&mut b);
            synced(&mut c);
            synced(&mut a);
        }
        let item = b
            .svc
            .offered_decisions()
            .iter()
            .find(|i| i.reason == DecisionKind::ChoicesDiffer)
            .expect("the holder of one copy is offered the conflict")
            .clone();
        let labels = item
            .options
            .iter()
            .filter(|o| matches!(o.label, OptionLabel::Release { .. }))
            .count();
        assert_eq!(labels, 2, "{:?}", item.options);
        let mine = item
            .options
            .iter()
            .find(|o| matches!(o.label, OptionLabel::Release { .. }))
            .unwrap()
            .id;
        b.svc
            .command(Command::Decide {
                item: DecisionId(item.scope.clone()),
                option: mine,
                shown: item.evidence,
            })
            .unwrap();
        for _ in 0..5 {
            synced(&mut b);
            synced(&mut c);
            synced(&mut a);
        }
        let want = b.read("Alpha/main.lua");
        assert_eq!(a.read("Alpha/main.lua"), want);
        assert_eq!(c.read("Alpha/main.lua"), want);
        for pc in [&a, &b, &c] {
            assert!(
                !offered_kinds(pc).contains(&DecisionKind::ChoicesDiffer),
                "{:?}",
                offered_kinds(pc)
            );
        }
    }
    fn name_of(pc: &mut Pc) -> String {
        pc.svc.status().computer_name
    }

    fn hash_of_alpha(lab: &Lab, g: &GroupId, pc: &Pc) -> Sha256 {
        let f = alpha_folder(lab, g, pc);
        wow_sync_core::hash::package_hash(&[(f.key.clone(), f.tree.unwrap())])
    }

    #[test]
    fn the_snapshot_reports_retirements_and_the_retired_computers_hold() {
        use wow_sync_core::status::{AllHoldCause, HoldScope};
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        let before = a.svc.status();
        assert!(before.retirements.is_empty(), "{:?}", before.retirements);
        assert!(before.this_retirement.is_none());
        assert_eq!(before.hold, HoldScope::None);

        a.svc
            .command(Command::Retire {
                subject: ref_of(&b),
            })
            .unwrap();
        synced(&mut a);
        synced(&mut b);
        let (a_name, b_name) = (a.svc.status().computer_name, b.svc.status().computer_name);
        let sa = a.svc.status();
        assert_eq!(sa.retirements.len(), 1, "{:?}", sa.retirements);
        assert_eq!(sa.retirements[0].name, b_name);
        assert_eq!(sa.retirements[0].retired_by, "this computer");
        assert!(!sa.retirements[0].is_this);
        assert!(sa.this_retirement.is_none(), "the retirer is not retired");
        assert_eq!(sa.hold, HoldScope::None);

        let sb = b.svc.status();
        assert_eq!(sb.retirements.len(), 1);
        assert!(sb.retirements[0].is_this);
        assert_eq!(sb.retirements[0].retired_by, a_name);
        let info = sb.this_retirement.expect("b knows it is retired");
        assert_eq!(info.retired_by, pc_ids(&a).0);
        assert_eq!(sb.hold, HoldScope::AllApplies(AllHoldCause::Retired));
        assert!(!sb.review_needed);
        assert_eq!(
            sb.enrollment_names.get(&pc_ids(&b).1),
            Some(&b_name),
            "the retired enrollment still has a name"
        );
    }

    #[test]
    fn the_snapshot_lists_every_enrollment_with_its_standing_and_the_retirement_date() {
        use crate::detail::PeerStanding;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        b.join(&g);
        synced(&mut b);
        synced(&mut a);
        let before = a.svc.status();
        assert_eq!(before.peers.len(), 2, "{:?}", before.peers);
        assert!(before
            .peers
            .iter()
            .all(|p| p.standing == PeerStanding::Active));
        let theirs = before.peers.iter().find(|p| !p.is_this).unwrap();
        assert_eq!(
            theirs.enrollment,
            ref_of(&b),
            "retire must name this exact pair"
        );
        assert_eq!(theirs.name, b.svc.status().computer_name);
        assert!(before.peers[0].is_this && before.peers[0].name == "this computer");

        a.svc
            .command(Command::Retire {
                subject: ref_of(&b),
            })
            .unwrap();
        synced(&mut a);
        let after = a.svc.status();
        let theirs = after.peers.iter().find(|p| !p.is_this).unwrap();
        assert_eq!(theirs.standing, PeerStanding::Retired);
        let mine = after.peers.iter().find(|p| p.is_this).unwrap();
        assert_eq!(
            mine.standing,
            PeerStanding::Active,
            "the retirer is not retired"
        );
        assert!(
            theirs.retired_at_ms.is_some_and(|t| t > 0),
            "the date is carried"
        );
    }

    #[test]
    fn the_snapshot_hold_follows_pause_and_a_pending_decision_never_raises_review_needed() {
        use wow_sync_core::status::{AllHoldCause, HoldScope};
        let lab = Lab::new();
        let (mut a, b, _c, g) = alpha_v2_published(&lab);
        a.svc.set_paused(true).unwrap();
        assert_eq!(
            a.svc.status().hold,
            HoldScope::AllApplies(AllHoldCause::Paused)
        );
        a.svc.set_paused(false).unwrap();
        assert_eq!(a.svc.status().hold, HoldScope::None);

        put_keep_of_alpha(&lab, &g, &a);
        put_keep_of_alpha(&lab, &g, &b);
        synced(&mut a);
        synced(&mut a);
        let st = a.svc.status();
        assert!(
            matches!(&st.hold, HoldScope::Addons(v) if !v.is_empty()),
            "{:?}",
            st.hold
        );
        assert!(
            !st.review_needed,
            "a pending decision is not a reconfirmation (Q-59)"
        );
    }

    #[test]
    fn review_needed_never_hides_needs_help_or_error() {
        use wow_sync_core::status::Headline as H;
        for (pending, base, shown) in [
            (true, H::Ok, true),
            (true, H::Working, true),
            (true, H::Waiting, true),
            (true, H::NeedsHelp, false),
            (true, H::Error, false),
            (false, H::Ok, false),
            (false, H::Waiting, false),
        ] {
            assert_eq!(
                crate::detail::review_needed(pending, base),
                shown,
                "{pending} {base:?}"
            );
        }
    }

    #[test]
    fn the_snapshot_lists_held_releases_but_never_the_installed_one() {
        let lab = Lab::new();
        let (mut a, mut b, _c, g) = alpha_v2_published(&lab);
        let v1 = hash_of_alpha(&lab, &g, &a);
        let v2 = hash_of_alpha(&lab, &g, &b);
        let b_name = name_of(&mut b);
        let held = a.svc.status().held_releases;
        assert!(held.iter().all(|h| h.release != v1), "{held:?}");
        let row = held
            .iter()
            .find(|h| h.release == v2)
            .expect("b's newer release is held");
        assert_eq!(row.package.as_str(), "alpha");
        assert!(row.holders.contains(&b_name), "{row:?}");
        let rollback = a.svc.command(Command::Rollback {
            package: row.package.clone(),
            recovered: row.release,
        });
        assert!(
            !matches!(rollback, Err(CommandError::UnknownCopy(_))),
            "a listed release is one the command accepts: {rollback:?}"
        );
    }

    #[test]
    fn the_snapshot_lists_removed_addons_with_who_has_not_acknowledged() {
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
        assert!(b.svc.status().removed.is_empty());
        a_removes_alpha(&mut a, &g);
        synced(&mut b);
        let (a_name, c_name) = (name_of(&mut a), name_of(&mut c));
        let removed = b.svc.status().removed;
        assert_eq!(removed.len(), 1, "{removed:?}");
        assert_eq!(removed[0].folders, vec!["Alpha".to_string()]);
        assert!(removed[0].provenance.contains(&a_name), "{removed:?}");
        assert!(
            removed[0].awaiting.contains(&c_name),
            "the offline computer has not acknowledged: {removed:?}"
        );
    }

    fn recording() -> std::sync::Arc<wow_sync_adapters::notify::RecordingNotifier> {
        std::sync::Arc::new(wow_sync_adapters::notify::RecordingNotifier::new())
    }

    fn release_json(tag: &str) -> Vec<u8> {
        format!("{{\"tag_name\":\"{tag}\"}}").into_bytes()
    }

    fn fake_releases(
        tags: &[&str],
    ) -> std::sync::Arc<wow_sync_adapters::update_http::FakeReleases> {
        std::sync::Arc::new(wow_sync_adapters::update_http::FakeReleases::new(
            tags.iter().map(|t| Ok(release_json(t))).collect(),
        ))
    }

    #[test]
    fn an_applied_change_is_announced_once_and_not_when_summaries_are_off() {
        use wow_sync_core::notice::{NoticeKind, NoticePolicy};
        let lab = Lab::new();
        let mut a = lab.pc("a");
        let mut b = lab.pc("b");
        let mut c = lab.pc("c");
        a.addon("Alpha", "alpha-v1");
        let g = a.create();
        synced(&mut a);
        let (nb, nc) = (recording(), recording());
        b.join(&g);
        c.join(&g);
        b.svc
            .attach_notifier(nb.clone(), NoticePolicy::default())
            .unwrap();
        c.svc
            .attach_notifier(nc.clone(), NoticePolicy::default())
            .unwrap();
        c.svc
            .save_toggles(&AppToggles {
                notify_summaries: false,
                ..AppToggles::default()
            })
            .unwrap();
        assert_eq!(synced(&mut b).applied.len(), 1);
        assert_eq!(synced(&mut c).applied.len(), 1);
        assert_eq!(nb.sent().len(), 1, "{:?}", nb.sent());
        assert!(nb.sent()[0].body.contains("alpha"), "{:?}", nb.sent());
        assert!(!nb.sent()[0].urgent);
        synced(&mut b);
        assert_eq!(nb.sent().len(), 1, "an unchanged cycle announces nothing");
        assert!(nc.sent().is_empty(), "summaries off: {:?}", nc.sent());
        let _ = NoticeKind::Summary;
    }

    #[test]
    fn an_update_check_fills_the_snapshot_and_a_new_release_is_announced_once() {
        use wow_sync_core::notice::NoticePolicy;
        use wow_sync_core::update::UpdateResult;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v1");
        a.create();
        assert_eq!(
            a.svc.command(Command::CheckUpdates),
            Err(CommandError::UpdatesUnavailable)
        );
        assert_eq!(a.svc.status().update, None);
        let source = fake_releases(&["v9.9.9", "v9.9.9"]);
        let notes = recording();
        a.svc
            .attach_updates(source.clone(), system_clock())
            .unwrap();
        a.svc
            .attach_notifier(notes.clone(), NoticePolicy::default())
            .unwrap();
        a.svc
            .save_toggles(&AppToggles {
                update_check: false,
                ..AppToggles::default()
            })
            .unwrap();
        synced(&mut a);
        a.svc.wait_for_updates();
        assert_eq!(source.calls(), 0, "automatic check is off");
        assert_eq!(
            a.svc.command(Command::CheckUpdates),
            Ok(CommandOutcome::UpdateCheck(Started::Started)),
            "a manual check works with the automatic one off"
        );
        a.svc.wait_for_updates();
        assert_eq!(source.calls(), 1);
        assert!(matches!(
            a.svc.status().update,
            Some(UpdateResult::Available { ref tag, .. }) if tag == "v9.9.9"
        ));
        synced(&mut a);
        let sent: Vec<_> = notes
            .sent()
            .into_iter()
            .filter(|n| n.body.contains("9.9.9"))
            .collect();
        assert_eq!(sent.len(), 1, "{:?}", notes.sent());
        synced(&mut a);
        synced(&mut a);
        let again = notes
            .sent()
            .into_iter()
            .filter(|n| n.body.contains("9.9.9"))
            .count();
        assert_eq!(again, 1, "the same release is not announced twice");
    }

    #[test]
    fn the_diagnostics_export_reports_the_environment_the_adapters_read() {
        use wow_sync_core::diagnostics::ExportOptions;
        let lab = Lab::new();
        let mut a = lab.pc("a");
        a.addon("Alpha", "alpha-v1");
        a.create();
        synced(&mut a);
        let Ok(CommandOutcome::Exported(e)) = a.svc.command(Command::ExportDiagnostics {
            options: ExportOptions::default(),
        }) else {
            panic!("preview refused");
        };
        let line = |key: &str| {
            e.text
                .lines()
                .find(|l| l.starts_with(key))
                .unwrap_or_else(|| panic!("no {key} line in {}", e.text))
                .to_string()
        };
        let mem = line("process_memory:");
        match wow_sync_adapters::sysinfo::process_private_mib() {
            Some(_) => assert!(mem.ends_with(" MiB") && !mem.contains("unknown"), "{mem}"),
            None => assert_eq!(mem, "process_memory: unknown"),
        }
        let os = line("os:");
        match wow_sync_adapters::sysinfo::os_version() {
            Some((a, b, c)) => assert!(os.ends_with(&format!("{a}.{b}.{c}")), "{os}"),
            None => assert!(os.ends_with("unknown"), "{os}"),
        }
    }

    #[test]
    fn the_diagnostics_export_records_the_journals_left_on_disk() {
        use wow_sync_core::diagnostics::ExportOptions;
        let export = |pc: &mut Pc| {
            let Ok(CommandOutcome::Exported(e)) = pc.svc.command(Command::ExportDiagnostics {
                options: ExportOptions::default(),
            }) else {
                panic!("preview refused");
            };
            e.text
        };
        let mut f = two_package_fixture();
        let clean = export(&mut f.b);
        assert!(!clean.contains("journal:"), "{clean}");
        f.b.svc.inject_crash_after(Some(3));
        assert!(synced(&mut f.b).crashed);
        assert!(f.b.journals() > 0);
        let text = export(&mut f.b);
        assert_eq!(
            text.matches("journal: pending").count(),
            f.b.journals(),
            "{text}"
        );
        f.b.restart();
        for _ in 0..3 {
            synced(&mut f.b);
        }
        assert_eq!(f.b.journals(), 0);
        assert!(!export(&mut f.b).contains("journal:"));
    }

    #[test]
    fn diagnostics_preview_and_save_match_and_never_land_in_the_shared_or_install_folders() {
        use crate::diag::{ExportError, SaveError};
        use wow_sync_core::diagnostics::ExportOptions;
        let lab = Lab::new();
        let mut a = lab.pc("workstation-one");
        a.addon("Alpha", "alpha-v1");
        a.create();
        synced(&mut a);
        let options = ExportOptions::default();
        let Ok(CommandOutcome::Exported(preview)) =
            a.svc.command(Command::ExportDiagnostics { options })
        else {
            panic!("preview refused");
        };
        assert!(
            preview.text.contains("alpha"),
            "folder names are on by default: {}",
            preview.text
        );
        assert!(
            !preview.text.contains("workstation-one"),
            "{}",
            preview.text
        );
        let shared = lab.shared.to_string_lossy().into_owned();
        assert!(!preview.text.contains(&shared));

        let out = lab.tmp.join("elsewhere");
        std::fs::create_dir(&out).unwrap();
        let dest = out.join("diagnostics.txt");
        let Ok(CommandOutcome::Exported(saved)) =
            a.svc.command(Command::SavePreviewedDiagnostics {
                export: preview.clone(),
                dest: dest.clone(),
            })
        else {
            panic!("save refused");
        };
        assert_eq!(saved.text, preview.text);
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), saved.text);

        for forbidden in [lab.shared.clone(), a.install.root.clone()] {
            let bad = forbidden.join("diagnostics.txt");
            assert_eq!(
                a.svc.command(Command::SavePreviewedDiagnostics {
                    export: preview.clone(),
                    dest: bad.clone()
                }),
                Err(CommandError::Export(ExportError::Save(
                    SaveError::ForbiddenLocation
                )))
            );
            assert!(!bad.exists(), "nothing is written into {forbidden:?}");
        }
    }

    mod change_history {
        use super::*;
        use crate::history::{HistoryItem, Phase};

        fn changes(pc: &Pc) -> Vec<ChangeRecord> {
            pc.svc
                .history(100)
                .into_iter()
                .filter_map(|i| match i {
                    HistoryItem::Change(c) => Some(c),
                    _ => None,
                })
                .collect()
        }

        #[test]
        fn an_install_then_an_update_record_source_versions_counts_and_reason() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            let mut b = lab.pc("b");
            a.addon("Alpha", "alpha-v1");
            let g = a.create();
            synced(&mut a);
            b.join(&g);
            synced(&mut b);
            let first = changes(&b);
            let rec = first
                .iter()
                .find(|c| c.kind == ChangeKind::InstalledNew)
                .unwrap_or_else(|| panic!("{first:?}"));
            assert_eq!(rec.source, ChangeSource::Computer("a".into()));
            assert_eq!(rec.old_version, VersionLabel::New);
            assert_eq!(rec.new_version, VersionLabel::Toc("2.0".into()));
            assert_eq!(rec.files.added, 2);
            assert!(rec.byte_delta > 0);
            assert_eq!(rec.reason, ChangeReason::NewerRelease);
            assert!(rec.paths.contains(&"main.lua".to_string()), "{rec:?}");

            a.addon("Alpha", "alpha-v2");
            synced(&mut a);
            synced(&mut b);
            let all = changes(&b);
            let upd = all
                .iter()
                .find(|c| c.kind == ChangeKind::Applied)
                .unwrap_or_else(|| panic!("{all:?}"));
            assert_eq!(upd.source, ChangeSource::Computer("a".into()));
            assert_eq!(upd.old_version, VersionLabel::Toc("2.0".into()));
            assert_eq!(upd.new_version, VersionLabel::Toc("3.0".into()));
            assert_eq!(upd.files.changed, 2, "{upd:?}");
            assert_eq!(upd.files.added + upd.files.removed, 0);
            assert!(upd.unix_ms >= rec.unix_ms);

            b.restart();
            assert_eq!(changes(&b), all, "records survive a restart");
            let json = format!("{all:?}");
            assert!(!json.contains(&b.install.addons.to_string_lossy().to_string()));
        }

        #[test]
        fn a_removal_is_recorded_with_its_author_and_no_new_version() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            let mut b = lab.pc("b");
            let g = a.create();
            b.addon("Alpha", "alpha-v1");
            b.join(&g);
            synced(&mut b);
            a_removes_alpha(&mut a, &g);
            synced(&mut b);
            let all = changes(&b);
            let rm = all
                .iter()
                .find(|c| c.kind == ChangeKind::Removed)
                .unwrap_or_else(|| panic!("{all:?}"));
            assert_eq!(rm.new_version, VersionLabel::Removed);
            assert_eq!(rm.reason, ChangeReason::Removal);
            assert_eq!(rm.source, ChangeSource::Computer("a".into()));
            assert!(rm.files.removed >= 1);
            assert!(rm.byte_delta < 0);
        }

        #[test]
        fn progress_runs_scanning_to_idle_in_order_and_reports_each_record() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            let mut b = lab.pc("b");
            a.addon("Alpha", "alpha-v1");
            a.addon("Beta", "beta-v1");
            let g = a.create();
            synced(&mut a);
            b.join(&g);
            let rx = b.svc.progress_hub().subscribe();
            synced(&mut b);
            let events: Vec<_> = rx.try_iter().collect();
            let first = events.first().expect("events");
            assert_eq!(first.phase, Phase::Scanning);
            assert_eq!(first.items_done, 0);
            assert_eq!(events.last().unwrap().phase, Phase::Idle);
            assert!(events.windows(2).all(|w| w[0].seq < w[1].seq));
            assert!(events.iter().all(|e| e.pass_id == first.pass_id));
            let done: Vec<_> = events.iter().filter_map(|e| e.completed.as_ref()).collect();
            assert_eq!(done.len(), 2, "{done:?}");
            let inc = events
                .iter()
                .find_map(|e| e.incoming.as_ref())
                .expect("a two add-on apply is summarised");
            assert_eq!(inc.addons, 2);
            assert_eq!(inc.source, Some(ChangeSource::Computer("a".into())));
            assert!(events.iter().any(|e| e.phase == Phase::Installing));
            assert!(events.iter().any(|e| e.phase == Phase::Publishing));
            assert_eq!(
                b.svc
                    .history(100)
                    .into_iter()
                    .filter(|i| matches!(i, HistoryItem::Change(c) if c.kind == ChangeKind::InstalledNew))
                    .count(),
                2
            );
        }

        #[test]
        fn a_rollback_a_restore_and_a_settings_change_are_recorded_with_their_reasons() {
            use wow_sync_core::hash::package_hash;
            let lab = Lab::new();
            let (_a, mut b, c, g) = alpha_v2_published(&lab);
            let v1 = alpha_folder(&lab, &g, &c);
            let v1_hash = package_hash(&[(v1.key.clone(), v1.tree.unwrap())]);
            synced(&mut b);
            b.svc
                .command(Command::Rollback {
                    package: v1.key.clone(),
                    recovered: v1_hash,
                })
                .unwrap();
            synced(&mut b);
            let all = changes(&b);
            let rb = all
                .iter()
                .find(|c| c.kind == ChangeKind::RolledBack)
                .unwrap_or_else(|| panic!("{all:?}"));
            assert_eq!(rb.reason, ChangeReason::Rollback);
            assert_eq!(rb.source, ChangeSource::ThisComputer);
            assert_eq!(rb.files.changed, 2, "{rb:?}");
            assert_ne!(rb.old_version, rb.new_version);
        }

        #[test]
        fn publishing_a_package_announces_its_record_on_the_progress_stream() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            a.addon("Alpha", "alpha-v1");
            let g = a.create();
            let rx = a.svc.progress_hub().subscribe();
            synced(&mut a);
            let _ = g;
            let done: Vec<_> = rx
                .try_iter()
                .filter_map(|e| e.completed.map(|c| (e.phase, c)))
                .collect();
            assert!(
                done.iter()
                    .any(|(ph, c)| *ph == Phase::Publishing && c.kind == ChangeKind::Published),
                "{done:?}"
            );
        }

        #[test]
        fn a_peers_copy_choice_is_recorded_on_the_receiving_computer_as_made_there() {
            use wow_sync_core::decision::{DecisionId, DecisionKind, OptionLabel};
            let lab = Lab::new();
            let (mut a, mut b, _c, g) = alpha_v2_published(&lab);
            put_mutual_keep_of_alpha(&lab, &g, &a, &b);
            put_mutual_keep_of_alpha(&lab, &g, &b, &a);
            for _ in 0..3 {
                synced(&mut a);
                synced(&mut b);
            }
            let item = a
                .svc
                .offered_decisions()
                .iter()
                .find(|i| i.reason == DecisionKind::ChoicesDiffer)
                .expect("the conflict offers a decision")
                .clone();
            let copy = item
                .options
                .iter()
                .find(|o| matches!(o.label, OptionLabel::Release { .. }))
                .unwrap()
                .id;
            a.svc
                .command(Command::Decide {
                    item: DecisionId(item.scope.clone()),
                    option: copy,
                    shown: item.evidence,
                })
                .unwrap();
            for _ in 0..4 {
                synced(&mut a);
                synced(&mut b);
            }
            let on_b: Vec<_> = changes(&b)
                .into_iter()
                .filter(|c| c.kind == ChangeKind::Applied && c.reason != ChangeReason::Recovery)
                .collect();
            let on_a: Vec<_> = changes(&a)
                .into_iter()
                .filter(|c| c.kind == ChangeKind::Applied && c.reason != ChangeReason::Recovery)
                .collect();
            assert_eq!(on_a.len() + on_b.len(), 1, "{on_a:?} {on_b:?}");
            if let Some(r) = on_b.first() {
                assert_eq!(r.reason, ChangeReason::Choice(Some("a".into())), "{r:?}");
            }
            if let Some(r) = on_a.first() {
                assert_eq!(r.reason, ChangeReason::Choice(None), "{r:?}");
            }
        }

        #[test]
        fn a_peers_rollback_is_recorded_on_the_receiving_computer_as_a_rollback() {
            use wow_sync_core::hash::package_hash;
            let lab = Lab::new();
            let (mut a, mut b, c, g) = alpha_v2_published(&lab);
            let v1 = alpha_folder(&lab, &g, &c);
            let v1_hash = package_hash(&[(v1.key.clone(), v1.tree.unwrap())]);
            synced(&mut a);
            a.svc
                .command(Command::Rollback {
                    package: v1.key.clone(),
                    recovered: v1_hash,
                })
                .unwrap();
            synced(&mut a);
            let mine = changes(&a);
            let own = mine
                .iter()
                .find(|c| c.kind == ChangeKind::RolledBack)
                .unwrap_or_else(|| panic!("{mine:?}"));
            assert!(matches!(own.new_version, VersionLabel::Toc(_)), "{own:?}");
            for _ in 0..3 {
                synced(&mut b);
            }
            let all = changes(&b);
            let got = all
                .iter()
                .find(|c| {
                    c.new_version == VersionLabel::Toc("2.0".into())
                        && c.old_version == VersionLabel::Toc("3.0".into())
                })
                .unwrap_or_else(|| panic!("{all:?} {:?}", b.read("Alpha/main.lua")));
            assert_eq!(got.reason, ChangeReason::Rollback, "{got:?}");
        }

        #[test]
        fn a_restore_is_recorded_as_a_recovery() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            let mut b = lab.pc("b");
            a.addon("Alpha", "alpha-v1");
            let g = a.create();
            synced(&mut a);
            b.join(&g);
            synced(&mut b);
            a_removes_alpha(&mut a, &g);
            let (ac, _) = pc_ids(&a);
            let removal = removal_hash(&lab, &g, &ac);
            synced(&mut b);
            b.svc.command(Command::RestoreRemoved { removal }).unwrap();
            for _ in 0..3 {
                synced(&mut b);
                synced(&mut a);
            }
            let all = changes(&b);
            let r = all
                .iter()
                .find(|c| c.kind == ChangeKind::RecoveryRestored)
                .unwrap_or_else(|| panic!("{all:?}"));
            assert_eq!(r.reason, ChangeReason::Recovery);
            assert_eq!(r.source, ChangeSource::ThisComputer);
            assert_eq!(r.old_version, VersionLabel::New);
            assert_eq!(r.files.added, 2, "{r:?}");
            assert!(
                matches!(&r.new_version, VersionLabel::ContentId(s) if s.len() == 8),
                "{r:?}"
            );
        }

        #[test]
        fn a_masters_settings_change_is_recorded_on_both_computers_without_paths() {
            let lab = Lab::new();
            let (mut a, mut b, _g) = settings_pair(&lab, true, Some(1 << 20));
            write_wtf(&a, "Alpha", "AlphaDB = { v = 1 }\n");
            synced(&mut a);
            write_wtf(&a, "Alpha", "AlphaDB = { v = 2 }\n");
            synced(&mut a);
            synced(&mut b);
            let on_b = changes(&b);
            let got = on_b
                .iter()
                .find(|c| c.kind == ChangeKind::SettingsApplied)
                .unwrap_or_else(|| panic!("{on_b:?}"));
            assert_eq!(got.source, ChangeSource::Computer("a".into()));
            assert_eq!(got.reason, ChangeReason::SettingsMaster);
            assert_eq!(got.name, "Alpha settings");
            assert_eq!(got.files.added, 1);
            assert!(matches!(&got.new_version, VersionLabel::ContentId(s) if s.len() == 8));
            assert_eq!(got.old_version, VersionLabel::New);
            assert_eq!(got.byte_delta, "AlphaDB = { v = 2 }\n".len() as i64);
            assert_eq!(got.identity.as_deref(), Some("account-wide"));
            assert!(!format!("{got:?}").contains("ACC"));
            let on_a = changes(&a);
            let sent = on_a
                .iter()
                .find(|c| c.kind == ChangeKind::SettingsPublished)
                .unwrap_or_else(|| panic!("{on_a:?}"));
            assert_eq!(sent.source, ChangeSource::ThisComputer);
            let all: Vec<_> = on_a
                .iter()
                .filter(|c| c.kind == ChangeKind::SettingsPublished)
                .collect();
            assert_eq!(all.len(), 2, "the first share and the change: {on_a:?}");
            for c in &all {
                assert!(
                    matches!(&c.new_version, VersionLabel::ContentId(s) if s.len() == 8),
                    "{c:?}"
                );
                assert_eq!(c.identity.as_deref(), Some("account-wide"));
            }
            assert!(all.iter().any(|c| c.old_version == VersionLabel::New));
            assert!(all.iter().any(|c| {
                matches!(&c.old_version, VersionLabel::ContentId(s) if s.len() == 8)
                    && c.byte_delta == 0
            }));
        }

        #[test]
        fn a_removal_streams_removing_and_a_publish_of_nothing_new_is_not_listed() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            let mut b = lab.pc("b");
            let g = a.create();
            b.addon("Alpha", "alpha-v1");
            b.join(&g);
            synced(&mut b);
            a_removes_alpha(&mut a, &g);
            let rx = b.svc.progress_hub().subscribe();
            synced(&mut b);
            let events: Vec<_> = rx.try_iter().collect();
            assert!(
                events.iter().any(|e| e.phase == Phase::Removing),
                "{events:?}"
            );
            assert!(events.iter().all(|e| e.phase != Phase::Installing));
            let st = LocalState::open_at(&b.cfg.state_dir).unwrap();
            crate::control::log_activity(&st, "published", None, "published seq 3 (0 new objects)");
            crate::control::log_activity(&st, "published", None, "published seq 4 (2 new objects)");
            let h = b.svc.history(50);
            let text = |t: &str| {
                h.iter()
                    .any(|i| matches!(i, HistoryItem::Legacy(e) if e.text == t))
            };
            assert!(!text("published seq 3 (0 new objects)"));
            assert!(text("published seq 4 (2 new objects)"));
        }

        #[test]
        fn a_published_update_counts_only_the_files_that_differ() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            a.addon("Alpha", "alpha-v1");
            let dir = a.install.addons.join("Alpha");
            for i in 0..6 {
                std::fs::write(dir.join(format!("f{i}.lua")), format!("v1 {i}")).unwrap();
            }
            a.install.set_old_mtimes();
            a.create();
            synced(&mut a);
            std::fs::write(dir.join("f2.lua"), "edited").unwrap();
            std::fs::write(dir.join("new.lua"), "new").unwrap();
            std::fs::remove_file(dir.join("f5.lua")).unwrap();
            a.install.set_old_mtimes();
            let r = synced(&mut a);
            assert!(r.published.is_some(), "{r:?}");
            let all = changes(&a);
            let rec = all
                .iter()
                .find(|c| c.kind == ChangeKind::Published)
                .unwrap_or_else(|| panic!("{all:?}"));
            assert_eq!(
                (rec.files.added, rec.files.changed, rec.files.removed),
                (1, 1, 1),
                "{rec:?}"
            );
            assert_eq!(
                rec.paths,
                vec!["f2.lua".to_string(), "f5.lua".into(), "new.lua".into()]
            );
        }

        #[test]
        fn an_uninstall_leaves_a_removed_record_on_the_computer_that_made_it() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            let g = a.create();
            a_removes_alpha(&mut a, &g);
            let all = changes(&a);
            let removed: Vec<_> = all
                .iter()
                .filter(|c| c.kind == ChangeKind::Removed)
                .collect();
            assert_eq!(removed.len(), 1, "{all:?}");
            assert_eq!(removed[0].source, ChangeSource::ThisComputer);
            assert_eq!(removed[0].new_version, VersionLabel::Removed);
            assert_eq!(removed[0].reason, ChangeReason::Removal);
            assert!(removed[0].files.removed >= 1);
            let again = synced(&mut a);
            assert!(!again.crashed);
            let n = changes(&a)
                .iter()
                .filter(|c| c.kind == ChangeKind::Removed)
                .count();
            assert_eq!(n, 1, "recorded once");
        }

        #[test]
        fn a_removal_applied_from_a_peer_is_not_also_recorded_as_a_local_removal() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            let mut b = lab.pc("b");
            let g = a.create();
            b.addon("Alpha", "alpha-v1");
            b.join(&g);
            synced(&mut b);
            a_removes_alpha(&mut a, &g);
            synced(&mut b);
            synced(&mut b);
            let removed: Vec<_> = changes(&b)
                .into_iter()
                .filter(|c| c.kind == ChangeKind::Removed)
                .collect();
            assert_eq!(removed.len(), 1, "{removed:?}");
            assert_eq!(removed[0].source, ChangeSource::Computer("a".into()));
        }

        #[test]
        fn the_checking_files_counter_starts_again_each_pass() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            a.addon("Alpha", "alpha-v1");
            a.addon("Beta", "beta-v1");
            a.create();
            let ctl = wow_sync_adapters::fs_scan::ScanCtl::default();
            a.svc.set_read_ctl(ctl.clone());
            let mut counts = Vec::new();
            for _ in 0..4 {
                synced(&mut a);
                counts.push(ctl.files.load(std::sync::atomic::Ordering::Relaxed));
            }
            assert!(counts[1] > 0, "{counts:?}");
            assert_eq!(counts[2], counts[1], "{counts:?}");
            assert_eq!(counts[3], counts[1], "{counts:?}");
        }

        #[test]
        fn a_crash_recovery_that_finishes_an_install_leaves_a_record() {
            let mut recorded = false;
            for k in 0..80 {
                let lab = Lab::new();
                let mut a = lab.pc("a");
                let mut b = lab.pc("b");
                a.addon("Alpha", "alpha-v1");
                let g = a.create();
                synced(&mut a);
                b.join(&g);
                b.svc.inject_crash_after(Some(k));
                let crashed = synced(&mut b).crashed;
                b.restart();
                synced(&mut b);
                recorded |= changes(&b)
                    .iter()
                    .any(|c| c.reason == ChangeReason::Recovery);
                if !crashed {
                    break;
                }
            }
            assert!(recorded, "no crash point ended in a recovered install");
        }

        #[test]
        fn a_recovered_peer_install_keeps_its_source_and_is_not_a_local_publish() {
            let mut checked = false;
            for k in 0..80 {
                let lab = Lab::new();
                let mut a = lab.pc("a");
                let mut b = lab.pc("b");
                a.addon("Alpha", "alpha-v1");
                let g = a.create();
                synced(&mut a);
                b.join(&g);
                b.svc.inject_crash_after(Some(k));
                let crashed = synced(&mut b).crashed;
                b.restart();
                synced(&mut b);
                let all = changes(&b);
                if let Some(r) = all.iter().find(|c| c.reason == ChangeReason::Recovery) {
                    checked = true;
                    assert_eq!(r.source, ChangeSource::Computer("a".into()), "k={k}");
                    assert!(
                        all.iter().all(|c| c.kind != ChangeKind::Published),
                        "k={k}: {all:?}"
                    );
                }
                if !crashed {
                    break;
                }
            }
            assert!(checked, "no crash point ended in a recovered install");
        }

        #[test]
        fn a_recovered_install_without_a_saved_source_is_not_called_this_computer() {
            let mut checked = false;
            for k in 0..80 {
                let lab = Lab::new();
                let mut a = lab.pc("a");
                let mut b = lab.pc("b");
                a.addon("Alpha", "alpha-v1");
                let g = a.create();
                synced(&mut a);
                b.join(&g);
                b.svc.inject_crash_after(Some(k));
                let crashed = synced(&mut b).crashed;
                let _ = std::fs::remove_file(b.cfg.state_dir.join("apply-sources.json"));
                b.restart();
                synced(&mut b);
                if let Some(r) = changes(&b)
                    .iter()
                    .find(|c| c.reason == ChangeReason::Recovery)
                {
                    checked = true;
                    assert_ne!(r.source, ChangeSource::ThisComputer, "k={k}");
                }
                if !crashed {
                    break;
                }
            }
            assert!(checked, "no crash point ended in a recovered install");
        }

        #[test]
        fn older_audit_lines_still_appear_in_the_history() {
            let lab = Lab::new();
            let mut a = lab.pc("a");
            a.create();
            a.addon("Alpha", "alpha-v1");
            synced(&mut a);
            let st = LocalState::open_at(&a.cfg.state_dir).unwrap();
            crate::control::log_activity(&st, "recovered", None, "an older line");
            let h = a.svc.history(50);
            assert!(h
                .iter()
                .any(|i| matches!(i, HistoryItem::Legacy(e) if e.text == "an older line")));
        }
    }
}
