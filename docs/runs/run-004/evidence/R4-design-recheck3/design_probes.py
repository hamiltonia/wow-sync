"""Synthetic written-rule probes, NOT product tests or acceptance evidence."""

from dataclasses import dataclass
from functools import lru_cache
from itertools import permutations
import json


UNKNOWN = object()
ABSENT = object()


@dataclass
class Copy:
    scope: frozenset
    trees: dict
    source: bool = False

    def content(self, key):
        if not self.source and key not in self.scope:
            return UNKNOWN
        return self.trees.get(key, ABSENT)


def match_over(left, right, keys):
    pairs = [(left.content(k), right.content(k)) for k in keys]
    if any(a is not UNKNOWN and b is not UNKNOWN and a != b for a, b in pairs):
        return "Different"
    if any(a is UNKNOWN or b is UNKNOWN for a, b in pairs):
        return "Unknown"
    return "Same"


def written_match(left, right):
    return match_over(left, right, left.scope | right.scope)


def scoped_source_match(entry, source):
    """Proposed minimal repair, deliberately NOT attributed to current text."""
    return match_over(entry, source, entry.scope)


def release_probes():
    # A finite subset is enough to expose the universal-source-scope problem.
    universe = frozenset(("core", "extra", "unrelated"))
    rejected = Copy(frozenset(("core", "extra")), {"core": "U", "extra": "BAD"})
    recovered = Copy(rejected.scope, {"core": "U"})
    bad_source = Copy(universe, dict(rejected.trees), True)
    good_source = Copy(universe, dict(recovered.trees), True)
    assert written_match(rejected, recovered) == "Different"
    assert written_match(rejected, good_source) == "Different"
    assert written_match(rejected, bad_source) == "Unknown"
    bad_source.trees["unrelated"] = "Z"
    assert written_match(rejected, bad_source) == "Unknown"
    bad_source.trees["extra"] = UNKNOWN
    assert written_match(rejected, bad_source) == "Unknown"
    bad_source.trees["extra"] = "BAD"
    assert scoped_source_match(rejected, bad_source) == "Same"
    assert scoped_source_match(rejected, good_source) == "Different"
    dropped_scope = frozenset(("core", "old"))
    dropped_bad = Copy(dropped_scope, {"core": "U"})
    dropped_old = Copy(dropped_scope, {"core": "U", "old": "O"})
    assert written_match(dropped_bad, dropped_old) == "Different"
    old_source = Copy(universe | dropped_scope, dict(dropped_old.trees), True)
    assert written_match(dropped_bad, old_source) == "Different"

    old_scope = frozenset(("a", "b"))
    expanded = frozenset(("a", "b", "c"))
    declined = Copy(old_scope, {"a": "3", "b": "U"})
    kept = Copy(expanded, {"a": "2", "b": "U", "c": "C"}, True)
    replay = Copy(expanded, {"a": "3", "b": "U", "c": "C"}, True)
    assert written_match(declined, kept) == "Different"
    assert written_match(declined, replay) == "Unknown"
    assert scoped_source_match(declined, kept) == "Different"
    assert scoped_source_match(declined, replay) == "Same"
    # An unconsidered release that agrees on a,b but has c cannot settle by
    # downloading more data: the immutable entry does not describe c.
    assert declined.content("c") is UNKNOWN
    assert written_match(declined, replay) == "Unknown"
    print("PASS original R4D-23 added/dropped-content distinction")
    print("DESIGN FAIL R4D-28: exact replay remains Unknown; immutable scope gap")
    print("PASS proposed recorded-scope comparison repairs these source cases")


