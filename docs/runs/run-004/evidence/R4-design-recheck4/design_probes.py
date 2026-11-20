"""Fix4 written-rule models, not product tests or acceptance evidence."""

from dataclasses import dataclass, field
import json


UNKNOWN = object()
ABSENT = object()


@dataclass(frozen=True)
class Entry:
    scope: frozenset
    trees: dict


@dataclass(frozen=True)
class Source:
    trees: dict
    admitted: bool = True


def content(copy, key):
    if isinstance(copy, Source) and not copy.admitted:
        return UNKNOWN
    if isinstance(copy, Entry) and key not in copy.scope:
        return UNKNOWN
    return copy.trees.get(key, ABSENT)


def compare_over(left, right, keys):
    pairs = [(content(left, k), content(right, k)) for k in keys]
    if any(a is not UNKNOWN and b is not UNKNOWN and a != b for a, b in pairs):
        return "Different"
    if any(a is UNKNOWN or b is UNKNOWN for a, b in pairs):
        return "Unknown"
    return "Same"


def same_release(left, right):
    if isinstance(left, Source) and isinstance(right, Entry):
        keys = right.scope
    elif isinstance(left, Entry) and isinstance(right, Source):
        keys = left.scope
    elif isinstance(left, Entry) and isinstance(right, Entry):
        keys = left.scope | right.scope
    else:
        raise TypeError("Fix4 specifies source/entry and entry/entry only")
    return compare_over(left, right, keys)


def release_cases():
    scope = frozenset(("core", "extra"))
    rejected = Entry(scope, {"core": "U", "extra": "BAD"})
    recovered = Entry(scope, {"core": "U"})
    cases = []
    for outside in ({}, {"z": "Z"}, {"z": UNKNOWN}):
        bad = Source(dict(rejected.trees, **outside))
        good = Source(dict(recovered.trees, **outside))
        cases.extend((
            (bad, rejected, "Same"),
            (good, recovered, "Same"),
            (good, rejected, "Different"),
            (bad, recovered, "Different"),
        ))

    # Regrouping changes the parent record, not the carried exclusion's scope.
    declined = Entry(frozenset(("a", "b")), {"a": "3", "b": "U"})
    cases.extend((
        (Source({"a": "3", "b": "U", "c": "C"}), declined, "Same"),
        (Source({"a": "2", "b": "U", "c": "C"}), declined, "Different"),
        (Source({"a": "4", "b": "U", "c": "C"}), declined, "Different"),
        (Source({"a": "3", "b": "U", "c": UNKNOWN}), declined, "Same"),
    ))

    dropped_scope = frozenset(("core", "old"))
    dropped_bad = Entry(dropped_scope, {"core": "U"})
    dropped_old = Entry(dropped_scope, {"core": "U", "old": "O"})
    cases.extend((
        (Source({"core": "U"}), dropped_bad, "Same"),
        (Source({"core": "U", "old": "O"}), dropped_old, "Same"),
        (Source({"core": "U", "old": "O"}), dropped_bad, "Different"),
        (Source({"core": "U"}), dropped_old, "Different"),
        (Source({"core": "U", "extra": UNKNOWN}), rejected, "Unknown"),
        (Source({"core": "U", "extra": "BAD"}), rejected, "Same"),
        (Source({"core": "V", "extra": UNKNOWN}), rejected, "Different"),
        (Source(dict(rejected.trees), admitted=False), rejected, "Unknown"),
    ))

    narrow = Entry(frozenset(("core",)), {"core": "U"})
    cases.extend((
        (rejected, Entry(scope, dict(rejected.trees)), "Same"),
        (recovered, Entry(scope, dict(recovered.trees)), "Same"),
        (rejected, recovered, "Different"),
        (dropped_bad, dropped_old, "Different"),
        (narrow, rejected, "Unknown"),
        (narrow, recovered, "Unknown"),
        (narrow, Entry(frozenset(("other",)), {"other": "X"}), "Unknown"),
        (narrow, Entry(scope, {"core": "V", "extra": "BAD"}), "Different"),
    ))
    return cases


def check_release_matrix(compare):
    cases = release_cases()
    for index, (left, right, expected) in enumerate(cases, 1):
        actual = compare(left, right)
        assert actual == expected, (index, expected, actual)
        assert compare(right, left) == expected, ("reversed", index)
    return len(cases)


def old_source_union(left, right):
    if isinstance(left, Source) or isinstance(right, Source):
        # A finite subset of the old source's universal knowledge domain.
        keys = {"core", "extra", "z"} | set(left.trees) | set(right.trees)
        keys |= left.scope if isinstance(left, Entry) else right.scope
        return compare_over(left, right, keys)
    return same_release(left, right)


def common_tree_keys_only(left, right):
    return compare_over(left, right, set(left.trees) & set(right.trees))


def expect_model_control_failure(compare):
    try:
        check_release_matrix(compare)
    except AssertionError:
        return
    raise AssertionError("The model control did not break a named case")


def recovered_resolution(recovered, rejections, sources):
    copies = [s for s in sources if same_release(s, recovered) == "Same"]
    if not copies:
        return "RecoveredUnavailable", []
    classifications = []
    for source in copies:
        matches = [same_release(source, r) for r in rejections]
        result = (
            "Rejected" if "Same" in matches
            else "Waiting" if "Unknown" in matches
            else "Usable"
        )
        classifications.append((source, result))
    if all(result == "Rejected" for _, result in classifications):
        return "Advance", []
    usable = [s for s, result in classifications if result == "Usable"]
    if usable:
        return "Constrain", usable
    return "Wait", []


