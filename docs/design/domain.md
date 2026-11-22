# Domain rules

Part of the [slice design](README.md). All rules here are implemented once, in
`wow-sync-core` ([crates.md](crates.md)). Inputs are bytes and observations
supplied by adapters; nothing is executed (R-11).

## 1. Computer, group and enrollment identity (R-03)

| Identity | Form | Created by | Notes |
|---|---|---|---|
| `ComputerId` | 32 lowercase hex chars (128 random bits) | shell, once per local app-state directory, via `adapters::sysinfo::random_id_hex` | Stored in local state `computer.json`. Not derived from host name, user name, SID or hardware (privacy, R-15) |
| display name | free text ≤ 64 chars | user | Label only; never identity or authorization |
| `GroupId` | 32 hex | shell, when the user creates a group | One group = one variant product code (§2). A shared location may hold several groups |
| `EnrollmentId` | 32 hex | shell, each time this computer joins a group | Member records and state files carry it. Retirement/re-enrollment (later run) keys on `(ComputerId, EnrollmentId)` |
| `OpId` | 32 hex | shell, per apply operation | Names staging, journal and recovery directories |

Enrollment steps (GUI in R2-slice; rules in core):

1. Shared location chosen by the user; must be an existing directory. The app
   uses `<shared>\WoWSync\` beneath it ([publication.md](publication.md) §1).
   For tests it is a temp dir; for provider tests exactly
   `D:\Dropbox\Gaming Sync\WoW Sync Test\<run-subdir>` (D-08).
2. Install root chosen by the user; candidates = immediate subdirectories whose
   names match `^_[a-z0-9_]+_$` (case-insensitive) and contain `.flavor.info`.
3. Variant validated (§2) and its layout assessed (§2.1). `Refused` or
   `Unsupported` layout → no enrollment, shown with the reason.
   `Unfamiliar` → guided validation (§2.1): the GUI shows the evidence and
   requires two separate confirmations bound to that evidence (A-46). Consent
   alone never validates. The display name cannot change any of these values.
4. Create a new group (records the variant product code) or join an existing
   group in the shared location. Join is refused unless the group's product
   code equals the validated local product code byte for byte.
5. Consent text shown before writing anything (R-03), minimum content:
   "This computer will share its installed addons for <label> (<product>) with
   other computers in this group, and will install newer versions found on
   them. Folders it replaces are kept on this computer for recovery under
   `<variant>\.wowsync\recovery`. Addons missing here are installed, never
   treated as removed. Settings (WTF) are not synchronized in this version."
6. Member record written, then the first state is published (§ apply.md).

## 2. Variant identity and validation (R-02)

Observations (adapters read bytes; core parses):

| Input | Path | Parsed as |
|---|---|---|
| folder name | `<root>\<folder>` | string, e.g. `_forever_` |
| `.flavor.info` | `<root>\<folder>\.flavor.info` | BPSV (below); exactly one column, first data row = product code |
| `.build.info` | `<root>\.build.info` | BPSV; columns `Product` and `Active` required, `Version` optional |
| AddOns directory | `<root>\<folder>\Interface\AddOns` | must exist as a plain directory (no reparse point) |

`.product.db` (binary) is **not parsed** in the slice; the S-forever finding
(REPORT §3: `wowf` consistent across `.build.info`, `.product.db`,
`.flavor.info`) is matched by the first two (Q-11).

**BPSV format** (`core::bpsv`): UTF-8 (BOM allowed), lines split on `\n`
(strip trailing `\r`); lines starting `##` are comments; empty lines ignored.
First non-comment line is the header: fields separated by `|`, each
`Name!TYPE:size`; the name is the part before `!`. Every following line is a
row with exactly as many `|`-separated fields as the header, else
`Unparseable`. Max 1 MiB input. The real header names of `.flavor.info` are
not assumed: only "exactly one column" is required.

**Product code** syntax: `^[a-z0-9_]{1,32}$` after trimming; otherwise
`InvalidProductCode`.

