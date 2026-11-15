# Addon coverage report

Folders scanned: 102

## Version schemes

| scheme | folders | orderable vs same scheme |
|---|---|---|
| git-hash | 1 | no |
| integer-build | 5 | yes |
| missing | 5 | no |
| numeric-dotted | 3 | yes |
| revision-rN | 14 | yes |
| semver-like | 66 | yes |
| tagged-numeric | 7 | yes |
| unsubstituted-token | 1 | no |

Folders with an orderable version string: 95 (93.1%)
Folders with at least one external package id (X-Curse/WoWI/Wago/GitHub): 29 (28.4%)
Folders with no external id (folder name is the only key): 73 (71.6%)
Folders without any .toc: 0
Folders with a .toc name not matching the folder: 0 []
Folders with only flavor-suffixed tocs (no plain toc): 2 ["QuestieDB", "QuestieTrace"]
Folders whose tocs disagree on Version: 1 ["AtlasLootClassic_DungeonsAndRaids"]

## Version strings per scheme (derived metadata)

### git-hash

DBM-VPVEM=a8e9f0e

### integer-build

ArkInventory=31217; ArkInventoryConfig=31217; ArkInventoryRules=31217; ArkInventorySearch=31217; Auctionator=346

### missing

DBM-Brawlers=; DBM-Party-BfA=; DBM-Party-Shadowlands=; DBM-WorldEvents=; Questie=

### numeric-dotted

SmartBuff=1.121126; VuhDo=3.220; VuhDoOptions=3.220

### revision-rN

DBM-Challenges=r272; DBM-Delves-Midnight=r272; DBM-Delves-WarWithin=r272; DBM-Party-BC=r272; DBM-Party-Cataclysm=r272; DBM-Party-Dragonflight=r272; DBM-Party-Forever=r272; DBM-Party-Legion=r272; DBM-Party-Midnight=r272; DBM-Party-MoP=r272; DBM-Party-Vanilla=r272; DBM-Party-WarWithin=r272; DBM-Party-WoD=r272; DBM-Party-WotLK=r272

### semver-like

ArkInventoryRules_Auctionator=1.0.0; AtlasBIStooltips=1.0.1; AtlasLootClassic=v1.3.9; DBM-Core=12.1.23; DBM-GUI=12.1.23; DBM-Lairs-Midnight=12.1.23; DBM-Midnight=12.1.23; DBM-Raids-Midnight=12.1.23; DBM-StatusBarTimers=12.1.23; EnhanceQoL=13.21.3; EnhanceQoLBags=13.21.3; EnhanceQoLChatSocial=13.21.3; EnhanceQoLClassBuffReminder=13.21.3; EnhanceQoLCooldownPanels=13.21.3; EnhanceQoLCooldownPulse=13.21.3; EnhanceQoLDamageMeter=13.21.3; EnhanceQoLDungeonRaid=13.21.3; EnhanceQoLItems=13.21.3; EnhanceQoLMover=13.21.3; EnhanceQoLQuickActions=13.21.3; EnhanceQoLResourceBars=13.21.3; EnhanceQoLSharedMedia=13.21.3; EnhanceQoLSkinner=13.21.3; EnhanceQoLSound=13.21.3; EnhanceQoLTooltip=13.21.3; EnhanceQoLUnitFrames=13.21.3; EnhanceQoLVendor=13.21.3; ForeverFishing=0.9.130; ForeverLootSparkles=v2.0.0; GearQuestForever=1.1.6; GearQuestForever_DRUID=1.1.6; GearQuestForever_HUNTER=1.1.6; GearQuestForever_Languages=1.1.6; GearQuestForever_MAGE=1.1.6; GearQuestForever_PALADIN=1.1.6; GearQuestForever_PRIEST=1.1.6; GearQuestForever_ROGUE=1.1.6; GearQuestForever_SHAMAN=1.1.6; GearQuestForever_WARLOCK=1.1.6; GearQuestForever_WARRIOR=1.1.6; HealKit=1.10.1; HideAnything=1.10.0; MaxDps=v11.3.94; MaxDps_Priest=v11.2.16; MoveAny=1.14.16; Prat-3.0=3.9.117; PriestHUD=0.1.3; QuestieDB=1.1.1; QuestieTrace=1.11.0; RXPGuides=v4.11.36; Spoken=3.9.0; SpokenAll=3.1.0; SpokenBossesAudio=1.0.1; SpokenContributions=3.9.0; SpokenPlacesAudio=2.1.2; SpokenPlayers=0.18.0; SpokenQuestsAudioAlliance=2.7.1; SpokenQuestsAudioGossip=2.7.1; SpokenQuestsAudioHorde=2.7.1; SpokenQuestsAudioShared=2.7.1 (first 60 of 66)