def recovered_probes():
    recovered = Entry(frozenset(("core",)), {"core": "U"})
    rejected = Entry(frozenset(("core", "extra")), {"core": "U", "extra": "BAD"})
    safe = Source({"core": "U"})
    bad = Source({"core": "U", "extra": "BAD"})
    waiting = Source({"core": "U", "extra": UNKNOWN})
    assert same_release(recovered, rejected) == "Unknown"
    assert recovered_resolution(recovered, [rejected], []) == ("RecoveredUnavailable", [])
    assert recovered_resolution(recovered, [rejected], [bad]) == ("Advance", [])
    assert recovered_resolution(recovered, [rejected], [waiting]) == ("Wait", [])
    assert recovered_resolution(recovered, [rejected], [bad, waiting]) == ("Wait", [])
    for copies in ([safe], [safe, bad], [safe, waiting], [safe, bad, waiting]):
        assert recovered_resolution(recovered, [rejected], copies) == ("Constrain", [safe])
    assert recovered_resolution(
        recovered, [rejected], [Source({"core": "V"})]
    ) == ("RecoveredUnavailable", [])
    print("PASS R4D-29: empty, rejected, waiting, usable and mixed-copy branches")
    print("MINOR R4D-32: mixed usable+Unknown follows rollback step 4, not unconditional wait")


def revision_valid(kind, issuer, listed_authorities):
    if kind == "auto":
        return bool(listed_authorities) and all(a == issuer for a in listed_authorities)
    return True  # Other validity, membership and ancestry gates are assumed satisfied.


def policy_conflict(current_authorities):
    return len(set(current_authorities)) > 1


def can_publish_auto(issuer, current_authorities):
    return bool(current_authorities) and all(a == issuer for a in current_authorities)


def policy_probes():
    assert revision_valid("auto", "A", ["A"])
    assert not policy_conflict(["B"])
    assert not can_publish_auto("A", ["B"])
    assert can_publish_auto("B", ["B"])
    for kind in ("push", "choice", "restore"):
        assert revision_valid(kind, "C", ["B"])
        assert not policy_conflict(["B"])
        assert policy_conflict(["B", "D"])
    assert not revision_valid("auto", "A", [])
    assert not revision_valid("auto", "A", ["B"])
    agreeing_heads = [(f"{i:02}", "A") for i in range(9)]
    witness = sorted(agreeing_heads)[:8]
    assert revision_valid("auto", "A", [a for _, a in witness])
    assert not policy_conflict([a for _, a in agreeing_heads])
    assert policy_conflict([a for _, a in agreeing_heads] + ["B"])
    print("PASS R4D-30: historical author, settled reassignment, explicit peer, 9/10 heads")


@dataclass
class HeldSelf:
    retirement: str = "Unresolved"
    pending: list = field(default_factory=list)
    shared_writes: list = field(default_factory=list)
    cancelled: int = 0

    def reserve(self, seq, fixed_bytes):
        assert self.retirement != "Yes"
        self.pending.append((seq, fixed_bytes))

    def hold(self):
        return {"Unresolved": "RetirementUnresolved", "Yes": "Retired", "No": None}[
            self.retirement
        ]

    def cancel(self):
        self.cancelled += len(self.pending)
        self.pending.clear()

    def settle(self, status):
        self.retirement = status
        if status == "No":
            self.shared_writes.extend(self.pending)
            self.pending.clear()
        elif status == "Yes":
            self.cancel()


def held_probes():
    # Opaque synthetic reserved bytes, not a purported valid production record.
    reserved = (72, b'{"synthetic_choice":"keep"}')
    held = HeldSelf()
    held.reserve(*reserved)
    assert held.pending == [reserved] and not held.shared_writes
    assert held.hold() == "RetirementUnresolved"
    # Reconstruct the local queue across a model restart; no provider write.
    local_bytes = json.dumps([
        [seq, payload.hex()] for seq, payload in held.pending
    ])
    restarted = HeldSelf(pending=[
        (seq, bytes.fromhex(payload)) for seq, payload in json.loads(local_bytes)
    ])
    restarted.settle("No")
    assert restarted.shared_writes == [reserved]
    assert not restarted.pending and restarted.hold() is None

    retired = HeldSelf()
    retired.reserve(*reserved)
    retired.settle("Yes")
    assert retired.cancelled == 1 and not retired.shared_writes
    assert not retired.pending and retired.hold() == "Retired"

    cancelled = HeldSelf()
    cancelled.reserve(*reserved)
    cancelled.cancel()
    cancelled.settle("No")
    assert cancelled.cancelled == 1 and not cancelled.shared_writes
    print("PASS R4D-31: pending/restart/Void/Yes/explicit-cancel model transitions")


if __name__ == "__main__":
    count = check_release_matrix(same_release)
    print(f"PASS R4D-28: {count} release cases, both argument orders")
    expect_model_control_failure(old_source_union)
    expect_model_control_failure(common_tree_keys_only)
    print("PASS model controls: old source-union and common-tree-only rules detected")
    recovered_probes()
    policy_probes()
    held_probes()
    print("Design models complete; no new product behavior or A-38 execution claimed.")
