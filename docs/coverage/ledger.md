# Config claim ledger contract

SDK-523 produces an inventory of source questions. A source assertion says what CWT claims; it is
not proof of game behavior. Atlas coverage asks whether the snapshot has established an answer to
the corresponding question, within its stated context. Agreement is not part of the calculation.

## Source and identity

`ledger::inventory` accepts a sorted map of relative filenames to decoded source text. The CLI reads
every `.cwt` below the supplied config root. It rejects empty inputs, unreadable/non-UTF-8 files,
and symlinks rather than silently omitting them. UTF-8 byte hashes identify the actual inputs.

Each claim records a subject path, property, structural conditions, the CWT assertion, exact source
text/span, owner and rationale, and an expected roadmap method for engine-fact questions. Source
spans are half-open UTF-8 byte offsets with one-based physical line numbers. Type and field presence
are separate from value forms, occurrence bounds, scopes, and documentation. Adjacent documentation
lines attached to one entry form one documentation claim. Unknown annotations retain their text
and a diagnostic; expected method assignment does not prove that the method already works.
Minimum and maximum cardinality are separate questions. Each soft bound belongs to consumer policy;
the other bound can still be an engine fact. Define-derived bounds retain a separate reference question.
The block around `type[...]` is CWT metadata, not a claim that game definitions must be blocks.
Value-form questions come from the separate schema entries. Subtype selectors inside type metadata
express field predicates; subtype arms inside a schema retain the base field, alias, or content
questions and their ownership, with the subtype carried in their conditions.

The question ID hashes relative file, subject path, property, and structural conditions. Occurrence
IDs append an ordinal to distinguish repeated declarations of the same question. CWT answer text
and line positions do not affect those IDs. Bare values use their position among bare siblings as
the subject segment. Annotation subjects include the annotation name, so distinct scope operations
cannot share credit merely because both concern scope context. Moving a file, renaming a subject,
or reordering bare items can change identity;
this first format does not claim identity across those changes.

The denominator counts source claim occurrences, not unique engine facts. One qualified question
answer can cover repeated source occurrences. Alias expansion does not duplicate the shared grammar,
and source-line links and closing braces do not create extra claims. CWT grouping, subtype naming
and tests, and alias factoring are explicit consumer-policy claims. A complete negative answer can cover a
presence question without establishing any of its value/cardinality questions. Atlas-only questions
are listed separately and do not change the config denominator.

`files[].lines` accounts for every meaningful physical line with claim IDs, diagnostic IDs, or both.
Blank lines, ordinary single-hash comments, and decorative comments with more than four hashes are
excluded; quoted multiline text remains meaningful even when a line starts with `#`. Semantic `##`
annotations and `###`/`####` documentation are included. Delimiters link to the enclosing claim.
Malformed syntax and unresolved annotations set `inventory_complete=false` and produce exit 2 after
writing both reports. Claims under known structural subtype contexts carry those context labels;
this does not qualify the underlying game condition.

## Ownership and routes

| Class | Examples | Authority |
| --- | --- | --- |
| Engine fact | Fields, accepted forms, hard cardinality, references, scopes | Qualified engine evidence |
| Content-derived | Shipped-comment documentation and content inventories | Content observations |
| Consumer policy | Severity, soft bounds, subtype naming and tests, aliases, scope groups, content queries | Consumer modeling choices |
| Authored text | Unattributed documentation outside expected engine/content families | Its author |

Documentation assignments are provisional; the historical [provenance measurement](documentation.md)
compared every entry against explicit engine dumps and installed comments. It tagged all
entries, recorded matching source locations, separated rewritten candidates, and listed the authored
remainder. Current ledger output keeps the expected-owner assignments provisional. A text match
does not move a claim between classes; it answers a content-derived documentation question only
as [below](#content-derived-documentation-sdk-718).
Unknown metadata has a provisional policy assignment plus a diagnostic; it is not declared
inherently manual.

Classification and expected method families are centralized in `src/ledger/classify.rs`.
Registry discovery, field discovery, reader binding, language declarations, shapes,
nested grammar, references, numerics, weights, names, loader selection, and separate
formats each have a route. A route is not a qualification or a game-rule answer.

## Snapshot input

The production `pdx-atlas snapshot` command writes `atlas_rule_snapshot` contract version 2.
Version 2 adds language subjects (`effect:`, `trigger:`, `modifier:`, `scope:`, `define:` and
the others in the schema) and states each Native answer once in `answers`, with its source,
completeness and typed gaps; an evidence link names its answer and keeps only the gaps that name
its typed subject. A named Native gap has a subject kind; localization contexts and scope types
also carry stable IDs, so equal display names do not merge. Version 1 had registry and field subjects only, was never published, and is no longer
read. The ledger accepts the file directly and projects its registry and field rules onto matching
config questions. The join comes from the ledger's own `loader_path` claims; the rule producer
does not read CWT. Each rule keeps Native's source stamp, completeness and typed gaps. Field
joins use typed subject fields, including registry-relative nested paths. Recorded sources
are unqualified for current-engine coverage. A matching gap blocks credit for its
question. Unknown contract versions or broken subject, source or schema references are errors.
The full format is [rule-snapshot-v2.schema.json](../contract/rule-snapshot-v2.schema.json).

Language rules join by the config's structure: `alias[effect:NAME]` command, scope and
documentation claims; `modifiers.cwt` names and categories; `links.cwt` names, input and output
scopes and data prefixes; localisation commands and links; `game_rules.cwt` names and
`replace_scopes`; and `common/defines` names and value forms. A command's own `value_form` claim
joins its `forms`, and a scope annotation on the command joins its `scope_context`. Each key that
Native's command grammar establishes is an `argument:{command}/{key path}` subject with the same
facets as a registry field, and claims under that key join it through the field question table.
A claim under a key that the grammar does not establish joins the command's `arguments` gap,
which exists only while the grammar's keys are partial or unresolved: an established argument
never credits an unresolved sibling. Where CWT identifies a subject only by a value, that value
selects the question and never decides credit: an `on_actions.cwt` item's name, a `scopes.cwt`
scope's aliases, which must match the keywords of exactly one Native scope type or group, and a
type's `localisation` or `images` naming line. A naming line joins the one
`derived_name:{registry}/{lookup}/{name}` subject whose name it writes (`$` for the item key; a
bare field name for the text of that field), in the registry of the type's `path`. A naming
question whose occurrences state different values, such as `desc = "$_desc"` and `desc = desc`
in one type, joins none. A game rule's
evaluation kind does not answer its authored value form. The loaded modifier count summary is a
content observation under a `content:` condition and credits no unconditional question.
Snapshot assembly refuses a failed registry-discovery answer. It runs authored fixture recipes
only for registries in the discovery answer, so an unrelated static listing does not require
tradition fixture recordings.

A directory can hold several config types (SDK-785). Each type then earns credit only through
the selector that the config writes for it, read from its `## type_key_filter`, `skip_root_key`
and `base_type` lines; Native never reads a selector.

- The root type is the one type with no `type_key_filter = KEY` and no `skip_root_key`. A
  `type_key_filter <> KEY` only excludes keys, so `species_name` (`<> named_lists`) is the root
  type of `common/species_names`. Registry and field rules join it as they join a type that is
  alone in its directory, and a `root_key[KEY]` condition reaches its subtypes as above; the
  subtypes of the directory's other types do not make that key ambiguous. `event`
  is the root type of `events`.
- A nested type has `skip_root_key = any` (then any further keys), a `base_type` that names the
  root type, and `type_key_filter = KEY`. Its definitions are objects inside the root type's
  definitions, at the field path of the further keys and `KEY` (`tradition_swap`;
  `swappable_data/swap_type` for `swapped_job`). Native gives such an object no identity of its
  own: it stays a field under its owner's path (Native's `registry-fields.md`, "Registry names",
  SDK-778). So the type is identified only when Native establishes that field's existence
  without conditions. That rule answers the type's `type_existence`, and the registry's
  `loader_path` rule answers its `path`. Other registry and field rules do not reach the type,
  and its naming lines join no derived name: Native's derived names describe the owner's items.