**Validation** `validate_variant(obs) -> VariantVerdict`, first match wins:

| # | Condition | Verdict |
|---|---|---|
| 1 | `.flavor.info` missing / unreadable | `Refused(FlavorMissing / FlavorUnreadable)` |
| 2 | not one column or no data row | `Refused(FlavorUnparseable)` |
| 3 | product code syntax invalid | `Refused(InvalidProductCode)` |
| 4 | `.build.info` missing / unreadable / unparseable / lacks `Product` or `Active` | `Refused(BuildInfo…)` |
| 5 | no row with `Product == code` | `Refused(ProductNotInBuildInfo)` (disagreement) |
| 6 | more than one such row | `Refused(ProductAmbiguousInBuildInfo)` |
| 7 | that row's `Active` is not `1` | `Refused(ProductInactive)` |
| 8 | code is a known profile and folder name (case-insensitive) differs from the profile's folder | `Refused(FolderMismatch)` (disagreement) |
| 9 | AddOns directory absent or not a plain directory | `Refused(AddOnsMissing)` |
| 10 | code is a known profile | `Recognized(identity)` |
| 11 | otherwise | `Unfamiliar(identity)`: blocked until guided validation (§2.1) |

The verdict is re-derived every cycle; if it changes from the enrolled
identity (different product, refused), the cycle stops before any write and
status is `NeedsHelp(VariantChanged)`.

### 2.1 Layout assessment and guided validation (M-02)

`assess_layout(identity, addons: &[FolderScan], others: &OtherVariants) ->
LayoutAssessment` (core, pure). Checks, all required:

| Check | Fails as |
|---|---|
| AddOns is a plain directory at exactly `<variant>\Interface\AddOns` | `Unsupported(AddOnsLayout)` |
| no other folder under the same install root reports the same product code | `Unsupported(DuplicateProductFolder)` |
| this variant folder is not enrolled in another group on this computer | `Unsupported(AlreadyEnrolled)` |
| the install root carries the synthetic marker `.wowsync-synthetic` (first line `wowsync-synthetic/1`) **only when test mode is on** (D-28, `MarkerPolicy::Required`); by default (`NotRequired`) the marker is neither required nor consulted, and a real install has none | test mode only: `Unsupported(NotSyntheticInstall)` |
| `.toc` mapping: `profile:wowf/2` for the recognized `wowf` client (the Forever rule, §3); `validated-plain/1` for every unfamiliar client (below) | — |