### tagged-numeric

AtlasLootClassic_Collections=Forever 1.60.1; AtlasLootClassic_Crafting=Forever 1.60.1; AtlasLootClassic_Data=Forever 1.60.1; AtlasLootClassic_DungeonsAndRaids=BCC 2.5.4; AtlasLootClassic_Factions=Forever 1.60.1; AtlasLootClassic_Options=Forever 1.60.1; AtlasLootClassic_PvP=Forever 1.60.1

### unsubstituted-token

ArkInventoryRules_Example=<project-version-goes-here>

## Package grouping

Evidence: shared X-* package id (7 distinct shared-id keys), hard dependency on an installed folder (70 edges).

Multi-folder groups: 8, covering 77 (75.5%) folders. Singletons: 25.

- ArkInventory (6): ArkInventory, ArkInventoryConfig, ArkInventoryRules, ArkInventoryRules_Auctionator, ArkInventoryRules_Example, ArkInventorySearch
- AtlasBIStooltips (9): AtlasBIStooltips, AtlasLootClassic, AtlasLootClassic_Collections, AtlasLootClassic_Crafting, AtlasLootClassic_Data, AtlasLootClassic_DungeonsAndRaids, AtlasLootClassic_Factions, AtlasLootClassic_Options, AtlasLootClassic_PvP
- DBM-Brawlers (25): DBM-Brawlers, DBM-Challenges, DBM-Core, DBM-Delves-Midnight, DBM-Delves-WarWithin, DBM-GUI, DBM-Lairs-Midnight, DBM-Midnight, DBM-Party-BC, DBM-Party-BfA, DBM-Party-Cataclysm, DBM-Party-Dragonflight, DBM-Party-Forever, DBM-Party-Legion, DBM-Party-Midnight, DBM-Party-MoP, DBM-Party-Shadowlands, DBM-Party-Vanilla, DBM-Party-WarWithin, DBM-Party-WoD, DBM-Party-WotLK, DBM-Raids-Midnight, DBM-StatusBarTimers, DBM-VPVEM, DBM-WorldEvents
- EnhanceQoL (18): EnhanceQoL, EnhanceQoLBags, EnhanceQoLChatSocial, EnhanceQoLClassBuffReminder, EnhanceQoLCooldownPanels, EnhanceQoLCooldownPulse, EnhanceQoLDamageMeter, EnhanceQoLDungeonRaid, EnhanceQoLItems, EnhanceQoLMover, EnhanceQoLQuickActions, EnhanceQoLResourceBars, EnhanceQoLSharedMedia, EnhanceQoLSkinner, EnhanceQoLSound, EnhanceQoLTooltip, EnhanceQoLUnitFrames, EnhanceQoLVendor
- GearQuestForever (10): GearQuestForever, GearQuestForever_DRUID, GearQuestForever_HUNTER, GearQuestForever_MAGE, GearQuestForever_PALADIN, GearQuestForever_PRIEST, GearQuestForever_ROGUE, GearQuestForever_SHAMAN, GearQuestForever_WARLOCK, GearQuestForever_WARRIOR
- MaxDps (2): MaxDps, MaxDps_Priest
- Spoken (5): Spoken, Spoken_Bosses, Spoken_Places, Spoken_Quests, Spoken_Readables
- VuhDo (2): VuhDo, VuhDoOptions

