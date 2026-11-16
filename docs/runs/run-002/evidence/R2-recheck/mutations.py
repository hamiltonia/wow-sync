"""Bounded negative controls for changed core areas; no test edits or commits."""
import json
from pathlib import Path
import re
import subprocess

base = Path(__file__).parent
root = base / "clean"
cases = [
    ("01-enrollment-digest", "variant.rs",
     "|| stored.mapping_id != mapping.id()",
     "|| stored.mapping_id != mapping.id()\n        || matches!(a, LayoutAssessment::Supported { digest, .. } if *digest != stored.digest)",
     "variant::tests::later_folders_do_not_invalidate_enrollment"),
    ("02-blocked-publication", "plan.rs",
     "for f in inv.folders.iter().filter(|f| f.transfer_eligible()) {",
     "for f in inv.folders.iter().filter(|f| f.status == FolderStatus::Ok) {",
     "plan::tests::blocked_folders_are_never_published"),
    ("03-cross-product-join", "variant.rs",
     "if group_product == v.product() {",
     "if true || group_product == v.product() {",
     "variant::tests::join_requires_equal_product"),
    ("04-drop-apply-report-publication", "plan.rs",
     "if has_apply_report {\n        return true;",
     "if has_apply_report {\n        return false;",
     "plan::tests::publish_needed_decisions"),
    ("05-partial-candidate", "target.rs",
     "if !missing.is_empty() {\n        return Err((",
     "if false && !missing.is_empty() {\n        return Err((",
     "target::tests::partial_suite_candidate_needs_decision"),
    ("06-timestamp-winner", "target.rs",
     "let dominator = list.iter().find(|c| {",
     """let newest_id = peers.iter().max_by_key(|(_, _, state)| state.written_unix_ms).map(|p| p.0);
        let dominator = list.iter().find(|c| {
            if newest_id.is_some_and(|id| c.holders.iter().any(|h| &h.0 == id)) { return true; }""",
     "target::tests::record_timestamps_cannot_influence_target"),
    ("07-false-confirmation", "status.rs",
     "let Some(rep) = &state.apply_report else {\n            continue;\n        };",
     """let Some(rep) = &state.apply_report else {
            for key in target.packages.keys() { out.entry(key.clone()).or_default().push(m.computer.clone()); }
            continue;
        };""",
     "status::tests::published_is_not_confirmed"),
    ("08-omit-ambiguity-sources", "explain.rs",
     "if !held.is_empty() {", "if false && !held.is_empty() {",
     "status::tests::needs_decision_shows_evidence"),
    ("09-false-game-cause", "status.rs",
     "if input.game_running {", "if true || input.game_running {",
     "status::tests::interruption_blames_the_game_only_when_it_is_running"),
    ("10-generic-wait-headline", "status.rs",
     "Headline::Waiting if v.blocks.contains(&PlanBlock::GameRunning) =>",
     "Headline::Waiting if false && v.blocks.contains(&PlanBlock::GameRunning) =>",
     "status::tests::headline_names_running_game_while_waiting"),
]
results = []
for name, file, old, new, test in cases:
    path = root / "crates" / "wow-sync-core" / "src" / file
    before = path.read_bytes()
    text = before.decode().replace("\r\n", "\n")
    assert text.count(old) == 1, (name, text.count(old))
    try:
        path.write_text(text.replace(old, new), newline="\n")
        p = subprocess.run(["cargo", "test", "-p", "wow-sync-core", test, "--", "--exact", "--nocapture"],
                           cwd=root, capture_output=True, text=True, encoding="utf-8", errors="replace")
        output = p.stdout + p.stderr
        (base / f"{name}.log").write_text(output)
        failed = sorted(set(re.findall(r"^test (\S+) \.\.\. FAILED", output, re.M)))
        result = dict(mutation=name, test=test, exit=p.returncode, failing_tests=failed,
                      result="CAUGHT" if failed else "BUILD_ERROR" if "error[E" in output else "SURVIVED")
        results.append(result)
        print(json.dumps(result), flush=True)
    finally:
        path.write_bytes(before)
    (base / "negative-controls.json").write_text(json.dumps(results, indent=2))
