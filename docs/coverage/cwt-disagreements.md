# CWT disagreement review

Milestone 5 closes only after each covered claim where Atlas and CWT disagree is reviewed: it is a
CWT error, or an Atlas or Native defect to repair (Native `docs/roadmap.md`, "Milestone 5
acceptance"). This page holds that review. Each entry names the config claim, Atlas's answer, the
evidence, and the verdict.

## Keys the engine never reads

The [not-a-field rule](ledger.md#keys-the-engine-never-reads-decision-of-2026-10-20) answers these
claims: Native's root-field answer for the registry is `Complete` and lists no field with the key.
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

Open check: whether `text` and `texture` are read by another class at use time (the
`add_timeline_event` effect takes `override_text` and `override_texture`). Native's answer covers
the database reader only.
