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
and source-line links and closing braces do not create extra claims. CWT grouping, subtype naming,
and alias factoring are explicit consumer-policy claims. A complete negative answer can cover a
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
| Content-derived | Dynamic content queries and shipped-comment documentation | Content observations |
| Consumer policy | Severity, soft bounds, subtype naming, aliases, scope groups | Consumer modeling choices |
| Authored text | Unattributed documentation outside expected engine/content families | Its author |

Documentation assignments are **provisional**. Effects/triggers route to expected engine text;
common schemas, defines, on_actions and game rules route to expected shipped comments. Other prose
has a provisional authored-text assignment. SDK-525 must establish actual provenance, including the
handwritten remainder. None of these assignments supplies coverage or asserts a verified origin.
Unknown metadata has a provisional policy assignment plus a diagnostic; it is not declared
inherently manual.

Classification and roadmap routes are centralized in `src/ledger/classify.rs`. Registry discovery,
field discovery, and reader binding route to SDK-528/530/531. Language declarations route to
SDK-535–540. Shapes, nested grammar, references, numerics, weights, names, modifiers, argument grammar,
and scope context route to SDK-541–550. Loader selection routes to SDK-552; the separate formats route
to SDK-554–556. These are expected methods, not qualifications or hardcoded game-rule answers.

## Snapshot input

The tool reads the retained Atlas caller's live JSON (`queries`) or replay JSON (`startup` and
`final_snapshots`). It retains observed item names/identities and reports registry counts, activation/completion, limits
and gaps.
Item enumeration establishes no config-rule answers, even when a registry name matches a CWT type.

Future qualified answers use this deliberately bounded scoreboard input, not the full publication
contract from the planning documents:

```json
{
  "kind": "atlas_coverage",
  "format_version": 1,
  "snapshot_id": "immutable-snapshot-id",
  "target": "exact-target-identity",
  "answers": [
    {
      "question": "question:<ID from the ledger>",
      "conditions": [],
      "value": { "accepted_forms": ["integer"] },
      "status": "supported",
      "evidence": [
        {
          "id": "durable-evidence-id",
          "method": "qualified-method-revision",
          "target": "exact-target-identity",
          "qualified": true,
          "origin": "engine"
        }
      ]
    }
  ],
  "gaps": []
}
```

This example is a shape illustration, not qualified evidence. The producer is responsible for
truthful evidence/qualification records. The scoreboard validates their required fields and applies
the contract; it does not replay experiments or independently qualify methods.

Credit requires `supported`, matching question and conditions, a non-null answer, and applicable
qualified evidence. Engine questions require engine evidence, content questions require content
observations, and policy/authored questions require authored authority. `synthetic` evidence never
grants coverage. An empty evidence list, another target, narrower conditions, an applicable gap,
`conflicted`, or incompatible qualified answers prevents full credit. The other states are
`partial`, `untested`, `unknown`, and `unsupported`; these remain distinct input states.
Assessments retain applicable states in `answer_states`, including when none earns complete credit.

An explicit gap has `question`, `conditions`, and a nonempty `reason`. Gap-only questions absent
from the ledger also appear in `atlas_only_questions`. Conflicts are between applicable Atlas
answers, not between Atlas and CWT. Source answer equality is never used. Conditions currently match
exactly; there is no inferred implication or nearest-target fallback. Snapshot answers are indexed
by question so scoring does not scan every answer for every source claim.

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

CWT emission/comparison is SDK-524. Documentation source matching is SDK-525. Neither is implemented
by this tool, and none of its outputs promotes Native evidence or publishes a rule snapshot.
