# CWT disagreement review

Milestone 5 closes only after each covered claim where Atlas and CWT disagree is reviewed: it is a
CWT error, or an Atlas or Native defect to repair (Native `docs/roadmap.md`, "Milestone 5
acceptance"). This page holds that review. Each entry names the config claim, Atlas's answer, the
evidence, and the verdict.

## Keys the engine never reads

The [not-a-field rule](ledger.md#keys-the-engine-never-reads-decision-of-2026-10-20) answers these
claims once the snapshot carries Native's `keeps_definition_text` (Native `8983b8b7`, `Known(false)`
here): Native's root-field answer for the registry is `Complete` and lists no field with the key,
and the loader keeps no definition text.
Snapshot: live M452 capture at Native `5143009` (`registry-fields/v61`).

### `common/timeline_events` (`common/timeline_events.cwt`)

Native lists seven root fields: `fire_only_once`, `gui`, `tooltip`, `tooltip_delayed`, `types`,
`override_text` and `override_texture`. The answer is `Complete`, with only the two standard
`OutsideMethod` boundaries.

| CWT key (line) | Claims | Atlas answer | Engine key | Verdict |
| --- | --- | --- | --- | --- |
| `type` (17-21) | `field_existence`, `value_form`, `cardinality_minimum`, `cardinality_maximum`, item `value_form` and item cardinality (7) | absent; `not_a_field` | `types` | CWT error: the shipped guide (`00_timeline_defines.txt`) and the reader both use `types` |
| `text` (26-28) | `field_existence`, `value_form`, `cardinality_minimum`, `cardinality_maximum`, item `value_form` (5) | absent; `not_a_field` | `override_text` | CWT error, to confirm: the shipped guide writes `text = { }` in its database example, but the reader reads only `override_text`; no shipped entry uses either |
| `texture` (30-32) | `field_existence`, `value_form`, `cardinality_minimum`, `cardinality_maximum`, item `value_form` (5) | absent; `not_a_field` | `override_texture` | As for `text` |

The loader keeps no definition text (Native `8983b8b7`, `keeps_definition_text` `Known(false)` for
`common/timeline_events`), so no later code reads `text` or `texture` from a definition; the
`add_timeline_event` effect's `override_text` and `override_texture` are command arguments, not
definition keys.

## Renamed directories

### `type[lawsuit]` (`common/lawsuits.cwt`)

| Claim | CWT | Engine | Verdict |
| --- | --- | --- | --- |
| `types/type[lawsuit]/path` (`loader_path`) | `game/common/lawsuit_type` | `common/lawsuits` | CWT error: the executable names only `common/lawsuits`, the game ships `common/lawsuits/00_example.txt` (whose guide documents `effect`, `cost` and `duration`), and Native's `common/lawsuits` registry reads those three fields. |
