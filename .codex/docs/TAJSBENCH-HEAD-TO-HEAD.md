# TajsBench: four-input display comparison

2026-10-16 · First **up to 25** outputs in desktop display order, not a global score list.

**Read the middle column carefully:** Phase 7 has no complete alternative solver yet. It is an **order-only replay**, choosing wording from each native bag’s retained orders while keeping that bag’s native position. It cannot reveal new bags, change their ranks, or establish Phase 7’s full top-25 recovery. The outer columns are actual recorded solver responses.

- **Baseline:** current native algorithm, on the experimental lexical60 dictionary used in these experiments—not the installed default dictionary.
- **Phase 7 replay:** broader Tatoeba-plus-books 5-gram; the previously tested `hybrid10` rule, native order objective + 0.10 × mean LM log10 per word, no OOV gate. Ties keep the retained-order index. No per-answer adjustment.
- **Broader LM + Phase 8:** actual bounded ordered-search prototype, beam 64 / 1,000,000 word checks / 10-second deadline. The LM guides search; this is not a stack of both distinct scoring formulas.
- **Display:** ascending word-count groups, then the captured rank within each group, ten results per group. `(3w #4)` means fourth in the three-word group. Table rows align screen positions, not matching word bags. A dash means that column has no further output.
- These are exact captured/replayed wordings, not quality endorsements. Intended recovery and usefulness of alternatives remain separate. No new judgments were inferred.

## TestingAnagramSolver

Recorded input: `testinganagramsolver`.

Available outputs: **10 / 10 / 0** (baseline / Phase 7 replay / Phase 8).
Native stop: `candidate_cap`; Phase 8 stop: `work_cap`. One recorded call: native 1.46 s, Phase 8 2.72 s. Phase 7 replay has no comparable end-to-end latency measurement.

