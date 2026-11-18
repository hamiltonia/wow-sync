# UI open questions

Numbering continues the project list in [../questions.md](../questions.md) (last used: Q-53),
so UI questions are prefixed `UQ-`.

| ID | Question | Current behaviour | Decider |
|---|---|---|---|
| UQ-1 | Should the dashboard headline be promoted when a guided reconfirmation is pending (for example "Review needed" instead of core's headline)? (R3 review RV-5) | **Decided yes by D-23.** A pending guided reconfirmation promotes the headline to "Review needed" (core `Headline::ReviewNeeded`); pause still never hides `NeedsHelp` or `Error`. Design: [../ops.md](../ops.md) §1.7. (Before D-23: the app showed core's `Headline` as is, with the reconfirmation as its own banner.) | owner, D-23 |
| UQ-2 | Should core expose a typed scope for a pending decision ("only this add-on is affected" versus "the whole group is held")? (UG-25) | **Decided yes by D-23.** Core exposes `StatusView.hold: HoldScope` (`Addons` versus `AllApplies`; named `Packages` before run-004) and `DecisionReach` (`Group` versus `ThisComputer`). Design: [../ops.md](../ops.md) §1.6. (Before D-23: the app showed no scope sentence.) | design, then owner, D-23 |
