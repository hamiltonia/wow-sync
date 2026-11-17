# R3-compat (Implementation, sonnet-5.5)

Read [COMMON.md](COMMON.md) first.

**Owned paths:**
- `crates/wow-sync-core/**`;
- `crates/wow-sync-runtime/**`, for wiring and e2e only;
- `docs/design/domain.md` §6 and the Q-21 row in `docs/design/questions.md`;
- `docs/design/tests.md`;
- `tests/required-cases.toml`, adding entries only.

## Rule (owner decision D-18, Q-21)

A candidate addon version from a computer with a different client
`build_version` installs automatically only when **every** selected `.toc`
`## Interface:` number in the candidate package is supported by the
receiving client. Otherwise it is
`NeedsDecision(DifferentClientBuild)`, as today.

## Tasks

1. **Define "supported by the receiving client" from local evidence only.**
   Suggested definition: the interface number is equal to an interface number
   seen in that client's own installed `.toc` files for the same variant, or
   to a profile-declared interface list. Choose the most conservative rule
   that keeps ordinary patch-day builds automatic.
   - Missing or unparsable interface numbers mean a decision is needed.
   - If the receiving client exposes no interface evidence, a decision is
     needed.
   - Document the rule in `domain.md` §6 and update Q-21.
2. **Implement it in core** as a pure function. Plan and executor use it.
   - A different `DifferentTocMapping` still needs a decision.
   - The same build behaves as before.
3. **Tests:**
   - core unit tests, covering at least:
     - interface number matches;
     - interface number differs;
     - number missing;
     - multi-folder package with one folder mismatching;
     - same build unchanged;
   - one runtime e2e test with two synthetic installs on different builds but
     the same interface number, where the apply happens;
   - one e2e test where the interface numbers differ, which produces a
     decision.
   - All tests are listed in required-cases (offline tier, must run on all
     OSes).
4. Record A-38 negative controls for each new test.

## Evidence

`docs/runs/run-003/evidence/R3-compat.md`.