| Position | Baseline — captured | Phase 7 — order-only replay | Broader LM + Phase 8 — captured |
| ---: | --- | --- | --- |
| 1 | several migrants tango · (3w #1) | several migrants tango · (3w #1) | — |
| 2 | arranging moves latest · (3w #2) | arranging moves latest · (3w #2) | — |
| 3 | governing states alarm · (3w #3) | governing states alarm · (3w #3) | — |
| 4 | governing alarms state · (3w #4) | governing alarms state · (3w #4) | — |
| 5 | engraving rates almost · (3w #5) | engraving rates almost · (3w #5) | — |
| 6 | several organs matting · (3w #6) | several organs matting · (3w #6) | — |
| 7 | migrants are galveston · (3w #7) | migrants are galveston · (3w #7) | — |
| 8 | game given translators · (3w #8) | game given translators · (3w #8) | — |
| 9 | grass moving alternate · (3w #9) | grass moving alternate · (3w #9) | — |
| 10 | starting loves manager · (3w #10) | starting loves manager · (3w #10) | — |

## TestingShit

Recorded input: `TestingShit`.

Available outputs: **32 / 32 / 28** (baseline / Phase 7 replay / Phase 8).
Native stop: `exhausted`; Phase 8 stop: `beam_pruned`. One recorded call: native 1.04 s, Phase 8 1.55 s. Phase 7 replay has no comparable end-to-end latency measurement.

| Position | Baseline — captured | Phase 7 — order-only replay | Broader LM + Phase 8 — captured |
| ---: | --- | --- | --- |
| 1 | this setting · (2w #1) | this setting · (2w #1) | the sittings · (2w #1) |
| 2 | this testing · (2w #2) | this testing · (2w #2) | est shitting · (2w #2) |
| 3 | testing hits · (2w #3) | testing hits · (2w #3) | stein tights · (2w #3) |
| 4 | setting hits · (2w #4) | setting hits · (2w #4) | tights inset · (2w #4) |
| 5 | the sittings · (2w #5) | the sittings · (2w #5) | het sittings · (2w #5) |
| 6 | set shitting · (2w #6) | set shitting · (2w #6) | tights stine · (2w #6) |
| 7 | hitting sets · (2w #7) | hitting sets · (2w #7) | eth sittings · (2w #7) |
| 8 | shit testing · (2w #8) | shit testing · (2w #8) | tights tines · (2w #8) |
| 9 | shit setting · (2w #9) | shit setting · (2w #9) | tights nites · (2w #9) |
| 10 | settings hit · (2w #10) | hit settings · (2w #10) | ting theists · (2w #10) |
| 11 | this tin gets · (3w #1) | this tin gets · (3w #1) | the tits ings · (3w #1) |
| 12 | its net sight · (3w #2) | its net sight · (3w #2) | the tits gins · (3w #2) |
| 13 | its gets thin · (3w #3) | its gets thin · (3w #3) | the stint gis · (3w #3) |
| 14 | test in sight · (3w #4) | test in sight · (3w #4) | the gis tints · (3w #4) |
| 15 | its get hints · (3w #5) | its get hints · (3w #5) | the gist nits · (3w #5) |
| 16 | its gets hint · (3w #6) | its gets hint · (3w #6) | the gist snit · (3w #6) |
| 17 | thing is test · (3w #7) | thing is test · (3w #7) | the nit gists · (3w #7) |
| 18 | test is night · (3w #8) | test is night · (3w #8) | the nits gits · (3w #8) |
| 19 | test sign hit · (3w #9) | test sign hit · (3w #9) | the snit gits · (3w #9) |
| 20 | sent is tight · (3w #10) | sent is tight · (3w #10) | tits is ghent · (3w #10) |
| 21 | its get is nth · (4w #1) | its get is nth · (4w #1) | the gis is tnt · (4w #1) |
| 22 | his get is tnt · (4w #2) | his get is tnt · (4w #2) | the sgt is nit · (4w #2) |
| 23 | his tnt gets i · (4w #3) | his tnt gets i · (4w #3) | the git is nts · (4w #3) |
| 24 | it is nth gets · (4w #4) | it is nth gets · (4w #4) | the ing sst it · (4w #4) |
| 25 | the git is nts · (4w #5) | the git is nts · (4w #5) | the gis nts it · (4w #5) |

## Hips Don’t Lie

Recorded input: `dehiilnopst`.

Available outputs: **20 / 20 / 20** (baseline / Phase 7 replay / Phase 8).
Native stop: `candidate_cap`; Phase 8 stop: `beam_pruned`. One recorded call: native 1.21 s, Phase 8 1.52 s. Phase 7 replay has no comparable end-to-end latency measurement.

| Position | Baseline — captured | Phase 7 — order-only replay | Broader LM + Phase 8 — captured |
| ---: | --- | --- | --- |
| 1 | point shield · (2w #1) | point shield · (2w #1) | delhi piston · (2w #1) |
| 2 | delhi points · (2w #2) | delhi points · (2w #2) | hotline dips · (2w #2) |
| 3 | site dolphin · (2w #3) | site dolphin · (2w #3) | til siphoned · (2w #3) |
| 4 | dolphins tie · (2w #4) | dolphins tie · (2w #4) | til sphenoid · (2w #4) |
| 5 | dolphin ties · (2w #5) | dolphin ties · (2w #5) | delhi points · (2w #5) |
| 6 | nihil posted · (2w #6) | nihil posted · (2w #6) | the ons lipid · (3w #1) |
| 7 | polished tin · (2w #7) | polished tin · (2w #7) | the nil ipods · (3w #2) |
| 8 | shield pinot · (2w #8) | shield pinot · (2w #8) | the lisp dion · (3w #3) |
| 9 | dipole hints · (2w #9) | dipole hints · (2w #9) | the nos lipid · (3w #4) |
| 10 | shield pinto · (2w #10) | shield pinto · (2w #10) | the lin ipods · (3w #5) |
| 11 | point is held · (3w #1) | point is held · (3w #1) | the pins lido · (3w #6) |
| 12 | pinot is held · (3w #2) | pinot is held · (3w #2) | the lisp dino · (3w #7) |
| 13 | pinto is held · (3w #3) | pinto is held · (3w #3) | the pins lodi · (3w #8) |
| 14 | hips don't lie · (3w #4) | hips don't lie · (3w #4) | the lisp odin · (3w #9) |
| 15 | lei don't ship · (3w #5) | lei don't ship · (3w #5) | the ponds lii · (3w #10) |
| 16 | points held i · (3w #6) | i held points · (3w #6) | the dis pol in · (4w #1) |
| 17 | its hold pine · (3w #7) | its hold pine · (3w #7) | the isp nil do · (4w #2) |
| 18 | this lip done · (3w #8) | this lip done · (3w #8) | the isp lin do · (4w #3) |
| 19 | hind lies top · (3w #9) | hind lies top · (3w #9) | the nil psi do · (4w #4) |
| 20 | depth is lion · (3w #10) | lion is depth · (3w #10) | the lin psi do · (4w #5) |

## I Am Lord Voldemort

Recorded input: `iamlordvoldemort`.

Available outputs: **20 / 20 / 8** (baseline / Phase 7 replay / Phase 8).
Native stop: `candidate_cap`; Phase 8 stop: `beam_pruned`. One recorded call: native 1.40 s, Phase 8 1.60 s. Phase 7 replay has no comparable end-to-end latency measurement.

| Position | Baseline — captured | Phase 7 — order-only replay | Broader LM + Phase 8 — captured |
| ---: | --- | --- | --- |
| 1 | immortal drove old · (3w #1) | old drove immortal · (3w #1) | dev oral mod rom til · (5w #1) |
| 2 | marmot rolled void · (3w #2) | marmot rolled void · (3w #2) | dev oral mit rom dol · (5w #2) |
| 3 | mold trim overload · (3w #3) | overload trim mold · (3w #3) | del rom var mod toil · (5w #3) |
| 4 | immortal lord dove · (3w #4) | immortal lord dove · (3w #4) | dev oral mit rom dlo · (5w #4) |
| 5 | treadmill do vroom · (3w #5) | treadmill do vroom · (3w #5) | del rom var mod loti · (5w #5) |
| 6 | middle motor valor · (3w #6) | middle motor valor · (3w #6) | dev oral mit rom old · (5w #6) |
| 7 | dollar moved timor · (3w #7) | timor moved dollar · (3w #7) | aol dev rom mit lord · (5w #7) |
| 8 | dol drove immortal · (3w #8) | dol drove immortal · (3w #8) | dev oral mod rom lit · (5w #8) |
| 9 | immortal loved rod · (3w #9) | rod loved immortal · (3w #9) | — |
| 10 | motor molded rival · (3w #10) | motor rival molded · (3w #10) | — |
| 11 | a drill moved motor · (4w #1) | a drill moved motor · (4w #1) | — |
| 12 | i lord moved mortal · (4w #2) | i moved mortal lord · (4w #2) | — |
| 13 | it moved moral lord · (4w #3) | it moved moral lord · (4w #3) | — |
| 14 | mall do drive motor · (4w #4) | mall do drive motor · (4w #4) | — |
| 15 | dollar do move trim · (4w #5) | dollar do move trim · (4w #5) | — |
| 16 | timor am loved lord · (4w #6) | lord am loved timor · (4w #6) | — |
| 17 | drill am moved root · (4w #7) | root am moved drill · (4w #7) | — |
| 18 | motor devil am lord · (4w #8) | lord am motor devil · (4w #8) | — |
| 19 | tirol am moved lord · (4w #9) | tirol am moved lord · (4w #9) | — |
| 20 | lord do live marmot · (4w #10) | lord do live marmot · (4w #10) | — |

## Evidence and limits

Both captured arms use the same stored generation request and dictionary. The baseline is not a Python reference. Requests allow one to six words, no required/excluded words, minimum Zipf 1.8, common short-word policy plus `hi`, and no optional repetition. Native generation is capped at 10,000 candidates. The prototype’s word-check cap uses a different work unit.

The higher-work primary capture retains every original attempt and fills only previously missing case/treatment pairs; it does not pick the best rerun. Full raw results, replay scores and input SHA-256 identities are in `.codex/temp/head-to-head-2026-10-16/comparison.json`. The generator is `build.cjs` beside it; run `node .codex/temp/head-to-head-2026-10-16/build.cjs` from the repository root. The capture’s hash is checked against the frozen Phase 8 summary before rendering.

Independent checks cover exact letters, rendered word bags, duplicate bags, matching requests, ascending bucket order, per-bucket limits, complete retained-order coverage of these displayed native bags, and frozen Phase 7 bag positions. No production code, settings, model package or installed app was changed.

**Still unmeasured:** a complete Phase 7 integration that can rerank every generated bag before the display cutoff. Do not read the middle column as that experiment. Likewise, a newer completion-aware Phase 8 frontier has not been run; these are the existing prototype’s outputs, shown before changing it.

### Input identities

| Input | SHA-256 |
| --- | --- |
| .codex/temp/phase7-followup-2026-10-16/ordered-highwork-complete.jsonl | `3edd44d3abcd521610d458e04570a2a795c680857319739b2f405d40979623e4` |
| .codex/temp/phase7-2026-10-15/ordering-inputs.json | `2d33ce2cb29cc28e5e09edd0c28c45bacd446e45cb054b3546581001a8929be9` |
| .codex/temp/phase7-2026-10-15/ordering-pools.jsonl | `0294e0950153a8032ff29533bf76fd7cd7a815b01df8c51646bfaa205eaa4bc2` |
| .codex/temp/phase7-followup-2026-10-16/ngram-scores.json | `9771e32d012853a4633d212aab4986f5b6b474fbed2ff180dadd9c917e510e2e` |
| .codex/temp/phase7-followup-2026-10-16/mixed5.arpa | `6ca655362d1d9ac1898f8c324189dafd3c6f3dda68cf3a506ff11c96db51b4c9` |
| apps/desktop/src/Results.svelte | `00839fd266e689f957e87a6602679e936217bcd61a1b274e8b1e20c29888b112` |
