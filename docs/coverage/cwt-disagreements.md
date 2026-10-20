# CWT disagreement review

Milestone 5 closes only after each covered claim where Atlas and CWT disagree is reviewed: it is a
CWT error, or an Atlas or Native defect to repair (Native `docs/roadmap.md`, "Milestone 5
acceptance"). This page holds that review. Each entry names the config claim, Atlas's answer, the
evidence, and the verdict.

## Renamed directories

### `type[lawsuit]` (`common/lawsuits.cwt`)

| Claim | CWT | Engine | Verdict |
| --- | --- | --- | --- |
| `types/type[lawsuit]/path` (`loader_path`) | `game/common/lawsuit_type` | `common/lawsuits` | CWT error: the executable names only `common/lawsuits`, the game ships `common/lawsuits/00_example.txt` (whose guide documents `effect`, `cost` and `duration`), and Native's `common/lawsuits` registry reads those three fields. |
