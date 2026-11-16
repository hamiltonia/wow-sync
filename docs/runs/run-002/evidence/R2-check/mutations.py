"""Independent negative controls. Only mutate clean-clone production files; restore in finally."""
import json
import os
from pathlib import Path
import re
import subprocess

BASE = Path(__file__).parent
ROOT = BASE / "clean"
CORE = "crates/wow-sync-core/src/"
AD = "crates/wow-sync-adapters/src/"
cases = []


def add(name, file, old, new, test, crate="wow-sync-core"):
    cases.append((name, file, old, new, test, crate))


add("01-cross-scheme", CORE + "version.rs",
    '_ => amb("different version schemes"),',
    '_ => VersionOrder::Greater,', "version::tests::cross_scheme_is_ambiguous")
add("02-channel", CORE + "version.rs",
    'return amb("different channels");',
    'return if prea.is_none() { VersionOrder::Greater } else { VersionOrder::Less };',
    "version::tests::prerelease_vs_release_is_ambiguous")
add("03-rename-gate", CORE + "exec.rs",
    """        if let Err(h) = gate(fx, &name) {
            return interrupted(h.why, h.block);
        }
        if let Err(e) = fx.rename(&s, &l)""",
    """        if let Err(e) = fx.rename(&s, &l)""",
    "exec::tests::activity_checked_before_every_live_rename")
add("04-rollback-on-launch", CORE + "exec.rs",
    """        if let Err(h) = gate(fx, &name) {
            return interrupted(h.why, h.block);
        }
        if let Err(e) = fx.rename(&s, &l)""",
    """        if let Err(h) = gate(fx, &name) {
            let _ = fx.rename(&r, &l);
            return interrupted(h.why, h.block);
        }
        if let Err(e) = fx.rename(&s, &l)""",
    "exec::tests::no_live_mutation_after_launch_mid_commit")
add("05-unverified-finalize", CORE + "exec.rs",
    "JournalPhase::Committed => verify_and_finish(fx, j),",
    "JournalPhase::Committed => finalize(fx, j),",
    "exec::tests::crash_before_verify_reverifies_on_restart")
add("06-destination-revalidation", CORE + "exec.rs",
    "let unchanged = match (&f.expected_live, &obs) {",
    "let unchanged = Some(obs.as_ref().map(|o| o.fingerprint)); /*\n match (&f.expected_live, &obs) {",
    "exec::tests::destination_changed_during_staging_replans")
add("07-ignore-pinned", CORE + "exec.rs",
    "!rp.pinned && (*op == j.op || !journals.contains(op.as_str()))",
    "(*op == j.op || !journals.contains(op.as_str()))",
    "exec::tests::pinned_recovery_points_exceed_two_visibly")
add("08-absence-as-removal", CORE + "target.rs",
    "out.packages.insert(pkg.key.clone(), target);",
    """if local_pv.is_some() && peer_invs.iter().any(|inv| package_version(&pkg, inv, input.local_client).is_none()) {
            out.effective.remove(&pkg.key);
            continue;
        }
        out.packages.insert(pkg.key.clone(), target);""",
    "target::tests::absent_package_is_not_removal")
add("09-partial-candidate", CORE + "target.rs",
    "if !missing.is_empty() {\n        return Err((",
    "if false && !missing.is_empty() {\n        return Err((",
    "target::tests::partial_suite_candidate_needs_decision")
add("10-timestamp-winner", CORE + "target.rs",
    "let dominator = list.iter().find(|c| {",
    """let newest_id = peers.iter().max_by_key(|(_, _, state)| state.written_unix_ms).map(|p| p.0);
        let dominator = list.iter().find(|c| {
            if newest_id.is_some_and(|id| c.holders.iter().any(|h| &h.0 == id)) { return true; }""",
    "target::tests::record_timestamps_cannot_influence_target")
add("11-fallback-selection", CORE + "admit.rs",
    "let latest = match newest(m, &o.files) {",
    """let mut usable_files = o.files.clone();
        if o.newest_bytes.as_ref().is_some_and(|b| b.as_ref().is_ok_and(|bytes| decode::<CodeState>(bytes, CodeState::LIMIT).is_err())) {
            if let Some((_, i)) = newest(m, &usable_files) { usable_files.remove(i); }
        }
        let latest = match newest(m, &usable_files) {""",
    "admit::tests::newest_malformed_no_fallback")
add("12-trust-existing-object", CORE + "exec.rs",
    "Ok(ObjectCheck::VerifiedOk) => continue,",
    "Ok(ObjectCheck::VerifiedOk | ObjectCheck::Corrupt) => continue,",
    "exec::tests::corrupt_existing_blob_replaced_before_state")
add("13-suffixed-toc", CORE + "toc.rs",
    "Some(Some(_)) => suffixed.push(c.file_name.clone()),",
    "Some(Some(_)) => plain.push(c),",
    "toc::tests::suffixed_tocs_unvalidated_for_recognized_client")
add("14-local-only-completeness", CORE + "target.rs",
    ".filter(|k| !have.contains(k))",
    ".filter(|k| input.local.folders.iter().any(|f| &f.key == *k) && !have.contains(k))",
    "target::tests::partial_candidate_not_installed_on_empty_peer")
