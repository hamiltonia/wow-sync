# Decisions made while the owner was away (for review)

Owner, 2026-11-24: "after you complete, I want you to get started on the
WTF sync, I will be away, I want you to continue as long as you can, you
will have to make decisions, collect them all for review when I'm back."
(D-33.)

Each row is a judgment call made without the owner. "Reversible" says
whether changing it later is cheap. Worker decisions are copied in from
their evidence files.

| # | When | Decision | Why | Reversible? |
|---|---|---|---|---|
| OD-01 | 2026-11-24 | "WTF sync" means making add-on saved settings (SavedVariables under `WTF`) sync for real between computers. It does not mean copying the whole WTF folder or WoW's own files (Config.wtf, bindings, macros). | SPEC R-07 forbids whole-WTF or WoW-own-file sync; settings code already exists from run-004 but is unused on the live group. | Yes, a SPEC change if you want more. |
| OD-02 | 2026-11-24 | Run-010 merged with the independent speed proof on a copy of your real group (3.25×), not on your live GUI. Your live app's 2–5 s no-change passes were not reproduced headless; R10-check measures them with the GUI artifact. | The gate (≥3×) is met on representative data; the GUI-only overhead is a separate question. | Yes |
| OD-03 | 2026-11-24 | WTF sync (run-011) keeps the approved design: accounts/characters match by exact folder name (Q-69), capture waits for two stable scans after WoW exits (Q-71), app never creates account/realm/character folders, so a character's settings wait until that character has logged in on the other PC. | Already owner-approved; changing it would be a product change. | Yes, new decision |
| OD-04 | 2026-11-24 | Settings sharing is offered, never switched on automatically: once add-ons are up to date, the app prompts "Share add-on settings?" with account consent and an explicit master choice (this computer is pre-selected, but you must click). The other computer gets its own prompt to include its accounts. | SPEC R-07 requires consent and an explicit master. | Yes |
| OD-05 | 2026-11-24 | Run-011 testing uses lab copies of the dev PC's WTF and add-ons (D-14 test data) with two synthetic computers. Nothing turns settings on in your real group; you do that yourself after release. | Workers may not write to the real group or your instances. | — |
| OD-06 | 2026-11-24 | Run-011 releases as `v0.4.0` (minor bump), because settings sharing becomes a guided feature. | Signals a larger behaviour change than 0.3.x fixes. | Yes |
| OD-07 | 2026-11-24 | v0.3.3 (RV10-14): while any add-on installs or is removed, the progress headline temporarily replaces 'Waiting for your decision'; the decision returns afterwards. PLAN said only satisfied decisions are replaced; accepted as-is and documented in the release notes. | Live install progress is more useful while it runs; the decision is not lost. | Yes |
| OD-08 | 2026-11-24 | Fix minors RV10-12 (byte count resets per folder) and RV10-13 (crash-recovered install labelled as published from this computer) before tagging v0.3.3; defer RV10-15 (some old-style log lines remain). | Both undermine the change visibility you asked for; RV10-15 is cosmetic. | Yes |
| OD-09 | 2026-11-24 | Run-011 workers may copy the dev PC's real add-ons and `WTF` SavedVariables (read-only) into `D:\WoWSyncLab\` for realistic testing, but nothing with real account, realm or character names, or file contents, is committed, logged in evidence or screenshotted. Committed tests use synthetic data. | Realistic shape (80 files, account-wide and per-character) finds real bugs; D-14 allows dev data for testing; privacy rules stay. | Yes |
| OD-10 | 2026-11-24 | The guided "Share add-on settings?" offer appears in the headline area once add-ons are up to date and nothing is waiting, never as a modal. "Not now" is remembered; the offer then lives only on the Settings sync page. | Least intrusive way to make an existing feature discoverable; the owner already dislikes stale headlines. | Yes |
| OD-11 | 2026-11-24 | In the consent step, all accounts found on the computer making the choice are ticked by default; on other computers the confirmation shows their own account list, also ticked by default. | design/settings.md §5.4 says "included by default only on the computer making the choice"; each other computer shows its own list at its own confirmation. | Yes |