- A type that a root key selects whose key Native answers as a root statement of its own is
  answered through that statement ([below](#root-statements-of-their-own-type-sdk-818)).
- Every other type keeps a gap on each of its questions that names its selector, such as
  "The type is selected by `type_key_filter = namespace`; no Native answer identifies it". This
  covers a type that a root key selects (`event_namespace`, whose definitions are `namespace = x`
  scalars with `name_field = "-"`; `diplo_phrase_greetings`; the `random_list` types other than
  `planet_class_random_list`), a
  `type_key_filter` list, a type
  inside a wrapper key with no nested base (`species_named_list` with `skip_root_key =
  named_lists`), a nested type whose field Native does not establish, and a second type with no
  selector. No event-kind rule reaches `event_namespace`: it has no subtypes.

A type that a root key selects would join through a `root_key[KEY]` condition, as a subtype does.
Native publishes no such answer for a whole type today: `namespace` is a key of the events file
loader, not a field of `CEvent::ReadMember`, and SDK-791 owns definition-name rules. Add that join
when Native answers one of those root keys.

### Event kinds and pre_trigger keys (SDK-793)

Native answers each event kind as a root field of `events` (Native's `registry-fields.md`,
"Event kinds"): `country_event` is read in the country scope, and its members are the `CEvent`
fields. In `events` and `common/component_templates`, and only there, a root field whose key
alone selects a subtype of the root type (the SDK-800 rule below) is a root statement, not a
field. Its path below the key joins with
the condition `root_key[KEY]`, so `country_event/pre_triggers` answers
`event/subtype[country]/pre_triggers`, and the kind's own `read_scope` answers
`types/type[event]/subtype[country]/$annotation:push_scope`. Its gaps join the same questions.
Native answers each component template class the same way (its `registry-fields.md`, "No owner
(SDK-843)"): `weapon_component_template/missile_speed` answers
`component_template/subtype[weapon_component_template]/missile_speed`. In every other registry a
root field is a key inside a definition, whatever its name; extend the registry list only when
Native answers another registry's root statements as fields.

Every root statement is a definition of the root type, so the config's type-wide definition block
(`event = { … }`, outside any subtype) describes each kind. An unconditional rule on a root
statement itself answers that block's unconditional claims as a field rule answers a field's: its
existence answers `field_existence` and its value form `value_form` (SDK-815). The block holds
for every kind, so the answer takes the agreement rule below: every statement that the config
requires must answer alike, and a gap on any kind blocks the credit. On M452 all 22 kinds that Native
lists are blocks with the one `CEvent::ReadMember` reader and the same 30 members; `pop_event`,
which the config does not name as a kind, is no root statement and answers nothing.

A field below the root statements answers the root type's claims that no subtype condition
limits (`event/is_triggered_only`, `component_template/key`) only when every statement agrees.
The statements are those that the config requires: each key whose `type_key_filter` alone
selects a subtype of the root type. A configured key that Native does not answer prevents the
agreement. Each one
must answer the property for the field, with no rule conditions and the same value, and each must
give the field the same read `conditions`. A statement that lacks the field or disagrees gives no
credit, and a gap on any statement's field blocks it. Every statement's answer and read
`conditions` must rest on qualified evidence; one recorded premise gives the agreement none. The agreed answer reaches the claims that
an ordinary field rule reaches: the `alias[X:KEY]` claims of an alias block, the members of an
`enum[X]` domain, and no count above 1 from a requirement. When the field is always read in every
statement, the SDK-716 rule below also reaches its claims under value subtypes
(`event/subtype[hidden]/…`), with the same agreement. On the live snapshot at Native `4b4e1d6`
these two rules credit 129 claims and lose none: 85 `event` claims (31 of them under value
subtypes) and 44 `component_template` claims.

A block's `read_scope` that Native gives as a known empty list (the block is read with no scope,
as for the `event` kind) is the rule `[]`, not a gap. It credits `scopeless` (`push_scope = any`).

When the config writes `alias_name[X] = alias_match_left[X]` directly in a block, each Native
field in that block answers the claims at `alias[X:KEY]` for its key, with no conditions: the
alias holds wherever it is used. So the keys of `possible_pre_triggers`, `can_join_pre_triggers`
and each event kind's `pre_triggers` answer the `pre_triggers.cwt` value forms. A key that the
config does not list stays Atlas-only, and a config key that Native does not list has no answer.
Two blocks that use one alias both answer it, and an applicable gap of either blocks the credit.
On M452 every pre_trigger key keeps Native's unresolved reader, so these claims stay gaps until
Native establishes the value reader.

A field whose config value is `single_alias_right[X]` stands for the block at `single_alias[X]`.
The Native field answers the claims on `single_alias[X]` (its value form), and each Native field
below it answers the claim below `single_alias[X]` at the same path, with no conditions; a `domain`
there answers the members of the alias field's `enum[X]`. A field inside one alias that names
another (`modifier = single_alias_right[modifier_clause]` in an aura) answers its own claim in the
first alias and the claims of the second. A field written with two single aliases answers both.
Every use answers the alias, and an applicable gap on any use blocks the credit, as for alias
blocks above. A use under a subtype that no root key selects takes the always-read rule
(SDK-716) below: an always-read field answers through it, and its gaps block. A root statement's
type ([`random_list`](#root-statements-of-their-own-type-sdk-818)) joins its fields below the
key. The fields of a root statement in `events` or `common/component_templates` reach these claims
through the agreement rule above, so `component_template/hostile_aura` answers
`single_alias[component_template_hostile_aura]` when every component template statement reads the
aura alike. A block written with `alias_name[X]` inside a single alias is not followed. On the live
snapshot at Native `4b4e1d6` this credits 4 claims and loses none: Native lists no aura members
there, and a gap on one use blocks 17 claims.

### Root statements of their own type (SDK-818)

Native answers `random_list` in `common/planet_classes` as a root field that is a root statement,
not a member of a planet class: its members are the fields of `CPlanetClassRandomList` (Native's
`registry-fields.md`, "Root-key owners (SDK-813)"). In the list of such (registry, key) pairs in
`projection.rs`, and only there, the type whose selector is `type_key_filter = KEY` with no
`skip_root_key` takes its answers from the statement, when Native establishes the statement's
existence without conditions:

- The statement's existence answers the type's `type_existence`, and its existence and value form
  answer the type's definition block (`planet_class_random_list = { … }`).
- Its path below the key answers the type's fields: `random_list/name` answers
  `planet_class_random_list/name`.
- Registry rules (`loader_path`, `extension`, `recursion`) reach the type as they reach the root
  type; `definition_names` reaches it only through a `RootKey(KEY)` form. The registry's own
  existence does not answer the type's existence.
- No other type of the directory takes the statement or its fields: a planet class has no
  `random_list` member. This holds when the statement's type is alone in its directory, and when
  the config declares no type for the key.

Extend the list only when Native answers another root key as a statement of its own.

### Definition lines (SDK-860)

A type's definition line (`ship_size = { … }`, outside any subtype) states that the type has
definitions (`field_existence`), that each is a block (`value_form`), and for some types the scope
of the whole definition (`## push_scope`, `## replace_scopes`).

- **Existence.** A registry's `definition_names` rule answers the definition line's
  `field_existence` with `true` for the root type and for a type that a root key selects: the
  form that names the type's naming lines (above) names its definitions, so they exist. A known
  empty list names no definition and answers nothing. A `definition_names` gap joins the same
  question and blocks it; the `.partial` gap joins no question, as for the naming lines, so the
  stated root form of civics and jobs still answers. A selector gap still blocks a type that a
  root key selects. A root statement's own type
  ([above](#root-statements-of-their-own-type-sdk-818)) and the root type of the event kinds
  (SDK-815) are answered by the statements, so the naming rule adds no answer there. Native
  identifies a nested type only as a field under its owner (SDK-778), so its naming form answers
  no definition line; this config writes none for a nested type.
- **Block form.** No Native answer states the value form of a root definition. Native's loader
  rules say a template loader logs `Expected format is "key={"` for any other statement, but that
  is not in the answer, and a custom loader states nothing. A field of a definition shows that a
  block is accepted, not that every definition is one (a script value can be `x = 5`). So the
  line's `value_form` keeps a gap with that reason, except where a root statement answers it.
  SDK-863 asks Native to state it.
- **Scope.** Native states the scope of each block that a definition reads on that block's field,
  which answers the block's own scope claims. No answer states one scope for the whole definition,
  so the line's `push_scope` and `replace_scopes` keep a gap with that reason.

These gaps apply to the types of each directory that Native lists, and only to a claim that no
rule answers without conditions. A type whose selector Native does not identify also keeps its
selector gap. A type whose directory Native does not list keeps no answer and no gap. On the pin
move B snapshot the existence rule credits 164 definition lines.

### Listed fields with no answer (SDK-859)

A claim on a root field that Native lists ends in an answer or in a gap that says what Native
leaves open. One mapping answers such claims:

- **Scope spellings.** The config writes a scope replacement as `## replace_scopes` or, in four
  files, `## replace_scope`; the ledger classifies both as one `scope_context` claim. A field's
  `scope_context` rule answers both spellings under the same conditions, and its gap blocks both.
  A `read_scope` rule alone answers neither: it gives the read-time `this` without the evaluation
  contexts.

Every other such claim that no rule or gap reaches takes a gap after all rules and gaps are
projected, as the definition lines above do. The claims are those of the root type of a listed
directory (`field_existence`, `value_form`, both cardinalities, scope and `file_extensions`) at the
field, its `$item:N` entries or its annotations. A key nested below the field waits on Native's
nested subjects (SDK-833) and takes no gap here; a claim that an answer or gap already reaches
under its conditions takes none either. The first reason that applies is the gap:

1. An item of the field's value list: "Native does not establish the items of this field's value
   list". Native reports a value list as a block whose members are unresolved, or as one value read
   per occurrence; neither states the item form or count (SDK-867).
2. `## file_extensions`: Native does not establish which files the field accepts (SDK-868).
3. `## color_type`: Native does not establish the color form of the field.
4. Under a `subtype[…]` condition, as the SDK-716 decision states: a minimum ("Native answers
   whether every definition needs this field, not whether a subtype's definitions need it"), a
   scope, and any other claim on a field that Native does not read on every path.
5. A scope claim, by how Native reads the field: a modifier block, whose keys a compiler checks by
   the categories it accepts and which publishes no scope; a field whose read Native does not
   establish (a `value_form.unresolved` gap or no value form); or a field that Native reads as one
   value, not a block.
6. Otherwise: "No Native answer about this field reaches this question". No claim takes this at
   pin move B.

Gaps earn no credit. On the pin move B snapshot (Native `2c94b30`) the scope mapping credits five
`replace_scope` claims (three in `dust_clouds`, `crisis_objective`, `planet_class`), and 785
claims that read "No qualified answer" take a gap; 761 of them are gate claims, and the other 24 are
under `gfx`, `interface` and `sound`. The gate claims are 301 list items,
224 subtype minimums, 84 subtype claims of fields not read on every path, 74 subtype scopes,
38 modifier-block scopes, 23 scopes of unresolved reads, 12 file extensions, 3 scopes of values and
2 color forms.

### Keys the engine never reads (decision of 2026-10-20)

A root key of a registry's root type is "not a field" when Native's root-field answer for the
registry (`registry_fields/R` in the snapshot's answers) is `Complete` and every gap it carries is
an `OutsideMethod` boundary, and Native lists no field with that first key. `Complete` means every
reader path, field name and reader classification is resolved, so a key formed at run time, a
dynamic name or an unfollowed inherited or nested reader would have made the answer partial; the
two boundaries every answer carries (defaults, domains, limits, nested grammars and runtime
callers; entry contexts) say nothing about which root keys exist. The key must be literal (not
`scalar`, `int`, `<type>`, `enum[X]`, `alias_name[X]` or a `$` parameter), not a root statement
key, and not the field that names a definition; the registry's `definition_names` must be
established, since a loader can read a key to name its definitions. The loader rule
`keeps_definition_text` (Native `loader_rules`, `loader-rules/v8`) must be `false` with qualified
evidence (decision of 2026-10-20): the item's member reader reads each statement once and reports
and skips a key it does not handle, so no later code reads the key (Native `loader-rules.md`,
"Definition text after the read", with its build-wide `later_text_reads` rule). A missing rule or
a gap credits nothing. Root statements (`events`,
`common/component_templates`) keep their own rules.

The key's claims are covered, and each joins the [CWT disagreement review](cwt-disagreements.md):

- `field_existence` on the key and on any key below it answers `false` (absent), under any
  subtype condition too, since the root reader reads the same keys whatever the subtype.
- The dependent claims (`value_form`, `cardinality_minimum`, `cardinality_maximum`,
  `scope_context`, on the key, its items and its annotations) answer `"not_a_field"`: the engine
  reads no value there, so it has no form, count or scope. A claim that another rule already
  answers or gaps is left alone.

A registry whose answer is `Partial` keeps every unlisted key open: an unresolved path could read
it. The live snapshot at Native `5143009` predates `keeps_definition_text`, so the rule credits
nothing there. Native `8983b8b7` states `Known(false)` for all 19 registries whose
`registry_fields` is `Complete` on M452; with that pin the rule would cover the 17
`common/timeline_events` claims on `type`, `text` and `texture` (the engine reads `types`,
`override_text` and `override_texture`).

### Enum domains (SDK-818)

Native's `Field.domain` lists the names that a keyword field accepts (`Listed`, SDK-828), or is
`Unknown`. A `Listed` domain is the rule `domain`, and the field's value form is
`{"form": "keyword", "names": […]}`; an `Unknown` domain is a `domain` gap, and a keyword reader
without a domain keeps its value-form gap. When the config value of the field that a rule or gap
joins is exactly `enum[X]`, the `domain` rule answers each `enum_member` claim of `enum[X]`, in
every file that lists it: `true` when the field accepts the member's name, `false` otherwise. The
answers have no conditions, since an enum holds wherever it is used. A field of a type whose
selector no Native answer identifies answers no member: the members keep that selector's gap. A
`domain` gap on any field
whose config value names `enum[X]` blocks the credit of every member, and two fields with different
domains are conflicting answers. The comparison report compares a member's answer with `true`:
the config lists it as accepted.

### Required fields (SDK-818)

Native's `Field.requirement` says whether loading a definition without the field logs a report of
its absence (Native's `registry-fields.md`, "Required fields"). `Required` is the rule
`occurrences.minimum = 1` and `Optional` is `0`; `Unknown` keeps the gap. The rule answers the
field's `cardinality_minimum` claim. A requirement gives presence, not a count, so `Required` does
not answer a config minimum above 1: that claim keeps the gap "Native establishes that a
definition must contain this field, not a count above 1". `Optional` answers every minimum.

### Loader rules (SDK-797)

Native's `loader_rules` answer (`loader-rules/v8`) becomes five properties of each registry
subject: `extension` (`{"suffix": …}`, `null` for every file), `recursion`, `duplicate_rule`,
`definition_names` (the forms of root statements) and `keeps_definition_text` (whether the loader
keeps a definition's text after it reads it; no config question joins it). A form that names a nested block becomes the
`definition_names` rule of the field at its path. A property that Native leaves unresolved, or a
partial `extension`, `recursion` or `duplicate_rule`, is a gap on the registry that blocks every
joined claim. A partial `definition_names` answer keeps its
stated forms and publishes a `definition_names.partial` gap that joins no config question; with no
root form it is unresolved. The file order, the mod-over-vanilla rule and whether other code reads a
definition's file again, which Native states once for the build (a live fixture control checks the
file order for a template loader), are the gaps `inventory:loader_rules#file_order`,
`#mod_over_vanilla` and `#later_text_reads`. They join no question, so a claim that needs only those stated facts and the
duplicate rule keeps its credit.

The type lines join by the type's selector:

- `path_extension` and `path_strict` join `extension` and `recursion` for the root type, a nested
  type and a type that a root key selects. `path_strict = yes` agrees with `None`; a `null` suffix
  differs from `.txt`.
- `name_field`, `skip_root_key`, `type_per_file` and `name_from_file` join the one form that names
  the type's definitions: the `Root` form (or the only root form) for the root type, the
  `RootKey(KEY)` form for a type with `type_key_filter = KEY` and no `skip_root_key`, and the
  nested field's form for a nested type. Native's root-key form makes `event_namespace` such a
  type; only its loader lines join it, and its other questions keep the selector gap. `name_field`
  agrees with `Field(name)`, and `"-"` with `Value`. `skip_root_key` agrees with a nested form.
  `type_per_file` and `name_from_file` agree only when they are `no`: Native has no form that a
  file names. A loader that names no definition (a known empty list) answers only `name_field`.
- `path_file = F` of a type in directory `D` joins only the `loader_path` rule of a registry named
  `D/F`. Native lists no such registry: the loaders of `common/alerts.txt` and
  `common/achievements.txt` are accepted special cases (Native's `loader-rules.md`, "Single named
  files"), so these claims stay uncredited. A join by name agrees or does not join; there is no
  differing answer. Other types in the folder do not make the claim's selector unresolved: only
  the registry named by its one file answers it. A folder registry `D` does not establish which files a loader reads.
- An `overrides.cwt` priority joins the `duplicate_rule` of its directory: its presence, and its
  value. `lios` agrees with `LastWins`, `ordered` with `Merges`, and every other pair differs.

The ledger accepts only the published `atlas_rule_snapshot` input. Its private scoring
projection joins rule subjects to ledger questions and preserves exact Native source stamps,
conditions and gaps. There is no second published coverage snapshot contract and no replay-era
registry observation input.

Credit requires a matching question and conditions, a supported answer, and applicable qualified
engine evidence. Recorded sources cannot give current-engine credit. Another target, narrower
conditions, an applicable gap, or incompatible qualified answers prevents full credit. Assessments
retain incomplete states in `answer_states`, including when none earns credit. Gap-only questions
absent from the ledger appear in `atlas_only_questions`. Conflicts are between applicable Atlas
answers, not between Atlas and CWT. Source answer equality is never used. Conditions match exactly;
there is no inferred implication or nearest-target fallback.

## Determinism and checks

File paths, source hashes, claims, diagnostics, and report breakdowns have deterministic ordering.
Coverage uses integer counts; percentages are rounded to six decimal places. Identical input bytes
produce identical report bytes, independent of filesystem enumeration and checkout location.

The game-free tests cover line accounting, ownership/method assignment, stable IDs across answer and
whitespace edits, exact and contradictory answers, unsupported/synthetic/partial evidence, target
and condition mismatch, conflict/gap handling, duplicate evidence, denominator reconciliation,
unknown formats, and relocated CLI inputs. The full-config gate separately pins both input hashes
and both output digests. Baseline changes require inspecting the changed findings before updating
the fixture; a changed digest is not a reason to rebaseline automatically.

Direct snapshot-to-claim comparison is implemented by SDK-524 as a separate test report; SDK-570
added the language name lists, the `script-docs` logs, the define files and the loaded modifier
tags to it. It does not promote Native evidence or publish a rule snapshot.

## Content-derived documentation (SDK-718)

Decision (2026-10-15): the content-derived claims split. Documentation is read from the shipped
content; content queries are consumer policy. Native stays out of content (vision 2026-10-02).

- **Content queries.** The 164 `content_query` claims (a `complex_enum` and its enumeration
  recipe) are consumer policy: an enum filled from content is a listing that the consumer builds,
  not an engine value set. They leave the Atlas-owned denominator.
- **Documentation.** The headline keeps its denominator, engine facts plus content-derived
  claims. Its content-derived claims are the `documentation` claims that
  `classify::documentation_owner` assigns to shipped content: those of `common/` files, defines,
  on_actions and game rules. The other documentation claims keep their classes: `effects.cwt` and
  `triggers.cwt` text is an engine fact, and other prose is authored text outside the headline.

`ledger --game-content INSTALLATION` reads the base-game `common/**/*.txt` of the installation, with
the SDK-525 matcher (the association and comparison rules in [documentation.md](documentation.md)):

- A claim is compared with the comments that document its key in its content directory: the
  `path` directory of its own type (descendants included); for a claim outside every type with a
  path, the directories of every type in its file; else the file's own stem;
  `common/defines` under the define's namespace; `common/on_actions`; and `common/game_rules`.
- An exact match (comment markers removed, whitespace folded) answers the question. The answer
  lists the distinct matched texts, with one content-origin evidence item for each location
  (`shipped-comment:FILE:LINE-END_LINE`, method `shipped-comments/v2`).
- Otherwise a comment that documents the same key and holds the text answers it (decision of
  2026-10-20): it contains the text word for word once both fold case, punctuation and whitespace,
  as where the shipped line adds the example key or "Optional". A comment of another key never
  answers, and neither do the lines of a comment that are a guide heading for another key
  (`other: prose`, a commented `other = …`, and the lines that continue it). The answer lists
  the holding comments' texts. On M452 at Native `5143009` this credits 75 claims.
- A question is answered only when each of its occurrences is answered. Otherwise it has one
  gap: "The shipped comment at FILE:LINE-END_LINE is a rewritten candidate, not the same text" for
  a rewritten candidate, or "No shipped comment in the installation has this text".
- Documentation that no shipped comment holds is authored text, not an Atlas-owned claim
  (decision of 2026-10-20, as for SDK-718): CWT wrote it. `ledger --game-content` makes such a
  claim `authored_text` before it scores. A comment holds the text when it contains it, word for
  word, once both fold case, punctuation and whitespace, so a comment that CWT shortened or
  recapitalized keeps the claim content-derived, and that comment answers it (above); every
  comment that documents the key in the claim's directories is checked, not only the similar
  ones. A claim keeps its owner when
  the installation was not read completely for it: a directory outside `common/` (the corpus), or
  a directory with a shipped file that has a parse problem, since a file that fails to parse keeps
  only its guide headings. So the denominator of a run with `--game-content` depends on the
  installation; a run without it keeps every documentation claim content-derived. On M452 at
  Native `5143009` this moves 497 claims (392 in `common/` and `events/`); 234 stay open, most in
  the folders of the README and example files that fail to parse (`scripted_actions`,
  `traits`, `missions`, `starbase_*`, `pop_faction_types`, `war_goals`). Reading those files'
  comments without a parse tree would decide them.

The content-origin rule: content evidence credits only content-derived claims, as the origin table
of `coverage::suitable` already requires (an engine fact needs engine evidence). Its target is the
installation's Native build (Native hashes the executable; no game starts), so it credits only a
rule snapshot of that build; `evaluate` refuses content of another build and content without a
snapshot. A snapshot whose sources are all recorded makes the content evidence unqualified: a
recording measures no current capture. `coverage.json` then has a `content` section with the
build, a digest of the sorted relative paths and file hashes, the file count, the parse problems,
the number of answered questions and each gap. A file that the parser repairs keeps the comments of
the repaired tree, as in SDK-525; a file that fails to parse keeps only its guide headings
(`key: prose`). A missed comment withholds credit; an answer still needs an exact or a held text. As with
config diagnostics, the CLI exits 2 after writing the reports when the shipped files have parse
problems.

At pin move B with SDK-860 (live snapshot `.local/sdk-818/rules.json`, M452 build `c621723d…`,
config `8574760`, 2,063 shipped files, 35 parse problems):

| Measure | Before | After |
| --- | ---: | ---: |
| Atlas-owned headline | 27,807 / 57,807 (48.103%) | 29,951 / 57,643 (51.959%) |
| Content-derived | 0 / 3,118 | 2,144 / 2,954 |
| Milestone 5 gate types | 6,799 | 7,206 |

The 2,144 claims are 1,285 of 1,329 defines, 269 of 342 on_actions, 183 of 215 game rules and 407
of 1,068 type-schema entries. Of the 794 questions without credit, 129 have a rewritten candidate
and 665 have no matching comment. Engine-fact credit is unchanged.

## Shared-reader answers

Field subjects retain paired Native `read` alternatives, summary `shape`, `conditions`,
`block_family`, `members`, and separate `uses`. Each read keeps its condition with its outcome;
Unknown and Unresolved values remain explicit. The `uses` stage is `stored_value_selection`,
not parser acceptance. Empty selections do not establish unconditional use. Domain Unknown values are published as gaps. An unconditional read publishes
`repeat_behavior` as `Replace`, `Accumulate` or `Merges`; Atlas publishes no engine occurrence
maximum. For config comparison, `Replace` corresponds to a maximum of 1, and `Accumulate` and
`Merges` to `inf`.
These facts answer maximum questions even when the CWT assertion differs; coverage and agreement
remain separate. Unknown or conditional repeat behavior earns no unconditional credit. A minimum
stays unresolved unless validation reports the field's absence; finite accepted fixtures do not
establish that a field is required or optional.

Conditional value forms keep all alternatives in one answer. A separate `value_form.unresolved`
gap blocks unconditional credit, including nested children of conditional parents. Comparison
retains the structured answer and gap. A CWT subtype name is not inferred from a Native predicate.
One exception, decided in SDK-800: a rule condition `root_key[KEY]` (the answer holds for
definitions whose root key is `KEY`) credits the subtype that `## type_key_filter = KEY` selects,
when that subtype's definition body is empty and it is the only such subtype of the registry's
type. The loader dispatches on the root key, so the key is an engine fact, and the config itself
writes the key-to-subtype mapping; Atlas infers nothing. A different key, a `<>` or list filter, a
subtype that also tests a field (`planet_killer`, `asset_selectors.cwt`), and a claim with any
other subtype condition earn no credit through this rule; the always-read rule below covers some
of those forms. At config
`8574760`, 32 of 35 `type_key_filter` subtypes qualify, and 636 Atlas-owned claims carry only such
conditions (440 in `components.cwt`, 143 in `sprites.cwt`, 53 in `events.cwt`). The 53 in
`events.cwt` and the `components.cwt` claims join through the root statements above; the
others earn credit only once Native answers per root key.

A second rule, decided in SDK-716, reaches other subtype conditions. When every read alternative
of a field has the condition `Always` (its `conditions` rule lists only `Always`), Native reads
the field the same way on every path. Its unconditioned `existence`, `value_form` and
`repeat_behavior` then answer the same `field_existence`, `value_form` and
`cardinality_maximum` questions under any `subtype[…]` condition, negated and nested ones
included, and its gaps on those properties block them. The limits:

- Scope claims (`push_scope`, `replace_scopes`) under a subtype earn nothing, since a subtype can
  change its scope.
- A `cardinality_minimum` claim under a subtype takes a requirement (decision of 2026-10-20):
  `occurrences.minimum = 1` comes from an unconditional check that logs the field's absence, so it
  holds in every definition and answers 1 under every subtype. An optional field (0) earns
  nothing, since a subtype can require it; it gets 0 only after a fixture probe per subtype key
  shows that a definition without the field loads with no diagnostic. A minimum gap blocks the
  subtype claims. On M452 at Native `5143009` no subtype minimum has a required field, so this
  credits none; 224 claims of optional fields wait for that probe, 64 carry a minimum gap
  (requirement `Unknown`) and 260 have no Native field.
- A field with any other alternative (`FieldZero`, `Unresolved`, `All`) earns nothing, and so
  does a field that Native marks conditional, such as the child of a conditional parent. Native's
  only field condition is `FieldZero`, and matching it to bool subtype tests is deferred.
- The `Always` premise must itself rest on qualified evidence; a recorded premise earns nothing.
- A claim under a subtype with any `## type_key_filter` keeps the SDK-800 rule and takes nothing
  from this one. Event kinds and component templates join as root statements above; a field
  that every root statement reads alike takes this rule as stated there.
- A claim whose field Native does not list earns this credit once field discovery lists it.
- A type selector gap still blocks the claims of its type.

The tests that define a subtype in `types` (every key in `types/type[x]/subtype[y]/…`, such as
`colonizable = yes`) and the `## cardinality` bounds written on them are consumer policy, like
subtype naming: they are CWT's partition, not engine rules. The subtype's own `## push_scope` and
the naming lines under `localisation` or `images` subtypes stay engine facts. At pin move B
(Native `2c94b30`), the rule credits 546 claims, all under `common/` types (`trait` 262), and the
policy class removes 225 test claims from the Atlas-owned denominator.
Duplicate subject and record identities remain invalid. A missing reader ID does not erase an
established broad value form; an opaque reader ID alone does not establish one.

A block field's `nested_grammar` gap remains only when Native establishes neither its command
family (`Trigger`, `Effect`) nor its members (`Fields`, `ModifierBlock`, `WeightBlock`,
`TriggeredModifier`); the `block_family` and `members` rules carry what is established, including
any partial member properties. Weight claims are credited at field level: the shared
`alias[modifier_rule:…]` grammar in `modifier_rule.cwt` has no Native subject, because Native
answers one weight block for each field. A block's `read_scope` (the read-time `this`) answers
`push_scope` claims. Its `scope_context` answers `replace_scopes` only when the read-time `this`
and at least one evaluation context are established, Native reports no context gap on the field,
and every slot resolves; otherwise the gap says which part is established. Modifier blocks
publish `accepted_categories` instead of a scope context. `Float` readers have the `number` form,
and scoped-numeric readers the `scoped_number` form. A string-like field's `reference` gap closes
when every lookup target is a registry or the trigger commands, or a derived name uses the field's
text.

Fixture conclusions remain parser outcomes of Atlas's two fixture games, separate from static
answers. A question with no observation is a gap, never a rule. The parser and diagnostic checks
of the Native methods (SDK-542, SDK-544, SDK-545, SDK-549, SDK-550) live in Native's tests and earn
no Atlas credit.

Scope and localization context display names are explicit `display_name` rules. Subject IDs
remain opaque to consumers; an empty display name is retained but cannot name a comparison row.

Published gap owners are capability categories, with these ticket mappings maintained here:

| Category | Current owner |
| --- | --- |
| `argument_grammar` | SDK-548 |
| `scope_context` | SDK-549 |
| `modifier_application` | SDK-547 |
| `references` | SDK-543 |
| `callback_context` | SDK-608 (done); SDK-712 owns the remaining contexts |
| `field_semantics` | SDK-627 |
| `field_conditions` | SDK-541 inheritance relation; runtime selection is out of scope |

A null owner stays `null` in the snapshot. The [unowned gap triage](gap-triage.md) gives each such
gap a failure shape and its ticket or out-of-scope note. Owner categories do not change
the reason or evidence of a gap. Failed questions use deliberate stable text, retaining error
details and disposal uncertainty without depending on Native Debug formatting.

Version-2 readers also accept historical `SDK-<number>` owner strings; new snapshots emit only
categories. Native names a nested field gap by its full path, as a key path or a dotted field
name, and a root field gap by its key. Atlas attaches a full path to the field at that path, or,
for a path below a field's own members (a weight block's keys), to that field. A single key
attaches to the root field of that name; when there is none, a key that only one nested path ends
with attaches to that field. Other names stay in a registry `field_gap_subjects` gap with the
original Native gaps.

## Compiler scope and gap triage (2026-10-02)

Atlas publishes static facts that change what a compiler accepts, rejects, types or completes.
Fixture conclusions describe parsing, storage and validation. Evaluated values, runtime weights,
operand selection and duration consumption are out of scope. Numeric claims use established
conversion properties; removed clamp and default placeholders supply no claims. Scope claims
state declared entry scopes. SDK-608 established callback entry contexts; SDK-549 established field
`this`, and SDK-677 field `root`, `from` and `prev`. SDK-712 owns the contexts that they left unresolved.

For SDK-626, give a failure shape a ticket only when closing it changes one of those compiler
behaviors and a config claim depends on it. Record the dependent claim and compiler consequence.
Other shapes receive the note “out of scope, vision 2026-10-02”. The [triage](gap-triage.md) records
the result. Measure failure shapes from fresh
M451-hotfix recordings after the Native pin move. A count or broad capability owner alone does
not justify new discovery work. A compiler, rather than an Atlas policy overlay, selects severity;
subtype names and alias factoring remain emitter test mappings.

## Callback entry scopes (SDK-704)

Atlas applies Native's hand-checked self-link assumption to `entry_scopes`: self-linked `root`
has the type of `this`; the first self-linked `from` or `prev` is `not_set`; a later self-link
has the type of the preceding scope. `this` must establish its own type. Each call site's
alternative retains `this`, `root`, `from` and `prev`. Arrays describe the positions Native
reported, not a maximum chain length: a terminal self-link can continue beyond that position.
`prev` can pass through `not_set`. Unknown scope IDs, unresolved slots and missing call sites
remain gaps. A resolved alternative does not cover another site's Native gap.

[The independent-check ledger](entry-scope-checks.json) records the comparison for
`callbacks/v5` on M452: 238 of 294 on_actions agree with the vanilla scope comments, and 201 of
223 game rules agree with the config `replace_scopes`. It names all 18 confirmed disagreements
and all 35 names with no independent source, with stable reasons. The sources are SDK-608's
hand checks (carried by SDK-704 and SDK-625), SDK-799, SDK-818 and Native's
`docs/native/engine-commands.md`, “On_actions, game rules and their entry scopes”. The self-link
rule remains an assumption. These checks apply only to the exact recorded build and
`callbacks/v5`; comments and config never feed Native's extraction. Carry them to a later build or
method only after the same check; otherwise the gate drops them, and the names without an
independent source lose their gap.

A name agrees when every context that Native gives matches each slot that its source names.
A slot that the source does not name is not compared; config `any` matches every value, and
config `no_scope` matches `NotSet`. A vanilla comment is an independent source only when it
names the type of `this` (or of `root`, which some comments write for `this`); a comment with only
role words there, such as `Actor` or `Main Ally`, is not. Each comment slot that names a scope
type, an owner (a country) or an heir (a leader) is then compared. With this rule, the pinned
config and the `callbacks/v2` answers, the game-rule count is 181, as SDK-608 recorded. The
on_action count keeps SDK-608's result for the 232 names whose answer did not change and applies
this rule to the 62 that changed.

SDK-799 changed these results from `callbacks/v2`:

- 62 on_actions changed answer (SDK-712, SDK-729, SDK-730, SDK-731). Each had an unresolved scope
  or no context in `callbacks/v2`, so none agreed before. 50 agree now. Five keep no independent
  source (`on_add_to_council`, `on_colony_yearly_pulse`, `on_proxy_war_started`, `on_status_quo`
  and `on_status_quo_forced`), and `on_war_participant_leaves_early` joins them: its comment, as
  for `on_status_quo`, names only roles and a war. The four debris names join the names whose
  comment describes a branch past the path limit. Two are new confirmed disagreements:
  `on_rebels_take_colony_owner_switched` (`CColony::DailyUpdateSerialHandleCombat` builds only
  the country and the colony) and `special_project_success` (`CSpecialProjectInstance::OnSuccess`
  links the ship only when `FindShipOfCountryForProject` finds one).
- 7 game rules changed answer. The five `can_fill_*_job` rules and `system_blocks_sensors` agree;
  `can_orbitable_repair_ships` keeps an unresolved context and has no `replace_scopes`.
- 15 of the 17 game rules that SDK-608 recorded without `replace_scopes` have one in the pinned
  config. 14 agree. `is_seeded` is a new confirmed disagreement: `CGameRules::IsSeeded` builds a
  fresh scope and sets only the fleet, so script sees no `from`, and vanilla does not read it.

SDK-818 carried the checks from `callbacks/v4` to `callbacks/v5` (SDK-812; SDK-823's path joins).
No game rule answer changed. 16 on_action answers changed, and only gained contexts:

- Five that had no context now agree: `on_building_ruined`, `on_colony_attackers_win`,
  `on_colony_defenders_lose`, `on_entering_war` (its `From` comment names a role) and
  `on_pop_purged`.
- `on_colony_destroyed_by_bombardment`, `on_colony_zero_pops` and
  `on_colony_zero_pops_ground_combat` gain a context with a fleet as fromfrom, which their comments
  do not name; they still agree. `on_resolution_passed` gains a context and keeps no independent
  source.
- `on_monthly_pulse_country` gains an unresolved context, so it no longer agrees.
- The four debris names and `on_ship_quantum_catapult` now reach the call site that their comment
  describes, and Native follows another site that the comment does not describe (no science ship;
  no second system). They move to a group of their own with no independent source.
- `on_gaining_station_control` gains its first context, a ship from a planet. Its comment's
  `station` names no scope type, so it has no independent source.

The confirmed disagreements remain engine answers: ten game rules have no `from` despite the
config; two pass `colony` where config says `planet`; `dismiss_leader_cost` evaluates on a leader;
and five on_action comments name the wrong object or an object that the engine does not always
set. These findings create no Native gaps. Unrelated Native failures still block coverage.

The 35 names with no independent source receive an `entry_scopes` gap, including the six whose
comment describes a site or branch that Native could not follow. Native's original gaps remain
attached. The ledger's coverage projection therefore keeps these reasons and grants no
entry-scope credit.

The comparison report compares a `push_scope` on a subtype of a type (an event kind's scope) with
the root statement's `read_scope`. The config names a scope by a `scopes.cwt` alias, which joins
one Native scope type, or a scope group that stands for its members (`carrier` is planet or ship).
The two agree when they name the same set of scope types; `any` agrees with every answer,
including no scope. A read scope of `enclosing`, or an alias that joins no scope, is not
comparable. Field-level `push_scope` claims stay unmapped (SDK-717).

Comparison report format 3 adds `entry_scope_checks`. This field publishes the historical results
only when both the on_action and game-rule answers have the matching build and callback method.
It is `null` if either answer is absent or uses another build or method. It does
not compare today's config and does not alter current comparison statuses. The ordinary
scope-context comparison remains unmapped; it retains the structured engine answer or gap.
Recorded answers still earn no current-engine coverage credit.