For `validated-plain/1` (unfamiliar clients) a folder is **eligible** for transfer only if it
has a plain `F.toc` and **no** suffixed candidate (`F_<S>.toc` /
`F-<S>.toc`) at all. No such client's suffix order is known, so every folder with suffixed tocs is `Blocked
(UnvalidatedTocMapping)` — an addon-scoped decision, never installed or
replaced automatically (Q-20). With a single plain toc there is exactly one
file list the client can load, so equal summary fields cannot hide
different loaded content (R-M03). The result is
`Supported { mapping, eligible: Vec<FolderKey>, blocked: Vec<(FolderKey, reason)>, digest: Sha256 }`
where `digest` hashes the identity, mapping and both lists.

Guided validation (GUI, R2-slice) shows: product code, folder path, client
build version, AddOns path, the mapping, eligible/blocked folder counts with
names, and the disclosure "Settings (WTF) are not synchronized; their layout
for this client has not been validated." The user gives two separate
confirmations: **identity** ("this is the client I mean") and **scope**
("WoW Sync may change only the listed AddOns folders of this client").
`confirm_variant(verdict, assessment, GuidedConfirmation { identity, scope,
digest })` returns `ConfirmedVariant` only if the verdict is `Recognized`
(with a `Supported` assessment), or `Unfamiliar` with a `Supported`
assessment, both confirmations `true` and `digest` equal to the current
assessment's digest (stale consent is rejected). The stored
`validation` record keeps the evidence that was shown. Blocked folders
remain blocked after confirmation. The scope confirmation covers the
mapping rule, not a frozen folder list: folders that appear later are
evaluated by the same rule each scan; the digest only rejects consent given
for a different assessment at confirmation time.

Known profiles (the only one; everything else is unfamiliar, Q-12):

| product | folder | label | `.toc` suffixes, in order | family suffixes | evidence |
|---|---|---|---|---|---|
| `wowf` | `_forever_` | Forever | `Forever`, `Camelot` | `Mainline` | S-forever (dev PC); TOC-format documentation (warcraft.wiki.gg "TOC format", read 2026-11-22); an owner client check is optional (Q-3) |

Never label `wowf` as Classic Era (AGENTS.md). Interface numbers (e.g. 16001)
are **not** used for identity (S-addon: plain tocs carry unions).

The group record stores the product code; every record read from the provider
folder whose variant product differs from the group's is rejected as invalid
(never merged). The variant key for all data is `(GroupId, product)`.

## 3. `.toc` parsing and the single selection rule

One parser and one selection function for the product (P3-review M4, L5).

**Candidates** for folder `F`: files directly in `F` whose name, compared
ASCII-case-insensitively, is `F.toc`, `F_<S>.toc` or `F-<S>.toc` where `<S>`
is `[A-Za-z0-9]+`. Other `.toc` names are ignored for selection and listed in
diagnostics.

**Parse** (`parse_toc`): ≤ 256 KiB else `TooLarge`; strip UTF-8 BOM; decode
UTF-8, invalid bytes → `NotUtf8` error (fail closed, never "empty"). Only lines
starting `##` are read: `## Key: Value` (key and value trimmed; key compared
case-insensitively). Recorded keys:

| Key | Field |
|---|---|
| `Interface` | `interface: Vec<u32>` (valid numeric tokens; display only, never used to gate sync: Q-21, D-21) |
| `Version` | `version: Option<String>` |
| `Title` | `title: Option<String>` (display only; color codes kept as text) |
| `Dependencies`, `RequiredDeps`, any key starting `Dep` | `hard_deps: Vec<String>` (comma-separated, trimmed, non-empty) |
| `X-Curse-Project-ID`, `X-Wago-ID`, `X-WoWI-ID` | `ids` |

Everything else (including `SavedVariables`, file lists, Lua/XML) is ignored
in the slice. A candidate that cannot be read or parsed makes the selection an
`Error` (not empty).

**Select** `select_toc(folder, candidates, mapping) -> TocSelection`.
`validated-plain/1` (unfamiliar clients): if any suffixed
candidate exists → `Unvalidated { files }`; else the plain `F.toc` →
`Selected`; else `None`. `profile:wowf/2` (the Forever rule, D-28) is as built:

1. Tiers, in order: the profile's game-type suffixes (`Forever`, then
   `Camelot`), then its family suffixes (`Mainline`), then the plain `F.toc`.
   Candidates match either separator, case-insensitively. Other suffixes
   (`Vanilla`, `Classic`, `Wrath`, …) are ignored.
2. The first tier that has candidates decides. Its candidates are all parsed
   (any parse or read error → `Error`). If every one has the same `version`,
   `hard_deps` (as sets) and `ids`, the first listed suffix is selected;
   otherwise → `Ambiguous { files }`.
3. No candidate in any tier (only other suffixes, e.g. only `_Vanilla`) →
   `None { reason: "no toc for this variant" }`.

`mapping_id` is `profile:wowf/2`. An already enrolled `wowf` whose stored
validation says `validated-plain/1` gets `StaleConfirmation`: syncing stops
until the guided reconfirmation (Reconfirm, `NeedsReconfirmation`); nothing is
deleted. A peer that still publishes `validated-plain/1` is a
`DifferentTocMapping` holder until it upgrades and republishes.

The `wowf` order comes from the TOC-format documentation (warcraft.wiki.gg
"TOC format", read 2026-11-22) and the dev-PC folder survey; an owner client
check stays optional (Q-3).

