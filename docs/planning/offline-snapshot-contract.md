# Atlas offline snapshot and consumer contract

Status: accepted by Jackson on 2026-09-16 after final shared-understanding confirmation. The resolution comment on the linked decision is canonical; this document is its supporting contract artifact. No production implementation or new game-rule qualification is claimed.

Decision: [Define the offline snapshot and consumer contract](https://linear.app/unnamed-system/issue/SDK-477/define-the-offline-snapshot-and-consumer-contract).

## Scope and ownership

Atlas publishes static, evidence-supported game rules, documentation, and explicit gaps. Consumers own authored content, dependency loading, individual reference resolution, checks, diagnostic severity, and authoring recommendations. They receive no config fallback or vanilla/mod content catalogue.

The current SDK is the first direct consumer; the planned language compiler is another. The planned LSP uses the compiler and its bundled snapshot as its source of truth. Implementing the compiler, LSP, IDE selection mechanism, or SDK adoption is outside this decision.

Atlas's rule model stays platform-independent. Exact native targets qualify evidence and applicability; they do not select platform-specific Atlas implementations. PDX Native retains its already agreed native qualification boundary.

## Distribution, selection, and identity

A snapshot is a complete immutable publication of its declared coverage, not a claim of whole-game completeness. It contains the rule and schema reference closure needed for offline consumption. Corrections produce a new snapshot version, including when the supported game version stays the same.

SDK/compiler releases bundle compatible snapshots. Selecting a tool version pins its snapshot inventory; the project's game target selects among explicitly supported targets. Authors need not manage Atlas versions directly. There is no automatic online update during a build and no silent selection of the nearest unsupported game version.

Keep these identities separate:

| Identity | Meaning |
| --- | --- |
| Snapshot version | An immutable Atlas publication; a correction gets a new version |
| Artifact digest | SHA-256 of the deterministic JSON payload bytes |
| Contract version | The syntax and semantics a consumer must understand |
| Game applicability | Explicitly supported game releases/builds and conditions |
| Rule identity | Stable subject and property across snapshot revisions |
| Evidence identity | Shared material, methods, and exact claim-support locations |

Human-readable game labels are accompanied by qualified target references in evidence/applicability records. Matching a displayed label alone does not qualify a changed executable. Consumers may work offline without a local game installation; their declared target does not certify an installed executable.

A rule identity does not encode its current answer, evidence hash, array position, native address, or snapshot version. Correcting the answer keeps the identity for the same subject and property. Different properties have different identities. Diagnostic traceability uses both snapshot and rule identities.

## JSON and JSON Schema

JSON is the only selected serialization. Agent-facing reading/retrieval tools and YAML exports are deferred.

Use standard JSON Schema for logical value shapes and alternatives, including unions, primitive types, supported bounds, arrays, objects, and shared definitions. `$defs` and `$ref` provide reuse, including recursive shapes where required. All schema dependencies are bundled and resolved locally; a URI is an identity, not permission to fetch a network resource.

The selected baseline is **JSON Schema Draft 2020-12**. The Atlas contract fixes the schema dialect and supported vocabulary. Do not depend on validator-specific extensions, implicit coercion, default insertion, remote loading, or silently ignored assertion keywords. Schema validation alone does not apply all Atlas rules or establish evidence correctness.

Use standard schema composition for variants before adding Atlas-specific semantics. `$ref` does not itself provide arbitrary parameter substitution. Producers can emit shared concrete variants; a genuinely required parameterized operation must have defined contract semantics before publication. No general expression language or executable callbacks are introduced by this decision.

Schema references and game-content references are different: the first reuse shapes; the second describe categories and lookup relationships for consumers to resolve against project inputs.

## Snapshot records

The following record families define the logical contract. Wire names in examples are illustrative; renaming a field before implementing the initial contract does not change the agreed semantics.

| Record family | Required meaning |
| --- | --- |
| Manifest | Snapshot/contract identity, schema dialect, game applicability, producer/generator identity, coverage declarations |
| Definitions and fields | Stable subjects, supported forms, value-schema links, rule links, and documentation links |
| Schemas | Standard structural schemas and their bundled reference closure |
| Rules | Stable identity, exact subject/property, applicability/conditions, supported answer, and precise evidence links |
| Reference categories | Consumer lookup category and established lookup relationships; no content identifiers catalogue |
| Scope contracts | Structured scope types and availability under stated conditions; identity relationships remain documentation |
| Coverage and gaps | Declared workflow/form/property boundaries, obligations, established knowledge, unresolved questions, and exclusions |
| Documentation | Evidence-backed explanations, including runtime processing, recovery, and scope identity |
| Evidence summaries | Shared method/target/artifact records, limits, and exact supporting locations per claim |
| Corrections | Affected historical snapshot/rule identities, reason, withdrawal extent, and replacement or resulting gap |

Rules retain the distinction between parser/storage, validation, and runtime properties. Runtime outcomes are documented rather than converted into a runtime simulator. A reference category, scope type, shape, default, bound, and occurrence limit are separately assessable properties even when they share evidence.

For occurrences, represent a supported minimum/maximum separately from the value shape. Distinguish an established unbounded maximum from an unknown maximum. Do not encode unknown as zero, infinity, an empty schema, or an omitted constraint with permissive meaning.

For defaults, record evidence-supported omission behavior separately. Structural checks leave omitted fields absent. A condition may use an established default only when its rule specifies that interpretation. JSON Schema's `default` annotation does not authorize filling in source fields.

For conditions expressible structurally, link to standard schemas for the relevant logical input. Conditions on occurrence limits or scope availability retain the affected property and exact conditions. A relation requiring values from another authored definition identifies the consumer input and reference relationship. Publish only operations with defined interpretation; add new operations through contract evolution when concrete supported properties require them. Missing inputs never justify guessing at an answer.

## Consumer mapping and checks

Atlas does not prescribe a common parser, compiler, or syntax-tree format. Consumers may use their existing representations and lower the rules into generated types, compiler data, or validation checks.

A field value schema describes each occurrence. Occurrence rules describe how many entries may appear. Repeated scalar entries and a single list-valued field are different structures. Consumers retain original entries, operators, meaningful order, mixed named/bare items, and numeric precision. They cannot drop duplicates, select a convenient last value, or round a number merely to produce a JSON Schema validation result.

Logical object/union projections are allowed where faithful. If a singular logical property has multiple authored entries, handle the occurrence issue before relying on a singular projection; do not silently collapse the input. Order-sensitive and mixed forms remain available to the consumer's Atlas checks even when a structural schema cannot express their entire meaning.

The current SDK parser already retains ordered mixed containers, duplicate keys, operators, and numeric lexemes. This contract preserves those guarantees without adopting its TypeScript implementation as a required external format.

Applying a conditional rule distinguishes:

- The condition is established true: apply its answer.
- The condition is established false: this rule is inapplicable; it says nothing about the alternative.
- The consumer lacks required input or cannot determine the condition: report that limitation for the affected check.

A known absent authored field can make a presence condition false. It is not the same as unavailable input. Runtime triggers are not evaluated by these structural conditions.

Consumers may assign stricter authoring policy, but cannot attribute it to Atlas as an engine fact. A consumer that cannot represent or apply a supported form reports its limitation instead of claiming the game rejects it. The first supported SDK path still stops with a precise diagnostic for unsupported forms or gaps preventing its promised checks.

## Lookup and partial coverage

The JSON records support lookup by definition/subject, rule identity, schema identity, coverage obligation, and evidence identity. They expose reference relationships directly; no particular reader library or agent tool is required.

Keep knowledge and support separate. A feature can have known facts while remaining outside consumer support. Unknown, unsupported, contradicted, and untested retain their governing meanings; they are not one exclusive status enum.

| Lookup situation | Consumer-visible meaning |
| --- | --- |
| Supported property answer | Exact answer, conditions, evidence, and applicable coverage |
| Explicit gap | Property is unresolved, with scope and reason; no fabricated answer |
| Outside declared support | Coverage exclusion, possibly with separately available known facts |
| No matching subject/record | Not found; no inference of permission or prohibition |
| Established unconstrained property | A positive evidence-supported answer, distinct from absence |

A missing internal reference is an invalid snapshot, not a game-knowledge gap. Consumer project references that cannot be resolved are consumer-input outcomes, not missing Atlas schema definitions.

An SDK/compiler diagnostic derived from Atlas can link to the snapshot identity, exact rule identities, conditions, evidence summaries, and related documentation. Consumer policy supplies severity and message wording. Generated types/indexes remain derived data, with traceability to canonical records; they cannot become a competing rule authority.

## Evidence included and retained

Bundle enough to understand each claim offline: exact claim and conditions, evidence kind, method identity/version, qualified target references, relevant content fingerprints, concise rationale/limits, validation performed and gaps, and precise supporting locations in shared evidence. Share common run and artifact metadata instead of duplicating it per rule.

Retain bulky native material, run traces, fixtures, full analysis reports, and reproduction material on the producer side. Compact artifact references carry hashes and locators plus a location within the artifact where needed. The hash identifies content; it does not promise that the artifact is available offline.

Missing access to a remote/full artifact does not erase the bundled claim or masquerade as an engine uncertainty. The consumer reports that full evidence is unavailable while preserving its included summary and reference. Contradictory or invalid evidence is handled through gaps and correction records, not confused with a retrieval failure.

The physical artifact store, retention operations, package locations, and release owners remain with [Decide Atlas repository, packaging, and release ownership](https://linear.app/unnamed-system/issue/SDK-479/decide-atlas-repository-packaging-and-release-ownership).

## Compatibility, reproducibility, and corrections

A consumer checks the Atlas contract, schema dialect/vocabulary, required Atlas record semantics, and declared game applicability before claiming support. Reject unknown required semantics. Non-semantic documentation additions may remain compatible; a newer snapshot number alone neither implies a new contract nor proves compatibility.

Contract evolution must not rely on JSON Schema validators ignoring unknown keywords. New Atlas semantics require an explicitly compatible consumer. Nothing silently falls back to config, an older answer, or a nearby game target.

For the same fixed qualified inputs and generator version, emit identical UTF-8 JSON bytes. The serialization profile fixes key ordering, whitespace, escaping, and number representation. Sort unordered record collections by stable identity; preserve semantically significant array order. Keep changing publication metadata outside the hashed content. Evidence-run timestamps/identities supplied as fixed inputs remain provenance, so a genuinely different evidence run can yield a different artifact.

Hash the payload bytes with SHA-256 and carry that digest outside the payload it covers, avoiding a self-referential hash. Exact numeric constraints must not be rounded during serialization or consumer reading; a required value outside a tool's faithful numeric representation requires explicit support or a reported limitation.

A correction ships as a newer snapshot for the relevant game version. Include the affected prior snapshot/rule identities, reason, and replacement where established. If an answer is disproved without a replacement, remove it from active answers and publish the gap. Mark the affected claim or whole historical snapshot withdrawn as appropriate. Whole-snapshot withdrawal applies when promised coverage or evidence is broadly undermined.

Historical artifacts remain unchanged. Ordinary game updates supersede snapshots without invalidating their original supported claims. Correction records arrive with new publications/tools; an offline tool cannot know about notices it has never received. The normal correction path is a tool update carrying the corrected snapshot.

## Verification and handoff

Producer checks cover unique stable identities, valid JSON/schema syntax, complete internal reference closure, required evidence links, consistent conditions, correction references, and explicit coverage/gaps. Schema validity is not proof of a game claim. Conflicting applicable conclusions need corrected/narrowed evidence or a gap, never implicit load-order precedence.

The illustrative fixture at `fixtures/offline-snapshot-contract.example.json` is fictional and contains no qualified game facts. It checks standard schema reuse and a discriminated union, plus the separation between value shape and occurrence rules. It is not a production envelope schema or a prototype implementation.

Fixture verification on 2026-09-16: both schemas passed Draft 2020-12 schema validation; eight valid/invalid union cases and three occurrence/value cases passed. Union checks left their input objects unchanged. These checks establish only the fictional example's internal consistency.

The existing [Validate the tradition evidence-to-SDK interface with a bounded prototype](https://linear.app/unnamed-system/issue/SDK-484/validate-the-tradition-evidence-to-sdk-interface-with-a-bounded) remains the empirical check: real evidence to snapshot to SDK behavior, including an invalid case and a conditional/shared case. It must expose loss of repetition, order, reference conditions, precision, evidence, or gaps rather than hide it. This decision does not claim that prototype or full tradition coverage has passed.

No additional research prerequisite or new decision ticket was identified. Agent-facing reading remains deferred; production implementation, rollout, and planned compiler/LSP delivery remain outside this map. This planning decision is resolved; the existing prototype remains the empirical follow-up.

## References

- Governing decisions: [authority](https://linear.app/unnamed-system/issue/SDK-471/define-atlas-knowledge-authority-and-authoring-policy-boundaries), [first coverage](https://linear.app/unnamed-system/issue/SDK-472/choose-the-first-useful-atlas-coverage-and-consumer-guarantees), [evidence](https://linear.app/unnamed-system/issue/SDK-473/define-evidence-uncertainty-and-conflict-handling-for-atlas-rules), [conditional rules and scope](https://linear.app/unnamed-system/issue/SDK-474/define-conditional-rules-and-scope-context-contracts), [architecture](https://linear.app/unnamed-system/issue/SDK-475/choose-the-extraction-architecture-and-native-runner-boundary), and [support policy](https://linear.app/unnamed-system/issue/SDK-476/choose-supported-builds-and-the-update-maintenance-policy).
- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12), [schema composition](https://json-schema.org/understanding-json-schema/reference/combining), [shared definitions and references](https://json-schema.org/understanding-json-schema/structuring), and [annotations/defaults](https://json-schema.org/understanding-json-schema/reference/annotations).
- Local consumer evidence: `/Users/jackson/Developer/pdx-sdk/packages/pdxscript/src/ast.ts` and `/Users/jackson/Developer/pdx-sdk/packages/pdxscript/GRAMMAR.md`.
- The glossary is `CONTEXT.md`. The detailed final resolution belongs in the decision ticket. This accepted contract and its fictional fixture are retained locally and attached to that ticket.
