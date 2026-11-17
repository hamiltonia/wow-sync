# UI open questions

Numbering continues the project list in [../questions.md](../questions.md) (last used: Q-53),
so UI questions are prefixed `UQ-`.

| ID | Question | Current behaviour | Decider |
|---|---|---|---|
| UQ-1 | Should the dashboard headline be promoted when a guided reconfirmation is pending (for example "Review needed" instead of core's headline)? (R3 review RV-5) | None. The app shows core's `Headline` as is; the reconfirmation appears as its own banner and in the Computers page. Pause is the only app-level override, and it never hides a core `NeedsHelp` or `Error` headline. | owner |
| UQ-2 | Should core expose a typed scope for a pending decision ("only this add-on is affected" versus "the whole group is held")? (UG-25) | None. The app says nothing about scope and shows no "Only X is affected" sentence until core states it. | design, then owner |
