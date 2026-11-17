# Evidence S-forever: identity of the `_forever_` installation

Worker S-forever, run-001 Phase 2. Environment: Windows 11 x64 dev PC (Insider 26300,
not official support evidence). Read-only; nothing written under the WoW install.
Packet: [S-forever.md](../packets/S-forever.md).

## SPEC §7 question and answer

Question: authoritative variant/install identity, including verification of "Forever".

**Answer: FEASIBLE, confidence high (~90%).** "Forever" is a distinct Blizzard product,
product code `wowf`, not Classic Era (`wow_classic_era`), not Classic (`wow_classic`),
not Retail (`wow`). It installs into a `_forever_` folder. Identity derives from
installation metadata alone. What would change it: a Blizzard product-code rename, or
a machine with a Forever install lacking `.product.db`/`.flavor.info`.

## Raw values observed (PASS, dev PC, read-only)

Root `D:\Games\World of Warcraft`:

| Source | Value |
| --- | --- |
| `.build.info` Product | `wowf` |
| `.build.info` Branch / Active | `us` / `1` |
| `.build.info` Version | `1.60.1.70938` |
| `.build.info` Build Key / CDN Key | `3627b099867bfde3e37a17cf44f69cdc` / `3363ac656da5f261cdc675ab28b2744c` |
| `.build.info` CDN Path | `tpr/wow` |
| `.build.info` Tags | `Windows x86_64 US? acct-USA? geoip-US? enUS speech?` and `... enUS text?` |
| `_forever_\.flavor.info` | `Product Flavor!STRING:0` / `wowf` |
| `.product.db` (strings only) | `wowf`, `wow`, `enUS`, `USA`/`US`, subfolder `_forever_`, version `1.60.1.70938`, build key as above, `user_install_package_settings` empty |
| `.patch.result` | `0` |
| `Launcher.db` | `enUS` |
| `_forever_\Wow.exe` | ProductName `World of Warcraft`, version `1.60.1.70938`, OriginalFilename `WoW.exe`, size 81760464 |
| `_forever_\wow_loader.dll` | no version resource |
| `_forever_\BlizzardError.exe` | `2.4.2.0` |
| `World of Warcraft Launcher.exe` | `1.18.10.3140` |
| `Data\` subfolders | `config`, `data`, `ecache`, `indices`, `wowf` |
| `_forever_` subfolders | `Cache`, `D3D12`, `Fonts`, `Interface`, `Logs`, `Speech`, `UTILS`, `WTF` |
| AddOns folders | 102 |

Note the exe is `Wow.exe` (no `Classic`/`Era` suffix, no `_classic_` style folder);
the variant is told apart only by the folder `_forever_`, `.flavor.info` and
`.build.info`. `.build.info` Version and `Wow.exe` version agree (PASS).

## Derivation of a stable variant identity

Recommended key, in order: (1) `_<name>_\.flavor.info` second line (`wowf`); cross-check
(2) `.build.info` `Product` for the matching row and (3) `.product.db` product
code plus the `_forever_` subfolder. Treat build number (`70938`) and version as
mutable, not identity. Require all sources agree; on disagreement or unknown
product code, refuse automatic enrollment (guided identity per AGENTS.md). Do not
key on folder name, exe name or `Interface` number alone.

## Public corroboration (WEB, not retested)

- BlizzTrack lists product `wowf` as "World of Warcraft WoW: Forever" versions
  (US/EU `1.60.1.70707`, indexed 2026-10-30): <https://blizztrack.com/view/wowf?type=versions>
- Hyjal setup guide: "This installs WoW Forever (not Classic Era or Retail)", select the
  `_forever_` folder, requires 1.60.1.70862: <https://hyjal.gg/setup>

Build numbers differ (70707, 70862, 70938) with time, confirming build is not identity.

## Interface numbers in installed `.toc` files (corroboration)

Counts of `## Interface` values across toc files (depth ≤ 2, comma lists split; one
count per value occurrence): 16001: 112; 120105: 37; 11509: 33; 20506: 32; 120107: 25;
50504: 23; 38002: 13; 40402: 11; 30405: 9; 120100: 4; 11508: 2; 30403: 2; 00000: 1;
120007: 1; 38000: 1. `16001` matches the client version 1.60.1 (1.60.01), consistent
with [S-addon.md](S-addon.md) (16001 in `_Forever`/`_Camelot` toc suffixes). The
Interface scheme is Major.Minor.Patch → 1.60.1; this is the Forever target number.
CLAIM: addons with 16001 target Forever (PASS as correlation; not proof).

## Method limits and risks

- One installation, one region (`us`), one locale; multi-region or beta/PTR rows in
  `.build.info` not seen. `Data\` archives were not read; only directory names.
- `.product.db` is protobuf-like binary; only ASCII strings were inspected, not parsed.
- `.build.info` may hold several rows per machine (other products); the parser must
  select the row for the install directory.
- Product code `wowf` is not in older public Blizzard lists; it is new, so future
  renames are a risk. macOS runtime: BLOCKED (no Mac); layout there is unverified.
- Public sources are third-party; the packet's own local files are primary evidence.

## Rerun

No spike code. Commands: read-only `Get-Content` of `.build.info`, `_forever_\.flavor.info`,
`Launcher.db`, `.patch.result`; `(Get-Item _forever_\Wow.exe).VersionInfo`; ASCII-string
dump of `.product.db`; `Select-String '^##\s*Interface'` over `*.toc`.
Sanitization: no account, battle.net id or user path beyond install root recorded.