add("15-remembered-recovery-state", CORE + "exec.rs",
    """let cfg = match (present(fx, &l), present(fx, &s), present(fx, &r)) {
            (Ok(a), Ok(b), Ok(c)) => classify_slots(install, a, b, c),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => return failed(&name, e),
        };""",
    "let cfg = SlotConfig::Done;",
    "exec::tests::crash_during_recovery_at_every_effect_is_reentrant")
add("16-cleanup-all-ops", CORE + "exec.rs",
    """    if let Err(e) = fx.discard_staging(&j.op) {""",
    """    for (n, _) in fx.read_journals().unwrap_or_default() {
        if let Ok(op) = OpId::parse(&op_name(&n)) {
            let _ = fx.discard_staging(&op);
            let _ = fx.remove_journal(&op);
        }
    }
    if let Err(e) = fx.discard_staging(&j.op) {""",
    "exec::tests::settling_one_op_keeps_other_ops_evidence")
add("17-size-mtime-trust", CORE + "exec.rs",
    "ObjectStat::Local { .. } => Ok(if fx.hash_object(rel)? == *expect {",
    "ObjectStat::Local { .. } => Ok(if true {",
    "exec::tests::same_size_mtime_corrupt_object_rehashed_and_replaced")
add("18-recall-on-open", AD + "fs_safe.rs",
    "ATTR_OFFLINE | ATTR_RECALL_ON_OPEN | ATTR_RECALL_ON_DATA_ACCESS",
    "ATTR_OFFLINE | ATTR_RECALL_ON_DATA_ACCESS",
    "fs_safe::tests::unavailable_attribute_combinations", "wow-sync-adapters")
add("19-synthetic-marker", CORE + "variant.rs",
    "if !marker_ok(&obs.synthetic_marker) {",
    "if false && !marker_ok(&obs.synthetic_marker) {",
    "variant::tests::install_without_synthetic_marker_refused")
add("20-unsupported-consent", CORE + "variant.rs",
    """        return Err(RefuseReason::Layout(r.clone()));""",
    """        if c.is_some_and(|c| c.identity && c.scope) {
            return confirm_variant(v, &LayoutAssessment::Supported {
                mapping: TocMapping::ValidatedPlain, eligible: Vec::new(), blocked: Vec::new(), digest: c.unwrap().digest,
            }, c);
        }
        return Err(RefuseReason::Layout(r.clone()));""",
    "variant::tests::consent_alone_does_not_confirm")
add("21-mount-cloud", AD + "fs_safe.rs",
    "if tag & 0xFFFF_0FFF == TAG_CLOUD_FAMILY {",
    "if tag == TAG_MOUNT_POINT || tag & 0xFFFF_0FFF == TAG_CLOUD_FAMILY {",
    "fs_safe::tests::classify_tag_cloud_vs_link_vs_other", "wow-sync-adapters")
add("22-inaccessible-client", AD + "winproc.rs",
    "if is_possible_client(&e.exe_name, variant_exes) {",
    "if false && is_possible_client(&e.exe_name, variant_exes) {",
    "winproc::tests::inaccessible_possible_client_is_unknown", "wow-sync-adapters")
add("23-member-is-confirmed", CORE + "status.rs",
    """        let Some(rep) = &state.apply_report else {
            continue;
        };""",
    """        let Some(rep) = &state.apply_report else {
            for key in target.packages.keys() { out.entry(key.clone()).or_default().push(m.computer.clone()); }
            continue;
        };""",
    "status::tests::published_is_not_confirmed")
add("24-state-before-objects", CORE + "exec.rs",
    "let mut uploaded = 0;",
    """let mut uploaded = 0;
    if let Err(e) = fx.write_record(&plan.state_path, &encode(&plan.state)) {
        return PublishOutcome::Failed(e.to_string());
    }""",
    "exec::tests::publish_failure_leaves_no_state_record")

results = json.loads((BASE / "negative-controls.json").read_text()) if (BASE / "negative-controls.json").exists() else []
for name, rel, old, new, test, crate in cases:
    if any(r["mutation"] == name for r in results):
        continue
    path = ROOT / rel
    before = path.read_bytes()
    text = before.decode().replace("\r\n", "\n")
    if text.count(old) != 1:
        raise RuntimeError(f"{name}: expected unique anchor, got {text.count(old)}")
    mutated = text.replace(old, new)
    if name == "06-destination-revalidation":
        anchor = "        match unchanged {"
        mutated = mutated.replace(anchor, "        */\n" + anchor, 1)
    path.write_text(mutated, newline="\n")
    try:
        p = subprocess.run(["cargo", "test", "-p", crate, test, "--", "--exact", "--nocapture"],
                           cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
        output = p.stdout + p.stderr
        (BASE / f"{name}.log").write_text(output, encoding="utf-8")
        failed = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED", output, re.M)))
        result = dict(mutation=name, test=test, exit=p.returncode, failing_tests=failed,
                      result="CAUGHT" if failed else "BUILD_ERROR" if "error[E" in output else "SURVIVED")
        results.append(result)
        print(json.dumps(result), flush=True)
    finally:
        path.write_bytes(before)
    (BASE / "negative-controls.json").write_text(json.dumps(results, indent=2))