RECORDS = {
    "R": {"issuer": "A", "subject": "B", "cutoff": 10, "deps": ["S"]},
    "S": {"issuer": "C", "subject": "A", "cutoff": 20, "deps": ["T"]},
    "T": {"issuer": "D", "subject": "C", "cutoff": 30, "deps": ["U"]},
    "U": {"issuer": "E", "subject": "D", "cutoff": 40, "deps": []},
}
MEMBERS = ("A", "B", "C", "D", "E")
SEQS = {"A": 21, "B": 11, "C": 31, "D": 41, "E": 1}
EXPECTED_PREFIXES = {
    # Independently tabulated: record order R,S,T,U; member order A,B,C,D,E.
    # "." absent, U unresolved, E effective, V void; N no, Y retired.
    "": ("....", "NNNNN"),
    "R": ("U...", "NUNNN"),
    "S": (".U..", "UNNNN"),
    "T": ("..U.", "NNUNN"),
    "U": ("...E", "NNNYN"),
    "RS": ("UU..", "UUNNN"),
    "RT": ("U.U.", "NUUNN"),
    "RU": ("U..E", "NUNYN"),
    "ST": (".UU.", "UNUNN"),
    "SU": (".U.E", "UNNYN"),
    "TU": ("..VE", "NNNYN"),
    "RST": ("UUU.", "UUUNN"),
    "RSU": ("UU.E", "UUNYN"),
    "RTU": ("U.VE", "NUNYN"),
    "STU": (".EVE", "YNNYN"),
    "RSTU": ("VEVE", "YNNYN"),
}


def retirement_view(ledger):
    @lru_cache(None)
    def status(name):
        if name not in ledger:
            return "Unresolved"
        row = ledger[name]
        if "invalid" in row:
            return row["invalid"]
        statuses = [status(ref) for ref in row["deps"]]
        if "Effective" in statuses:
            return "Void"
        if any(s in ("Unresolved", "not_a_member") for s in statuses):
            return "Unresolved"
        return "Effective"

    statuses = {name: status(name) for name in ledger}
    members = {}
    for member in MEMBERS:
        rows = [r for r in ledger if ledger[r].get("subject") == member]
        effective = [ledger[r]["cutoff"] for r in rows if status(r) == "Effective"]
        unresolved = [
            ledger[r]["cutoff"] for r in rows if status(r) == "Unresolved"
        ]
        retired = "Yes" if effective else "Unresolved" if unresolved else "No"
        cutoff = min(effective) if effective else None
        holdcut = min(effective + unresolved) if effective + unresolved else None
        seq_status = "admitted"
        if retired == "Yes" and SEQS[member] > cutoff:
            seq_status = "invalid"
        elif retired == "Unresolved" and SEQS[member] > holdcut:
            seq_status = "held"
        members[member] = {
            "retired": retired,
            "cutoff": cutoff,
            "holdcut": holdcut,
            "seq_status": seq_status,
            "awaitable": retired != "Yes",
            "master_retired_prompt": retired == "Yes",
            "own_publish_paused": retired != "No",
        }
    return statuses, members


