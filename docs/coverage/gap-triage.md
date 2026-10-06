# Unowned gap triage (SDK-626)

This page gives each published gap with a `null` owner a failure shape and a disposition. A shape
gets an owner ticket only when closing it changes what a compiler accepts, rejects, types or
completes, and a config claim depends on it. Other shapes are **out of scope, vision 2026-10-02**.
The snapshot keeps its `null` owners and its owner categories; this page carries the tickets. No gap
closes here, and coverage does not change.

## Basis

- Atlas `3b9c32552dddcad34d2fed888e6c7ec120b43de8`, which pins Native
  `b5049f90daf9db03cb52f8b51cfe645c691ac6b7`.
- Answers: the [simplification capture](simplification.md) on build
  `29fa877366040a528098da39ec7e70b7baac76782a2a6bd161616d691f86fa38`, with SDK-704's
  `on_actions.json` and `game_rules.json` from `tests/fixtures/native/m451-hotfix`.
- Config: `85747602a614ad7daa8cc66453777ecb023463a8`.
- Snapshot, recorded mode: 28,434 rules and 13,780 gaps, SHA-256
  `152bbd0d666b1db6c5bfd887f2bb3caa5be47c79d5662204a30c752629b16938`.

**6,910 gaps have no owner.** The Milestone 3 review counted 755 on an earlier snapshot. The live
simplification capture has the same 6,910 unowned gaps; SDK-704 changed only the owned
`callback_context` gaps (517 to 270). Recorded answers earn no coverage, so "uncovered" below comes
from the live capture's coverage report.