### Version consistency inside groups (distinct versions)

- ArkInventory: 1.0.0 | 31217 | <project-version-goes-here>
- AtlasBIStooltips: 1.0.1 | BCC 2.5.4 | Forever 1.60.1 | v1.3.9
- DBM-Brawlers: 12.1.23 | <none> | a8e9f0e | r272
- MaxDps: v11.2.16 | v11.3.94

Folders that look name-related to a folder outside their confident group (cannot be grouped confidently): 25 (24.5%)

- GearQuestForever ~ GearQuestForever_Languages
- GearQuestForever_DRUID ~ GearQuestForever_Languages
- GearQuestForever_HUNTER ~ GearQuestForever_Languages
- GearQuestForever_Languages ~ GearQuestForever, GearQuestForever_DRUID, GearQuestForever_HUNTER, GearQuestForever_MAGE, GearQuestForever_PALADIN, GearQuestForever_PRIEST, GearQuestForever_ROGUE, GearQuestForever_SHAMAN, GearQuestForever_WARLOCK, GearQuestForever_WARRIOR
- GearQuestForever_MAGE ~ GearQuestForever_Languages
- GearQuestForever_PALADIN ~ GearQuestForever_Languages
- GearQuestForever_PRIEST ~ GearQuestForever_Languages
- GearQuestForever_ROGUE ~ GearQuestForever_Languages
- GearQuestForever_SHAMAN ~ GearQuestForever_Languages
- GearQuestForever_WARLOCK ~ GearQuestForever_Languages
- GearQuestForever_WARRIOR ~ GearQuestForever_Languages
- Questie ~ QuestieDB, QuestieTrace
- QuestieDB ~ Questie
- QuestieTrace ~ Questie
- Spoken ~ SpokenAll, SpokenBossesAudio, SpokenContributions, SpokenPlacesAudio, SpokenPlayers, SpokenQuestsAudioAlliance, SpokenQuestsAudioGossip, SpokenQuestsAudioHorde, SpokenQuestsAudioShared, SpokenReadablesAudio
- SpokenAll ~ Spoken
- SpokenBossesAudio ~ Spoken
- SpokenContributions ~ Spoken
- SpokenPlacesAudio ~ Spoken
- SpokenPlayers ~ Spoken
- SpokenQuestsAudioAlliance ~ Spoken
- SpokenQuestsAudioGossip ~ Spoken
- SpokenQuestsAudioHorde ~ Spoken
- SpokenQuestsAudioShared ~ Spoken
- SpokenReadablesAudio ~ Spoken

## Interface values

| interface | folders |
|---|---|
| 0 | 1 |
| 11508 | 2 |
| 11509 | 32 |
| 120007 | 1 |
| 120100 | 4 |
| 120105 | 37 |
| 120107 | 25 |
| 16001 | 82 |
| 20506 | 32 |
| 30403 | 2 |
| 30405 | 9 |
| 38000 | 1 |
| 38002 | 13 |
| 40402 | 11 |
| 50503 | 1 |
| 50504 | 22 |

## TOC file suffixes (toc files) and Interface values seen in each

| suffix | toc files | interface values |
|---|---|---|
| (plain) | 100 | 0, 11508, 11509, 16001, 20506, 30403, 30405, 38000, 38002, 40402, 50504, 120007, 120100, 120105, 120107 |
| BCC | 4 | 20506 |
| Camelot (undocumented) | 29 | 16001 |
| Cata | 1 | 40402 |
| Classic | 1 | 11509, 20506, 38002, 50504 |
| Forever (undocumented) | 10 | 16001 |
| Mists | 8 | 50503, 50504 |
| TBC | 6 | 20506 |
| Vanilla | 10 | 11508, 11509 |
| Wrath | 2 | 30403, 38002 |