def retirement_probes():
    expected_status = {"R": "Void", "S": "Effective", "T": "Void", "U": "Effective"}
    expected_members = {"A": "Yes", "B": "No", "C": "No", "D": "Yes", "E": "No"}
    snapshots = 0
    for order in permutations(RECORDS):
        ledger = {}
        settled = {}
        ever_retired = set()
        # These are model pending queues, not the actual admission watermark.
        pending = set()
        watermarks = {m: 0 for m in MEMBERS}
        for step in range(5):
            if step:
                name = order[step - 1]
                ledger[name] = RECORDS[name]
            statuses, members = retirement_view(ledger)
            expected_records, expected_retired = EXPECTED_PREFIXES["".join(sorted(ledger))]
            assert "".join(statuses[r][0] if r in statuses else "." for r in RECORDS) == expected_records
            assert "".join(members[m]["retired"][0] for m in MEMBERS) == expected_retired
            restarted = retirement_view(json.loads(json.dumps(ledger, sort_keys=True)))
            assert (statuses, members) == restarted
            assert retirement_view(dict(ledger, **ledger)) == restarted
            assert members["B"]["retired"] != "Yes"
            assert members["C"]["retired"] != "Yes"
            for name, result in statuses.items():
                if name in settled:
                    assert result == settled[name]
                if result in ("Effective", "Void"):
                    settled[name] = result
            for member, view in members.items():
                if member in ever_retired:
                    assert view["retired"] == "Yes"
                if view["retired"] == "Yes":
                    ever_retired.add(member)
                previous = watermarks[member]
                if view["seq_status"] == "held":
                    pending.add(member)
                    assert view["awaitable"]
                    assert not view["master_retired_prompt"]
                    assert view["own_publish_paused"]
                    assert watermarks[member] == previous
                elif view["seq_status"] == "admitted":
                    pending.discard(member)
                    watermarks[member] = SEQS[member]
                else:
                    pending.discard(member)
            snapshots += 1
        assert statuses == expected_status
        assert {m: v["retired"] for m, v in members.items()} == expected_members
        assert not pending
        assert watermarks["B"] == SEQS["B"]
        assert watermarks["C"] == SEQS["C"]

    partial = {"R": RECORDS["R"], "S": RECORDS["S"]}
    statuses, members = retirement_view(partial)
    assert statuses == {"R": "Unresolved", "S": "Unresolved"}
    assert members["B"]["holdcut"] == 10
    assert members["A"]["holdcut"] == 20
    # A separate historical-member fact is allowed to arrive later.
    missing_member = {"R": RECORDS["R"], "S": {"invalid": "not_a_member"}}
    assert retirement_view(missing_member)[0]["R"] == "Unresolved"
    missing_member["S"] = {"issuer": "C", "subject": "A", "cutoff": 20, "deps": []}
    assert retirement_view(missing_member)[0]["R"] == "Void"
    bad_dependency = {"R": RECORDS["R"], "S": {"invalid": "wrong_group"}}
    assert retirement_view(bad_dependency)[0]["R"] == "Effective"
    print(f"PASS R4D-25: 24 orders, {snapshots} prefix snapshots + restart/duplicates")
    print("PASS not_a_member stays unresolved; permanent invalid dependency settles")


def local_guidance_probes():
    copies_of_recovered = []
    assert all(copy["rejected"] for copy in copies_of_recovered)
    assert not copies_of_recovered
    print("LOCAL GUIDANCE R4D-29: empty recovered-source set needs unavailable branch")

    current_policy_heads = [{"master": "B"}]
    auto_revision = {"issuer": "A", "witness_master": "A"}
    historically_valid = auto_revision["issuer"] == auto_revision["witness_master"]
    heads_agree = len({p["master"] for p in current_policy_heads}) == 1
    rewritten_apply_guard = all(
        p["master"] == auto_revision["issuer"] for p in current_policy_heads
    )
    assert historically_valid and heads_agree and not rewritten_apply_guard
    print("LOCAL GUIDANCE R4D-30: one reassigned policy is not PolicyConflict")

    heads = [(f"{i:02}", "A") for i in range(9)]
    witness = sorted(heads)[:8]
    assert len(witness) == 8
    assert all(master == "A" for _, master in witness)
    conflict_outside_witness = heads + [("99", "B")]
    assert not all(master == "A" for _, master in conflict_outside_witness)
    print("PASS bounded witness: 9 agreeing heads fit; tenth conflict is not ignored")

    def publish(clean, local_seq, activation_seq, non_strict=False):
        boundary = local_seq >= activation_seq if non_strict else local_seq > activation_seq
        return clean and boundary

    assert not publish(False, 6, 5)
    assert publish(True, 6, 5)  # Ignoring dirty hold breaks that fixture.
    assert not publish(True, 5, 5)
    assert publish(True, 5, 5, non_strict=True)
    assert publish(True, 6, 5)
    print("PASS separated dirty-epoch and strict-boundary controls (model only)")


if __name__ == "__main__":
    release_probes()
    retirement_probes()
    local_guidance_probes()
    print("Model completed; expected counterexample is NOT a product-test failure.")