Each unowned gap belongs to exactly one shape below, and the sub-dispositions of each shape add up
to its total. "Linked claims" are the config claims that a shape's gaps attach to: field gaps use
the join of `src/coverage/projection.rs`, and other subjects join by name (`alias[effect:…]`,
`defines`, `localisation_links`, `on_actions`, `game_rules`, and modifier templates whose `<type>`
names a type of the family's registry). One claim can depend on several shapes, so do not add the
claim counts.

"On Native main" is Native `c9886a7` on the same build: its
`tests/population/m451-hotfix/registry-field-sweep.json` and one `derived-name-population` run. It
shows the answers that Native gives after the pin but Atlas does not carry yet. SDK-625 carries
them and measures this table again on its final run.

## Shapes

| Shape (gap properties) | Gaps | Linked claims | Compiler consequence | Disposition |
| --- | ---: | ---: | --- | --- |
| Required field (`occurrences.minimum`) | 1,605 | 1,235 | Missing-field diagnostics | SDK-627 |
| Repeat behavior (`occurrences.maximum`) | 1,025 | 817 | Duplicate-field diagnostics | SDK-627 |
| Reader value form (`value_form.unresolved`, `value_form`) | 523 | 463 | Scalar, block and type acceptance | [Below](#reader-value-form) |
| Nested grammar (`nested_grammar`) | 422 | 393 | Accepted child keys | [Below](#nested-grammar) |
| Field block scope (`scope_context`) | 422 | 226 | `this`, `root` and `from` types | [Below](#field-block-scope) |
| Lookup target (`reference`) | 284 | 259 | Completion and invalid-reference diagnostics | [Below](#lookup-target) |
| Command declared scopes (`declared_scopes`, `native.effect.unresolved_path`, `native.trigger.unresolved_path`) | 82 | 47 | Where a command is valid | SDK-711 |
| Callback entry contexts (`native.on_actions.*` and `native.game_rules.*` paths and unnamed sites) | 252 | 131 | Callback scope checking | SDK-712. Out of scope, vision 2026-10-02: on_actions that content fires, names built at run time, and helpers whose scope type is a run-time value |
| Modifier family generation (`generation`, `native.modifier_families.unresolved_path`, `category_tags`) | 62 | 44 | Which items make a template name valid | [Below](#modifier-families) |
| Unjoined generation sites (`native.modifier_families.unnamed_declaration` and `.outside_method`, `native.modifiers.unnamed_declaration`) | 329 | — | Template-name acceptance | [Below](#modifier-families) |
| Define readers (`native.defines.unresolved_reader`) | 80 | 148 | Define key and type acceptance | SDK-610 |
| Localisation link outputs (`alternatives`, `native.localization.unresolved_path`) | 32 | 16 | Completion after a link | SDK-609 |
| Partial root search (`fields_complete`) | 156 | — | Unknown-key diagnostics | [Below](#partial-root-search) |
| Fixture outcome (`occurrences.parser_accepted`, `storage`) | 1,615 | — | None: parser storage of one fixture | Out of scope, vision 2026-10-02 |
| Method boundaries (ten inventory `outside_method` gaps, eight loaded-modifier gaps) | 18 | — | None | Out of scope, vision 2026-10-02: Atlas reads content-defined names from content, and loaded names are observed |
| Ambiguous gap subjects (`field_gap_subjects`) | 3 | — | None directly | SDK-625: Native gap subjects name full paths since SDK-686 |

### Reader value form

- 51 gaps (40 claims): Native main reads these fields as `Block` (triggered modifier clauses,
  SDK-673). SDK-625.
- 10 gaps (9 claims): scoped-numeric and `Float` readers that Native establishes and Atlas does not
  map to a value form. SDK-625.
- 462 gaps (414 claims, including 182 `bool` and 137 `block`): no shared reader on main, or a color
  or vector reader of unknown kind. SDK-710.

### Nested grammar

Atlas publishes this gap for every block field, whatever Native answers.

- 244 gaps (220 claims): `Trigger` or `Effect` family. SDK-625 credits the family against the
  `trigger_clause` and `effect_clause` claims; the members stay unresolved.
- 107 gaps (103 claims): established members (`WeightBlock`, `ModifierBlock`, `Fields`). SDK-625.
- 18 gaps (18 claims): an identified reader with an unknown family. SDK-676 classifies them;
  confirmed modifier uses close there, and the others go to SDK-710.
- 53 gaps (52 claims): no reader identity. SDK-710.

### Field block scope

Atlas publishes this gap for every block field, whatever Native answers.

- 108 gaps (68 claims): entry contexts on main with no scope gap (SDK-549, SDK-677). SDK-625.
- 191 gaps (99 claims): part of the context on main. 137 have only a read scope, which gives
  `this` but not `root` or `from`; the others keep an unresolved entry context. SDK-625 carries the
  established part; SDK-712 owns the rest. Weight blocks are outside the SDK-677 method.
- 89 gaps (36 claims): no readable context: the scope is an argument from an unfollowed caller, the
  read-scope mask is zero, or no direct call evaluates the block. SDK-712.
- 34 gaps (23 claims): modifier blocks. Out of scope, vision 2026-10-02: a compiler checks modifier
  keys by the accepted categories of their container (SDK-708), which SDK-625 carries.

### Lookup target

- 29 gaps (26 claims): Native establishes the lookups; Atlas does not read `Field.reference` yet.
  SDK-625.
- 104 gaps (113 claims) where the config names a `<type>`, and 1 `name_format`. SDK-635.
- 35 gaps (38 claims): localisation, file or sprite names that `derived_names` reaches on main.
  SDK-625 credits the lookup target.
- 62 gaps (63 claims): localisation, file or sprite fields whose lookup target this run does not
  establish, such as `custom_tooltip`. The lookup is outside the owner's methods, or an internal
  path is unresolved (an unreached call, an unfollowed name part, a path limit, a string object).
  SDK-707.
- 5 gaps (6 claims): the config states an enum or one literal. SDK-627 (domains).
- 1 gap (1 claim): `$shader_effect`, a name from graphics files. SDK-554.
- 47 gaps (11 claims): the config states `scalar`, or no claim joins. Out of scope, vision
  2026-10-02: no lookup claim depends on them.

The unresolved conditions and miss behavior of a reached name are out of scope, vision 2026-10-02:
the claim needs the lookup target, and a compiler, not this data, selects the severity of a
missing key.

### Modifier families

- 22 `Unresolved` families back a config template: 22 name and 22 category claims. SDK-713, which
  also owns the `<leader_class>`, `<espionage_category>`, `<zone>`, `<economic_category>` and
  `enum[ship_class]` templates that have no joined family or site.
- Ship-size templates are SDK-678. Strategic resource and planet-class joins are SDK-551 AC6.
- Out of scope, vision 2026-10-02: 17 `Unresolved` families with no template claim; category tags
  that an item's own `modifier_category` field supplies; the dynamic-modifier and weapon-tag sites,
  which no template uses; and the method boundary repeated on each registry.

### Partial root search

Atlas publishes `fields_complete` for every partial field answer, and its reason says that the root
search is partial. Classify it by the attached Native gaps:

- 65 registries have a path that the method could not follow, an unnamed key or unreadable input.
  Root fields can be missing there: 499 of their 1,135 root `field_existence` claims are uncovered.
  SDK-710; SDK-553 measures the result.
- 91 registries have no such discovery failure; their causes are field readers, storage, numeric
  conversion or use-time selections. Each cause has the owner of its field shape: unresolved
  storage or repeat behavior is SDK-627; an unresolved or unclassified reader is SDK-710. Numeric
  conversion limits (overflow, token boundary, trailing text, the C library) are out of scope,
  vision 2026-10-02: SDK-544 closed with them, and the config claims only the established form.
  Unresolved use-time selections are out of scope, as the ledger's `field_conditions` row says.

## Owned categories

The owner categories have their own [ticket mapping](ledger.md#shared-reader-answers). The
[simplification capture](simplification.md#sdk-626-failure-shapes) records their shapes and the
decisions on runtime modifier application and category `supported_scopes`.

## Reproduce

Copy the simplification capture's answers to `.scratch/answers`, then:

```sh
cp tests/fixtures/native/m451-hotfix/{on_actions,game_rules}.json .scratch/answers/
cargo run --release --locked -- snapshot --recorded .scratch/answers .scratch/rules.json
cargo run --release --locked -- ledger --config "$PDX_CONFIG_PATH" --snapshot .scratch/rules.json --output .scratch/ledger
```
