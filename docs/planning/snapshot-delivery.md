# Snapshot delivery decisions

This page retains the parts of the 2026-09-16 offline contract and repository packaging decisions
that still govern Atlas. The original decisions are in Git history and Linear SDK-477 and SDK-479.
The implemented payload is defined by `docs/contract/rule-snapshot-v2.schema.json` and checked by
`src/snapshot.rs`.

- Atlas publishes immutable, versioned JSON for exact supported game targets. A complete
  publication of declared coverage does not claim complete knowledge of the game. Corrections
  produce a new snapshot version, including when the game version stays the same.
- Atlas owns extraction, rule conclusions, coverage and snapshot publication. Native owns
  platform and exact-build methods. Consumers own project inputs, reference resolution,
  diagnostics and authoring policy. Consumers do not use config as a fallback for missing Atlas
  rules.
- Snapshots bundle the rules, schemas, source stamps and gaps required for offline use. Standard
  JSON Schema Draft 2020-12 describes logical value shapes; Atlas records carry properties that
  schema alone does not express. Local references must resolve without network access.
- A field's value schema describes one occurrence. Occurrence limits are a separate property, and
  an unknown maximum is distinct from an established unbounded one. An unknown limit is never
  encoded as zero, infinity, an empty schema or an omitted constraint. Repeated scalar entries and
  one list-valued entry are different structures.
- Rule identity is stable for a subject and property across revisions. Snapshot version,
  contract version, game applicability and payload digest are separate identities. The digest
  covers deterministic JSON bytes and is stored beside the payload.
- Consumers bundle a compatible snapshot with their own release and select only explicitly
  supported targets. No build-time fetch or nearest-version fallback is permitted.
- The source repositories are separate. Native's exact source revision is pinned by Atlas.
  Atlas source releases, Native source releases and consumer releases have independent lifecycles.
- Publication requires a reviewed candidate with changed answers, applicability, gaps,
  corrections and check results. Jackson approves the reviewed bytes. The publication job
  verifies their digest instead of rerunning extraction during publication.
- Private historical prototype bundles remain in their current protected storage. They are
  not part of the consumer payload. Relevant unported findings remain indexed in Native's
  engine knowledge page.