**Compatibility gate (M-03).** Only `Selected` is evidence that the client
would load the folder. A folder whose selection is `None`, `Ambiguous`,
`Unvalidated` or `Error` is **never installed or replaced automatically**:
its package becomes `NeedsDecision(NoCompatibleToc)` ([§6](#6-group-target-r-05-r-06)).

## 4. Version ordering (R-05)

`version::classify(raw: Option<&str>) -> Version` and
`version::compare(a, b) -> VersionOrder` with `VersionOrder =
Less | Equal | Greater | Ambiguous(AmbiguityReason)`. Ported from
`spikes/addon/src/version.rs` (behavior, not code).

| Scheme | Recognized as | Orders against |
|---|---|---|
| `DottedNumeric` | optional `v`/`V`, ≥ 2 numeric parts separated by `.`, optional pre-release `-`/`.`/none + `dev`/`alpha`/`a`/`beta`/`b`/`rc` + optional number; or git-describe `<dotted>-<n>-g<hex>` | same scheme **and same channel** only; numeric parts zero-padded (`1.2` = `1.2.0`). Channel = `release`, or the pre-release word (`dev`, `alpha`, `beta`, `rc`); different channels → `Ambiguous("different channels")` (A-08 beta/stable), e.g. `1.3.0-beta1` vs `1.2.9`. Same pre-release channel and same base → ordered by its number. Git-describe: only `<tag>` < `<tag>-<n>-g<hex>` with the **identical** base tag is ordered; two describes (even with the same tag: different branches, M-03) or describe vs any other label → `Ambiguous`. Covers the spike's semver-like, numeric-dotted and git-describe as **one** scheme (Q-10) |
| `Integer` | all digits, ≤ 18 digits | `Integer` only |
| `Revision` | `r`/`R` + digits | `Revision` only |
| `Date` | `YYYY-MM-DD`, `YYYY.MM.DD`, `YYYYMMDD` with a valid date | `Date` only (checked before `Integer`) |
| `Tagged` | `<letters> <DottedNumeric>` e.g. `Forever 1.60.1` | `Tagged` with the same tag (case-insensitive); pre-release on either side → ambiguous |
| `Missing` | absent or blank | nothing |
| `Token` | contains `@` or `<…>` (unsubstituted template) | nothing |
| `GitHash` | 7–40 hex chars only | nothing (equal strings are still not proof of equal code) |
| `Other` | anything else, incl. unknown pre-release words | nothing |

Rules: any `Missing`/`Token`/`GitHash`/`Other` → `Ambiguous`; different
schemes → `Ambiguous("different version schemes")`; same scheme compares
strictly within the rules above. `Equal` means equal **labels**, never equal
content. There is no input for file times, clocks or computer preference
(R-05).

What a `Greater` result is evidence of (M-03): the author's label is larger
within one recognized scheme and channel. It is **not** proof that the
release is unmodified, from the same distribution, or compatible; those
come from other predicates in §6 (same variant group, same client build,
`Selected` toc under a known mapping, complete package). Manual edits that
keep the label but change content show up as equal labels with different
content → `Ambiguous`; a manual edit that also changes the label to a larger
one is indistinguishable from a release. The slice therefore does **not**
claim release provenance: same label with different content is `Ambiguous`
and never replaced (Q-22). The synthetic-marker gate is test mode only
(§2.1, D-28). Real
release-pair ordering has not been measured (S-addon: recognition only);
tests use real labels for recognition and synthetic pairs for ordering,
and the pairs are labeled as such.

Tests use real labels from run-001 S-addon (`12.1.23`, `v1.3.9`, `3.220`,
`r272`, `31217`, `Forever 1.60.1`, `BCC 2.5.4`, `a8e9f0e`,
`<project-version-goes-here>`), committed as D-14 derived metadata
([tests.md](tests.md)).

## 5. Addon identity and packages (R-04, R-05, A-09)

**Folder key**: `FolderKey` = folder name lowercased (`str::to_lowercase`).
Folder names and file paths must satisfy `paths` rules: ASCII printable only
(0x20–0x7E) minus `<>:"/\|?*`, no leading/trailing space, no trailing dot, not
`.`/`..`, not a Windows reserved device name (`CON`, `PRN`, `AUX`, `NUL`,
`COM1`–`COM9`, `LPT1`–`LPT9`, with or without extension), component ≤ 255
bytes, relative path ≤ 400 bytes, no tab/newline. Non-conforming folders are
`Unsupported` and never transferred (limitation, Q-14). Two names in one
inventory with the same key → both `Unsupported` (case collision, A-23).

**Grouping** `group_packages(metas) -> Vec<Package>`: union-find over folder
keys using every observation available (local inventory plus all valid
member states), so all computers derive the same packages from the same
inputs. Edges (each recorded with its kind for explanations):

| Edge | Condition | Example (S-addon data) |
|---|---|---|
| `SharedId` | two folders' selected tocs have the same value for the same id kind | `DBM-Core`/`DBM-Raids-Midnight` (curse 3358); `VuhDo`/`VuhDoOptions` |
| `Dependency` | `F` hard-depends on installed `G` **and** `stem(G)` is a prefix of `key(F)`, where `stem(G)` = `key(G)` up to its first `-` or `_` | `DBM-Party-BC → DBM-Core` (stem `dbm`); `AtlasLootClassic_Data → AtlasLootClassic`; **not** `AtlasBIStooltips → AtlasLootClassic` |
| `NamePrefix` | `key(F)` = `key(G)` + (`-` or `_`) + more | `GearQuestForever_Languages → GearQuestForever` |
| `Family` | `key(G)` has ≥ 5 characters and `key(F)` = `key(G)` + more (no separator) **and** neither folder has an external id that differs from the other's | `QuestieDB → Questie` (M-04) |

Dependencies on folders not present in any observation (e.g. `Blizzard_*`)
create no edge. Over-grouping only enlarges the unit that is replaced together
and can only add ambiguity; under-grouping can mix package parts, so the rule
errs toward grouping (Q-1). Every package with a `Family` or `NamePrefix`
edge shows the relationship in its explanation, so a wrong grouping is
visible, not silent.

Grouping is computed from the union of all observations and can change when
a new folder appears anywhere. The package hash is always computed over the
**current** grouping, so a membership change produces new hashes; any
earlier confirmation for the old grouping no longer matches (it is shown as
unconfirmed rather than misattributed).

**Package key** = smallest member `FolderKey` (byte order). **Display folder**
= member with an external id and fewest in-package dependencies, ties by key.

**Package version** (one per member computer) = that computer's folders of
the package, each with its selected toc version and folder tree hash, plus a
package hash over the folder hashes ([publication.md](publication.md) §3).

**Comparison** `compare_package(a, b) -> PackageOrder`:

1. Equal package hashes → `Same` (identical content converges without
   replacement, R-05).
2. Any member folder of either side with status other than `Ok`
   → `Ambiguous(FolderNotUsable)`.
3. Record the member-folder **set relation** (`Equal`, `AHasMore`,
   `BHasMore`, `Different`) in the result; it is always part of the result
   and the evidence. Ordering below uses common folders; §6 decides whether
   a candidate with a different set may be applied (M-04).
4. For each folder key present on **both** sides whose tocs are `Selected` on
   both and whose tree hashes differ: `version::compare`.
   Folders with identical trees are skipped. Folders whose toc is `Selected`
   with a `Missing` version label on both sides and whose trees differ
   contribute `Unversioned`.
5. Any `Ambiguous` result → `Ambiguous(reason, folder)`.
6. Both `Less` and `Greater` present → `Ambiguous(MembersDisagree)`.
7. Some `Greater` (and no `Less`) → `Greater`; some `Less` → `Less`.
   `Unversioned` members follow this result (they ship with their package,
   e.g. version-less `DBM-Brawlers`).
8. No ordered results (all `Equal` labels, only `Unversioned`, or no common
   folder) → `Ambiguous(SameLabelsDifferentContent | NoComparableVersion)`
   (A-08 duplicate label, different content).

Evidence for the explanation lists every compared folder with both labels.
This compares **all** versioned members rather than only a primary folder
(S-addon suggested primary-only), because primary-only would silently
downgrade a sub-folder updated on another computer.

## 6. Group target (R-05, R-06)

For each package key, candidates = distinct package hashes among: the local
inventory and the **admitted** state of every member
([publication.md](publication.md) §6). Members whose newest state is
unusable/regressed contribute nothing and are shown; they are not absence
(R-04), and their older states are not used.

**Eligibility (compatibility and completeness predicates, M-03/M-04).** A
candidate `c` can be chosen automatically only if all hold:

| Predicate | Fails as `NeedsDecision(...)` |
|---|---|
| every member folder of `c` has status `Ok` and a `Selected` toc under the source member's mapping | `NoCompatibleToc` |
| the source member's `client.toc_mapping` equals the local mapping | `DifferentTocMapping` |
| no member folder is `Blocked(UnvalidatedTocMapping)` locally (§2.1) | `UnvalidatedTocMapping` |
| `c` is **complete**: its folder set equals the package's full membership `Package.members` as established by grouping over **all** admitted observations and the local inventory (§5) — so neither leftover old parts here nor a missing part on an empty peer is possible (retry 2, R-M04) | `PartialCandidate` |

**No game-build or Interface gating (Q-21, owner decision D-21; replaces the
D-18 rule and interim rule A).** A game update, a different or unknown client
`build_version`, and the `## Interface:` declaration (missing, malformed or
different) never withhold a candidate. Only the variant and the matching
account/character boundaries (§2, R-05) and the other predicates above limit
sync; WoW Sync is a sync tool, not addon police. `DifferentTocMapping` is
unaffected. `DecisionReason::DifferentClientBuild` is no longer produced by
core.
A local candidate is always "eligible" for the purpose of staying as it is
(nothing is written). Completeness is checked against the group-wide
membership, not the local folder set; the mapping predicate is
per local computer, so two computers may reach different results and each
explains its own. Slice limitation (Q-2): if an author legitimately drops a
folder from a suite, the new version stays a `PartialCandidate` everywhere
until membership evidence changes (e.g. the old folder is removed by the
user on every computer); the slice has no removal or membership-retirement
rule.

**Choice:**

- Exactly one candidate and it is the local one → `InSync`.
- A candidate `c` with `compare_package(c, x)` ∈ {`Same`, `Greater`} for
  every other candidate `x` → it **dominates**. If it is eligible →
  `Chosen(c)`; if not → `NeedsDecision(reason)` (never fall back to a lesser
  candidate).
- A single non-local candidate (local has none of the package's folders) is
  `Chosen` only if eligible (an addition, R-04).
- Otherwise → `NeedsDecision { candidates, evidence }` for that package only;
  other packages proceed (R-06).

Example (review M-04, recheck R-M04): A has `Suite-Core` v1 + `Suite_Data`
v1, B has only `Suite-Core` v2, C has neither. Membership is
`{suite-core, suite_data}`. B's candidate dominates by Core but lacks
`Suite_Data` → `NeedsDecision(PartialCandidate)` on A **and** on C (the empty
peer does not receive an incomplete suite). On B the dominating candidate is
its own, so nothing is written (`InSync`, with a note that the package is
incomplete relative to the group membership). A's complete v1 does not
dominate (B is `Greater`), so it is not installed on C either — no
fallback. After a successful apply, the
local effective package hash must equal the chosen hash exactly
([apply.md](apply.md) §2.2 step 5 verifies every folder; the package hash
follows).

`sources` = all members holding the chosen hash, sorted by `ComputerId`.
Input order and duplicates never change the result: inputs are sorted by
`(ComputerId, seq)` and deduplicated by package hash before evaluation.
A package that a member does not have is simply not a candidate from that
member; absence never removes (R-04). There is no removal record in the slice.
