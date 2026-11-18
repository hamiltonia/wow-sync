"""Recheck models of the WRITTEN rules, not unimplemented product tests."""

import itertools
import json
import unittest


def original_merge(records):
    removals = {}
    boundaries = {}
    for kind, name, folders, covered in records:
        for folder in folders:
            if kind == "remove":
                removals.setdefault(folder, set()).add(name)
            else:
                boundaries.setdefault(folder, set()).update(covered)
    return {
        folder
        for folder, entries in removals.items()
        if entries - boundaries.get(folder, set())
    }


def revised_settings_target(basis, heads, live_equals_head=False):
    """settings 4.2: ancestor includes equality; rows evaluated in written order."""
    if live_equals_head:
        return "InSync"
    if all(all(b == head for b in basis) for head in heads):
        return "Yield"
    if basis == heads:
        return "LocalDifferent"
    if basis:
        return "BasisNotShared"
    return "FirstAdoption"


class RecheckProbes(unittest.TestCase):
    def test_removal_algebra_original_permutations(self):
        records = [
            ("remove", "r1", {"core", "data"}, set()),
            ("remove", "r2", {"other"}, set()),
            ("readd", "a1", {"core"}, {"r1"}),
            ("remove", "r3", {"core"}, set()),
        ]
        for order in itertools.permutations(records):
            self.assertEqual(original_merge(order), {"core", "data", "other"})
            self.assertEqual(original_merge(order + order), original_merge(order))
        print("CLOSED original algebra probe: 24 permutations and duplicate replays")

    def test_partial_ack_original_trace_is_repaired(self):
        ack_items = {("C", "r1", "data")}
        self.assertNotIn(("C", "r1", "core"), ack_items)
        print("CLOSED R4D-2: data-only ack leaves core owed")

    def test_ack_before_old_state_original_trace_is_repaired(self):
        admitted_seq = 10
        ack_issued_seq = 11
        self.assertFalse(admitted_seq >= ack_issued_seq)
        print("CLOSED R4D-3 original trace: seq-10 source cannot use seq-11 discharge")

    def test_absent_ack_schedule_does_not_test_pending_cleanup(self):
        in_audience, code_present, already_acked, own_readd = True, False, False, False
        cleanup_pending = True
        absent_ack_due = in_audience and not code_present and not already_acked and not own_readd
        self.assertTrue(cleanup_pending and absent_ack_due)
        print("PARTIAL R4D-5: written step-3b predicates permit ack before step-7 cleanup")

    def test_equal_heads_original_trace_is_repaired(self):
        graph = {"a": set(), "b": set(), "c": {"a", "b"}}
        parents = set().union(*graph.values())
        self.assertEqual(set(graph) - parents, {"c"})
        print("CLOSED R4D-7: parenting both equal heads leaves only c")

    def test_original_retirement_reactivation_is_repaired(self):
        retirements = [("A", "B"), ("C", "A")]
        for order in itertools.permutations(retirements):
            barrier = {subject for _, subject in order}
            self.assertEqual(barrier, {"A", "B"})
        print("CLOSED R4D-11 original trace: B stays retired in both orders")

    def test_known_compresses_only_the_authors_narrower_1024_example(self):
        records = [(issuer, seq) for issuer in range(8) for seq in range(1, 129)]
        seen = {issuer: 128 for issuer in range(8)}
        self.assertEqual(sum(seq <= seen[issuer] for issuer, seq in records), 1024)
        print("PASS narrowed example: 1,024 removals from 8 readable issuer indexes")

    def test_known_does_not_cover_the_original_general_counterexample(self):
        distinct_issuers = 1024
        maximum_covered = 512 + 128
        self.assertGreater(distinct_issuers, maximum_covered)
        self.assertEqual(distinct_issuers - maximum_covered, 384)
        # With unavailable issuer indexes, even 8 issuers need individual extras.
        self.assertGreater(1024, 128)
        print("OPEN R4D-4: 1,024 issuers exceed Known capacity by at least 384")
        print("OPEN R4D-4: mirror-only knowledge cannot use seen; extra allows only 128")

    def test_allowed_policy_body_is_not_fixed_size_below_the_record_cap(self):
        body = {
            "schema": "wowsync.settings-policy/1",
            "group_id": "1" * 32,
            "issuer": {"computer_id": "2" * 32, "enrollment_id": "3" * 32,
                       "issued_at_seq": 1},
            "master": {"computer_id": "2" * 32, "enrollment_id": "3" * 32},
            "folders": [
                {"folder": f"addon{i:04}", "rule": "master",
                 "computer_id": "4" * 32, "enrollment_id": "5" * 32}
                for i in range(512)
            ],
            "known": {"seen": [], "extra": []},
            "written_unix_ms": 1,
        }
        # records::encode uses pretty JSON plus a trailing LF; no non-ASCII here.
        encoded_size = len((json.dumps(body, indent=2) + "\n").encode())
        self.assertGreater(encoded_size, 64 * 1024)
        print(f"OPEN R4D-4: allowed 512-exception policy model is {encoded_size:,} bytes")

    def test_literal_v_ret_has_the_wrong_exemption(self):
        def table_rule(is_retirement, issuer_retired, seq, cutoff):
            return (not is_retirement) or (not issuer_retired) or seq <= cutoff

        self.assertTrue(table_rule(False, True, 11, 10))
        self.assertFalse(table_rule(True, True, 11, 10))
        print("NEW R4D-15: V-RET accepts post-cutoff ordinary authority, not retirement")

    def test_maximum_cutoff_reauthorizes_post_barrier_changes(self):
        ordinary_record_seq = 11
        self.assertFalse(ordinary_record_seq <= max([10]))
        self.assertTrue(ordinary_record_seq <= max([10, 12]))
        print("NEW R4D-16: later cutoff 12 reauthorizes retired issuer's seq-11 record")

    def test_settings_equal_basis_and_empty_basis_hit_yield_first(self):
        self.assertEqual(revised_settings_target({"h"}, {"h"}), "Yield")
        self.assertEqual(revised_settings_target(set(), {"h"}), "Yield")
        print("NEW R4D-18: unchanged shared head overwrites routine local differences")
        print("NEW R4D-18: empty/invalidated basis also hits yield before its own row")

    def test_authority_epoch_accepts_a_preexisting_startup_experiment(self):
        previous_capture_seq = 5
        # No capture happened while the app was closed and the experiment was made.
        current_local_seq_when_policy_learned = 5
        authority_since = current_local_seq_when_policy_learned
        compared_with_local_seq = previous_capture_seq
        self.assertTrue(compared_with_local_seq >= authority_since)
        print("NEW R4D-19: 5 >= 5 permits publishing an experiment made before reassignment")

    def test_partial_constraint_supersession_loses_the_other_folder(self):
        old = {"folders": {"a", "b"}, "target": "A1+B1"}
        new = {"folders": {"a"}, "covers_old": True}
        superseded = bool(old["folders"] & new["folders"]) and new["covers_old"]
        self.assertTrue(superseded)
        self.assertNotIn("b", new["folders"])
        print("NEW R4D-21: choice for a ends the whole old constraint, freeing b")

if __name__ == "__main__":
    unittest.main(verbosity=2)
