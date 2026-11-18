"""Executable traces of specified rules, NOT tests of product implementation."""

import itertools
import json
import unittest


def merged(records):
    removed = {}
    boundary = {}
    for kind, name, folders, supersedes in records:
        for folder in folders:
            if kind == "remove":
                removed.setdefault(folder, set()).add(name)
            else:
                boundary.setdefault(folder, set()).update(supersedes)
    return {
        folder
        for folder, removals in removed.items()
        if removals - boundary.get(folder, set())
    }


def owes(enrollment, folder, removal, acknowledgments):
    return (
        folder in removal["folders"]
        and enrollment in removal["audience"]
        and (enrollment, removal["name"]) not in acknowledgments
    )


def revision_heads(revisions):
    parents = {parent for revision in revisions.values() for parent in revision["parents"]}
    return {name for name in revisions if name not in parents}


class DesignProbes(unittest.TestCase):
    def test_removal_set_merge_is_permutation_invariant(self):
        records = [
            ("remove", "r1", {"core", "data"}, set()),
            ("remove", "r2", {"other"}, set()),
            ("readd", "a1", {"core"}, {"r1"}),
            ("remove", "r3", {"core"}, set()),
        ]
        for order in itertools.permutations(records):
            self.assertEqual(merged(order), {"core", "data", "other"})
            self.assertEqual(merged(order + order), merged(order))
        print("PASS: 24 delivery permutations and duplicate replays give the same M")

    def test_partial_removal_ack_releases_an_unremoved_stale_folder(self):
        removal = {"name": "r1", "folders": {"core", "data"}, "audience": {"C"}}
        records = [
            ("remove", "r1", removal["folders"], set()),
            ("readd", "a1", {"core"}, {"r1"}),
        ]
        live = {"core", "data"}
        self.assertEqual(merged(records), {"data"})
        # removal 4.1 level 3, then 5.2/5.6 finalize acks the removal record.
        live -= merged(records)
        acknowledgments = {("C", "r1")}
        self.assertEqual(live, {"core"})
        self.assertFalse(owes("C", "core", removal, acknowledgments))
        print("REPRODUCED: removing data acks r1 and releases untouched stale core")

    def test_ack_can_arrive_before_the_post_remove_state(self):
        removal = {"name": "r1", "folders": {"core"}, "audience": {"C"}}
        records = [
            ("remove", "r1", {"core"}, set()),
            ("readd", "a1", {"core"}, {"r1"}),
        ]
        admitted_state = {"seq": 10, "folders": {"core": "offline-v2"}}
        acknowledgments = {("C", "r1")}  # issued at seq 11, delivered first
        self.assertFalse(merged(records))
        self.assertFalse(owes("C", "core", removal, acknowledgments))
        self.assertIn("core", admitted_state["folders"])
        print("REPRODUCED: seq-11 ack admits seq-10 offline-v2 as a non-owing candidate")

    def test_equal_checkpoint_representative_creates_a_false_later_conflict(self):
        revisions = {
            "a": {"checkpoint": "same", "parents": set()},
            "b": {"checkpoint": "same", "parents": set()},
        }
        heads = revision_heads(revisions)
        representative = min(heads)
        self.assertEqual({revisions[h]["checkpoint"] for h in heads}, {"same"})
        # settings 4.2 records basis = representative; 4.1 auto parent = basis.
        revisions["c"] = {"checkpoint": "new", "parents": {representative}}
        heads = revision_heads(revisions)
        self.assertEqual(heads, {"b", "c"})
        self.assertEqual({revisions[h]["checkpoint"] for h in heads}, {"same", "new"})
        print("REPRODUCED: equal heads a/b followed by child of a creates b/c choice")

    def test_required_readd_references_exceed_the_record_limit(self):
        removal_hashes = [f"{number:064x}" for number in range(1024)]
        # This is only ONE required field, not even a full re-add record.
        encoded = json.dumps({"supersedes": removal_hashes}, separators=(",", ":")).encode()
        self.assertGreater(len(encoded), 64 * 1024)
        print(
            "REPRODUCED: 1,024 live removals require "
            f"{len(encoded)} bytes for supersedes alone; limit is 65,536"
        )

    def test_late_retirement_can_reactivate_a_previously_retired_enrollment(self):
        # A retires B at A seq 20. No retirement of A is initially known.
        retire_b_issued = 20
        retire_b_valid_before = True
        self.assertTrue(retire_b_valid_before)
        # C later retires A at cutoff 10, without knowing A's retirement of B.
        retire_a_cutoff = 10
        authority_known = set()
        retire_b_valid_after = not (
            retire_b_issued > retire_a_cutoff and "retire-b" not in authority_known
        )
        self.assertFalse(retire_b_valid_after)
        print("REPRODUCED: late cutoff invalidates B's only retirement; B is active again")


if __name__ == "__main__":
    unittest.main(verbosity=2)
