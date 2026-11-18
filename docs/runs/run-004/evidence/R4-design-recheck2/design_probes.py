"""Independent models of fix2's written rules, not product acceptance tests."""

import hashlib
import json
import unittest


def stored(value):
    return (json.dumps(value, indent=2) + "\n").encode()


def matches(candidate, entry):
    common = candidate.keys() & entry.keys()
    return bool(common) and all(candidate[key] == entry[key] for key in common)


def retirement_effective(name, ledger):
    """Literal barrier: direct V-REF presence, followed by boolean voidness."""
    record = ledger.get(name)
    if record is None:
        return False
    if any(ref not in ledger for ref in record["known"]):
        return False
    return not any(retirement_effective(ref, ledger) for ref in record["known"])


def retired_members(ledger):
    return {entry["subject"] for name, entry in ledger.items()
            if retirement_effective(name, ledger)}


class Fix2Probes(unittest.TestCase):
    def test_original_large_known_and_mirror_only_examples(self):
        prefixes = [(f"{issuer:032x}", 1) for issuer in range(1024)]
        prefix_parts = [prefixes[i:i + 128] for i in range(0, len(prefixes), 128)]
        self.assertEqual(len([item for part in prefix_parts for item in part]), 1024)
        extras = [hashlib.sha256(str(i).encode()).hexdigest() for i in range(1024)]
        extra_parts = [extras[i:i + 128] for i in range(0, len(extras), 128)]
        self.assertEqual(set(extras), {item for part in extra_parts for item in part})
        print("REPAIRED original R4D-4: partitioned semantic prefix/extra coverage")

    def test_original_full_policy_can_use_parts(self):
        folders = [{"folder": f"addon{i:04}", "rule": "master",
                    "computer_id": "4" * 32, "enrollment_id": "5" * 32}
                   for i in range(5000)]
        objects = {}
        level = []
        for offset in range(0, len(folders), 256):
            part = {"schema": "wowsync.authority-part/1", "group_id": "1" * 32,
                    "of": "policy_folders", "items": folders[offset:offset + 256],
                    "more": []}
            raw = stored(part)
            self.assertLessEqual(len(raw), 60 * 1024)
            name = hashlib.sha256(raw).hexdigest()
            objects[name] = part
            level.append(name)
        while len(level) > 16:
            next_level = []
            for offset in range(0, len(level), 16):
                part = {"schema": "wowsync.authority-part/1", "group_id": "1" * 32,
                        "of": "policy_folders", "items": [],
                        "more": level[offset:offset + 16]}
                name = hashlib.sha256(stored(part)).hexdigest()
                objects[name] = part
                next_level.append(name)
            level = next_level
        root = {"schema": "wowsync.settings-policy/1", "group_id": "1" * 32,
                "issuer": {"computer_id": "2" * 32, "enrollment_id": "3" * 32,
                           "issued_at_seq": 1},
                "master": {"computer_id": "2" * 32, "enrollment_id": "3" * 32},
                "folders": [], "folders_parts": level,
                "known": {"seen": [], "extra": [], "parts": []}, "written_unix_ms": 1}
        self.assertLessEqual(len(stored(root)), 64 * 1024)

        def flatten(name):
            node = objects[name]
            return node["items"] + [item for child in node["more"]
                                    for item in flatten(child)]

        self.assertEqual(folders, [item for name in level for item in flatten(name)])
        print(f"REPAIRED full policy: 5,000 exceptions, {len(objects)} model parts")

    def test_original_absent_ack_bypass_is_guarded(self):
        code_present = False
        cleanup_required = True
        ack_at_3b = not code_present and not cleanup_required
        self.assertFalse(ack_at_3b)
        print("REPAIRED R4D-5: cleanup_required blocks the step-3b absent ack")

    def test_original_settings_equality_and_invalid_basis_are_protected(self):
        def target(basis, valid_basis, heads, live_equals_head):
            if not basis:
                return "FirstAdoption"
            if not valid_basis:
                return "InSync" if live_equals_head else "BasisNotShared"
            if valid_basis == heads:
                return "InSync" if live_equals_head else "LocalDifferent"
            return "LaterCase"

        self.assertEqual(target({"h"}, {"h"}, {"h"}, False), "LocalDifferent")
        self.assertEqual(target({"invalid"}, set(), {"h"}, False), "BasisNotShared")
        self.assertEqual(target(set(), set(), {"h"}, False), "FirstAdoption")
        print("REPAIRED R4D-8/18: unchanged head, invalidated basis, first adoption")

    def test_original_epoch_counterexample_is_guarded(self):
        authority_since = 5
        activation_seq = 6
        old_comparison_seq = 5
        epoch_clean = False  # Activation found the preexisting experiment.
        self.assertFalse(old_comparison_seq > authority_since)
        self.assertFalse(epoch_clean and activation_seq > authority_since)
        print("REPAIRED R4D-19: strict boundary plus dirty-activation hold")

    def test_original_single_folder_keep_resume_semantics(self):
        considered = {"V2", "V3"}
        kept = "V2"
        exclude = considered - {kept}
        self.assertEqual(exclude, {"V3"})
        self.assertNotIn("V4", considered)
        target_while_kept = kept
        self.assertEqual(target_while_kept, "V2")  # New V4 requires explicit choice.
        resume_exclude = exclude - set()
        self.assertEqual({"V2", "V3"} - resume_exclude, {"V2"})
        self.assertEqual({"V2", "V3", "V4"} - resume_exclude, {"V2", "V4"})
        self.assertEqual(exclude - {"V3"}, set())
        print("REPAIRED original R4D-10 traces: keep, resume, replay, explicit allow")

    def test_original_partial_constraint_supersession_is_guarded(self):
        old_scope, new_scope = {"a", "b"}, {"a"}
        old_live = {f for f in old_scope if f not in new_scope}
        self.assertEqual(old_live, {"b"})
        print("REPAIRED R4D-9/21: a-only choice leaves old b constraint live")

    def test_original_retirement_predicate_and_cutoff(self):
        def v_ret(is_retirement, issuer_retired, seq, cutoff, void=False):
            return not void and (is_retirement or not issuer_retired or seq <= cutoff)

        self.assertFalse(v_ret(False, True, 11, 10))
        self.assertTrue(v_ret(False, True, 10, 10))
        self.assertFalse(v_ret(True, True, 11, 10, void=True))
        self.assertFalse(11 <= min([10, 12]))
        print("REPAIRED R4D-15/16 original truth table and higher-cutoff trace")

    def test_new_overlap_match_rejects_recovered_folder_subset(self):
        rejected = {"core": "unchanged", "extra": "bad"}
        recovered = {"core": "unchanged"}
        self.assertNotEqual(recovered, rejected)
        self.assertTrue(matches(recovered, rejected))
        print("NEW R4D-23: smaller recovered release matches larger rejected release")

    def test_new_exclusion_match_filters_an_unchanged_kept_component(self):
        kept = {"a": "2", "b": "unchanged"}
        declined = {"a": "3", "b": "unchanged"}
        self.assertFalse(matches(kept, declined))
        self.assertTrue(matches({"b": kept["b"]}, declined))
        print("NEW R4D-23: Q-50 resume excludes the kept copy's b projection")

    def test_new_projected_ordinary_selection_synthesizes_a_release(self):
        copies = [{"a": 3, "b": 1}, {"a": 2, "b": 2}]
        comparisons = {1 if copies[0][f] > copies[1][f] else -1 for f in copies[0]}
        self.assertEqual(comparisons, {-1, 1})  # MembersDisagree in compare_package.
        projected = {f: max(copy[f] for copy in copies) for f in copies[0]}
        self.assertEqual(projected, {"a": 3, "b": 2})
        self.assertNotIn(projected, copies)
        print("NEW R4D-24: per-folder newest creates a release no computer has")

    def test_new_partial_retirement_dag_can_reactivate_a_member(self):
        # All issuer/subject pairs are historical members; shapes/kinds are valid.
        complete = {
            "R": {"issuer": "A", "subject": "B", "known": ["S"]},
            "S": {"issuer": "C", "subject": "A", "known": ["T"]},
            "T": {"issuer": "D", "subject": "C", "known": ["U"]},
            "U": {"issuer": "E", "subject": "D", "known": []},
        }
        partial = {name: complete[name] for name in ("R", "S")}
        self.assertTrue(retirement_effective("R", partial))
        self.assertIn("B", retired_members(partial))
        self.assertFalse(retirement_effective("R", complete))
        self.assertNotIn("B", retired_members(complete))
        print("NEW R4D-25: R/S then T/U changes B from retired to active")


if __name__ == "__main__":
    unittest.main(verbosity=2)
