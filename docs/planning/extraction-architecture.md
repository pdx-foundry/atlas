# Atlas extraction architecture

Status: accepted by Jackson on 2026-09-16. The resolution comment on the linked decision is canonical. This local design record does not claim completed implementation or native qualification.

Decision: [Choose the extraction architecture and native-runner boundary](https://linear.app/unnamed-system/issue/SDK-475/choose-the-extraction-architecture-and-native-runner-boundary).

Naming clarification: Jackson named the shared engine-integration module **PDX Native** during [Choose supported builds and the update-maintenance policy](https://linear.app/unnamed-system/issue/SDK-476/choose-supported-builds-and-the-update-maintenance-policy). This names the existing boundary; it does not move platform or game-version operations into Atlas. Atlas uses one platform-independent rule model. Platform-specific rule variants require an established semantic difference and a later design decision.

## Agreed direction

- Routine extraction is a repeatable pipeline. Agents repair or extend extraction methods when gaps appear; routine operation does not depend on agents interpreting every rule again.
- Atlas is the first consumer of PDX Native. The end-to-end testing framework is paused and is not a prerequisite. Its experiments and design remain useful inputs.
- PDX Native hides OS and executable-version differences. Atlas requests engine operations and observations through its interface; it supplies no native addresses, layouts, function signatures, or platform/build selection logic. Exact target identity remains attached to evidence.
- The TypeScript SDK remains the first consumer of Atlas rules and the known integration target. Exploring a new language has not replaced that decision.
- Atlas publishes static rules, rule documentation, compact evidence references, and explicit gaps. Consumers own project inputs, reference resolution, checks, and diagnostic severity.
- The first coverage slice remains the agreed tradition/category workflow and bounded embedded-script vocabulary.

## Recommended modules and interfaces

Three modules have separate reasons to change. These are logical ownership decisions; repository, package, language, and serialization choices remain with their existing decisions.

| Module | Interface presented to callers | Decision hidden in its implementation |
| --- | --- | --- |
| PDX Native | Open the available qualified engine and request stable engine operations or observations; return structured results with evidence and explicit capability/failure outcomes. | Platform/build detection and adapter selection, native functions and layouts, static native analysis, observation hooks, process ownership, isolation, injection, transport, deadlines, cancellation, and independent cleanup. |
| Atlas extraction | Extract evidence for a declared coverage slice using the engine interface; return supported claims, source claims, observations, and precise gaps. | Source discovery, game-property questions, probe planning, fixtures, evidence interpretation, and qualification of rule conclusions. |
| Atlas rule assembly | Assemble a snapshot from qualified claims and a coverage declaration; return rules, documentation, evidence references, gaps, and completeness results. | Normalized rule identity, evidence applicability, conflict handling, reusable rule definitions, and coverage accounting. |

The SDK crosses only the offline snapshot interface. It does not load native tooling or coordinate extraction.

PDX Native earns a module even with Atlas as its first consumer: it hides consequential platform, executable, and process-lifetime decisions. Do not build a general plugin framework or the future test-author interface. Start with the engine operations and observations Atlas actually needs.

Expose stable operations at the seam, not one wrapper for every native function. For example, registration discovery, field-read observation, reference-resolution observation, and prepared-script execution may each need several native calls or hooks in an adapter. These are illustrative responsibilities, not settled method names. Atlas sees engine concepts and captured facts; it does not receive callable native pointers, assembly, or memory layouts to interpret.

PDX Native detects the environment and selects a qualified adapter once for an engine session. A deployment may locate a game installation without specifying its OS or version. Exact executable identity and method evidence travel with results automatically. Atlas preserves them for provenance but does not branch on them or choose an adapter. An unsupported installation or unavailable operation is an explicit outcome. Hiding version mechanics does not promise identical game behavior or support for every version.

## Extraction flow and authority

1. PDX Native identifies the executable/platform and selects its qualified adapter. Atlas records relevant content, extraction-method revisions, and probe inputs alongside the returned provenance. Keep shared identity and artifacts once per run, with precise references from individual claims.
2. Atlas discovers candidates from PDX Native's registration/loader observations, documentation dumps, installed content, and config migration inputs. A discovered candidate is not yet an established game rule.
3. PDX Native establishes native ownership. For definitions, follow directory construction through the database and value class to entry loading. For script commands, follow registration to the command implementation. Return engine-level identities and evidence references while retaining native paths as producer evidence.
4. PDX Native traces a field from its authored token and owner through control flow to a typed reader and destination. Return normalized read, initialization, resolution, validation, and use observations only where the connections are established. Atlas must not reconstruct those connections from offsets or disassembly.
5. Atlas supplies fixtures and observation requests where static results leave a material assumption unresolved. PDX Native executes qualified probes and correlates observations with the exact fixture, source location, owner, and processing stage.
6. Emit structured claims with conditions and evidence references. Keep parser storage, validation, and runtime outcome distinct. Return unresolved properties with the point at which analysis stopped.
7. Assemble supported claims into rules and documentation. Preserve unknown, unsupported, contradicted, and untested as distinct information. A gap that prevents a promised consumer check prevents a completeness claim.

Native offsets have meaning only within their target and established owner. Matching offsets, adjacent instructions, similar names, or config expectations cannot establish a join.

Rule assembly is deterministic over the same qualified input. It checks evidence applicability and required records; it does not magically prove an analyst's reasoning. Human analysis follows the already agreed evidence standard and retains its rationale. Conflicts with unresolved conditions leave a gap in the affected conclusion.

## Division among extraction methods

| Method | Responsibility | Limit |
| --- | --- | --- |
| Documentation dumps | Recover names, declarations, and source documentation in batches. | A declaration alone does not establish validation or execution behavior. |
| Static native analysis | Recover registration and loader ownership, reader paths, field destinations, constructor values, and supported resolution/validation patterns. | Unknown calls, state, or compiler forms remain explicit obstructions. |
| Installed-content discovery | Find examples, candidate fields, relevant dependencies, and producer fixtures independently of config. | Observed content does not establish exhaustive grammar or permissible identifiers; no catalogue ships as Atlas rules. |
| Parser and validation instrumentation | Observe reads, omissions, repeated assignments, reference resolution, and diagnostics at the relevant loading stages. | Observations establish the tested conditions, not universal rules from finite examples. |
| Behavioral probes | Test use-time relationships, conditional behavior, and scope availability when those claims require execution evidence. | A successful case does not establish every path or make runtime behavior a machine-executable Atlas model. |

Use static analysis where it can establish a claim, with live evidence where needed. Do not require a game launch for every property, or demand that one method recover every kind of rule.

## Native analysis and extension

Keep bounded native analysis inside PDX Native. Share instruction decoding, register/value provenance, call summaries, and supported control-flow traversal where they encode the same knowledge; isolate target-specific implementations behind its adapters. Extend those methods for demonstrated compiler and reader patterns rather than storing a handwritten answer for each command. Merely hiding live function calls would be insufficient if Atlas still interpreted platform-specific machine code.

Call summaries must describe established effects and limits. An unfamiliar call does not acquire a summary merely because its name looks familiar. Stop or narrow the result when required behavior cannot be traced. There is no commitment to building a general decompiler or solving arbitrary program behavior.

The existing failed reference cases distinguish real extension needs:

- The inline typed-map lookup is already recognized.
- `CAddDistrictEffect::PostInit` uses a linear search, requiring loop and comparison tracing or suitably bounded observations.
- `CCreateArmyEffect::PostInit` includes event-target-chain handling. Its conditions must be preserved; it cannot simply be labelled a direct database lookup.
- The previously inspected planet-class case uses an out-of-line getter, motivating a qualified call summary.

These are development cases now, not fresh held-out cases. After extending a method, freeze it and test a newly selected case before claiming reuse. Keep negative controls for wrong ownership, clobbered values, changed branches, missing hooks, and incomplete evidence.

Conditional validation follows the same approach: PDX Native traces the condition and constrained property together and reports the established relationship with evidence. Atlas determines the supported rule claim and its conditions. If a bound depends on a referenced definition, publish the relationship for consumers to apply. Keep observed project values out of the rule. Put recovery behavior and scope identity in documentation, as already agreed.

An explicit, evidence-backed manual exception remains possible under the prior policy. Record the exact claim, conditions, current obstacle, and route to removal. It must not silently become a second rule database or be reported as automatic extraction.

## PDX Native ownership

Reuse mechanisms from the real-game experiments, not their complete ready-world contract or unfinished product implementation.

| Concern | Owner |
| --- | --- |
| Process creation, isolation, injection, resource journal, deadlines, transport, disposal | PDX Native |
| Target discovery/selection, native loading conventions, engine entry points, hook installation machinery | PDX Native's qualified adapters |
| All hook locations, native signatures, field layouts, reader implementations, and compiler-pattern analysis | PDX Native |
| Qualification of native operations, observation methods, and target-specific assumptions | PDX Native |
| Qualification of a rule conclusion from engine observations and other evidence | Atlas extraction |
| Fixtures, probe matrices, evidence interpretation, rule coverage | Atlas extraction |
| Future test discovery, author hooks, assertions, and test reporting | Future testing framework |

Each native assumption has one home in PDX Native, including operations initially needed only by Atlas. Atlas requests capabilities through the same interface as future consumers. PDX Native establishes what was called, read, or observed and under which conditions; Atlas owns what those results establish about an authoring rule. Missing capabilities lead to PDX Native extensions or explicit gaps, never an Atlas escape hatch for direct native calls.

Atlas supplies fixtures, engine-level operation/observation requests, and time bounds. PDX Native selects and fixes the target, adapter, native probe code, and required hook phases before launch. It owns the process through completion and cleanup. A native job need not load a saved world if its purpose is only to observe registration or parsing.

Early hooks require explicit ordering evidence: the required hooks are installed before the relevant phase starts, the phase is observed, and the observation stream is complete enough for the claim. A delay or a later ready-world marker cannot establish this. Missed activation, unsupported hook timing, or lost records must be reported as unavailable or incomplete evidence. Independently supported static claims can remain available.

The mechanism that supplies this ordering is target-specific and still requires qualification. Do not promise that injecting before process resume is safe or sufficient without proving it. The interface states the required ordering; a qualified adapter establishes how it is achieved.

Keep independent cleanup after worker loss, partial-launch ownership, raw failure evidence, and exact target checks from the runner design. Do not replay an operation whose completion is uncertain. A fresh isolated attempt receives a new identity. Completion and confirmed disposal are separate results; unclear cleanup prevents reuse of the supposed isolation.

## Bounded verification before relying on the design

These are open empirical gates, not additional product choices or claims of completed work:

1. **Reference-method extension:** recover appropriately bounded results for the two failed resolver forms and the getter case; then evaluate a fresh held-out case. Report shared-method changes, command-specific changes, agent interventions, human interventions, and retained unknowns.
2. **Early observation:** on the selected target, prove required hook activation precedes the selected registration/loading phase. Include a deliberately late or missing hook and worker-loss cleanup. The existing delayed Windows injection does not pass this gate.
3. **Tradition path:** trace one representative tradition field through loader ownership, reader, applicable validation, qualified evidence, offline rule assembly, and an SDK check. Include an invalid input and a conditional/shared-reader case. This tests the proposed seams without claiming the full slice is complete.

Tracked as separate bounded prototypes:

- [Validate reusable reference observations through PDX Native](https://linear.app/unnamed-system/issue/SDK-482/validate-reusable-reference-observations-through-the-native-foundation).
- [Verify native observations before registration and parsing](https://linear.app/unnamed-system/issue/SDK-483/verify-native-observations-before-registration-and-parsing).
- [Validate the tradition evidence-to-SDK interface with a bounded prototype](https://linear.app/unnamed-system/issue/SDK-484/validate-the-tradition-evidence-to-sdk-interface-with-a-bounded).

All three checks must use PDX Native's interface from Atlas. A successful experiment that requires Atlas to select an OS/build, interpret native layouts, or call native functions directly does not establish the proposed seam. A later second-target replay must keep the Atlas extraction logic unchanged while retaining observations and explicit capability gaps. An apparent platform difference triggers investigation; it does not introduce speculative platform-specific rule variants.

Measure first-time tooling, shared-method work, target-specific work, fixture corrections, and per-rule reasoning separately. Preserve active effort when measured; identify missing measurements. Output counts and process elapsed time are not human-effort measurements.

The supported-build decision owns the initial target and second-build replay. The consumer-contract decision owns concrete snapshot representation. The repository/release decision owns physical packaging. Full tradition coverage and production acceptance remain later work. Atlas does not wait for the paused testing framework to satisfy those obligations.

## Evidence inspected in this session

Extraction evidence root: `/Users/jackson/Developer/typed-pdxscript-prototype/spikes/config-information-extraction/`.

- `findings.md`, `evidence/phase2-findings.md`, and `evidence/phase3-findings.md` establish the spike's scope and limits.
- `loader_paths.py` and `evidence/loader-paths.json` include both tradition databases and their value classes. This is ownership evidence, not complete schema evidence.
- `reference_extract.py`, `native_cfg.py`, `native_fields.py`, `numeric_extract.py`, and retained resolver disassembly expose the actual analysis limits.
- Offline replay in this session reproduced one matched reference case and three unknown cases, including both frozen unfamiliar cases. All seven field-provenance controls passed. No game was launched and no new game property was qualified.

Runner evidence in `/Users/jackson/Developer/pdx-sdk`:

- Commit `d53cb4e0a47123d33db8e0b885196a7b185230a0`, `packages/sdk-testing/prototype/compatibility-harness/contract.ts`: launch returns a ready-world snapshot; disposal is independently callable.
- The same commit, `packages/sdk-testing/prototype/compatibility-harness/windows-446/host.py`: the game resumes before injection, which occurs after a delay of more than eight seconds. This supplies no before-parsing guarantee.
- Commit `cb8da78ab749b7c60baf64b2c7eb0377a3ff04a3`, `docs/specs/stellaris-real-game-testing.md`: the specified adapter separation, independent ownership/cleanup, exact evidence, and Windows-first direction. This document is on `spike/sdk-testing-framework`, not the current `main` checkout; it is not evidence of a completed production framework.

These are local evidence pointers. This proposal neither selects Atlas's supported platform nor requalifies historical evidence for another target.

## Reference observation review — 2026-09-17

Jackson accepted the bounded result of [Validate reusable reference observations through PDX Native](https://linear.app/unnamed-system/issue/SDK-482/validate-reusable-reference-observations-through-pdx-native). The native interface with per-OS/build adapters remains the agreed architecture; Atlas does not select or inspect those adapters.

Require stage, conditional alternatives, incomplete evidence joins, and separate native-method and rule qualification in observation records. Target-local opaque join handles are separate from stable rule identities. Partial/unknown outcomes never establish unconditional rules.

The pinned ARM64 4.5 beta prototype passed 27 method controls, and its frozen district scan method transferred to a fresh relic initializer without source changes. Army event-target conditions, remaining reader/loader/callee joins, live behavior, and other targets remain unqualified. These are retained coverage and qualification gaps, not blockers to this architecture decision. The existing portability/maintenance and final release-acceptance decisions own their respective remaining gates; no maintenance-cost or cross-platform support claim follows from this result.

Retained prototype: `/Users/jackson/Developer/typed-pdxscript-prototype/spikes/config-information-extraction/reference-observation-prototype/`, branch `prototype/sdk-482-reference-observations`, experiment commit `c2258d2ef5bdcb195f6d2a3a88d7a45e2f80cc57`. See `findings.md`, `interface.md`, and the frozen evidence manifests. The ticket resolution is canonical.


## Early observation review — 2026-09-17

Jackson accepted the bounded result of [Verify native observations before registration and parsing](https://linear.app/unnamed-system/issue/SDK-483/verify-native-observations-before-registration-and-parsing). On the pinned native ARM64 Stellaris 4.5 beta, hooks were active at the loader entry before startup registration and the selected category parsing path. Three initial registration call entries and both fixture fields were captured with explicit ordering and completion witnesses.

Keep activation, bounded observation completion, and confirmed disposal separate in the PDX Native result. Missing hooks produce unavailable capabilities; sequence/count gaps produce incomplete evidence; worker loss cannot imply completion. PDX Native retains process ownership independently of the observation worker. Atlas supplies only engine-level requests, fixtures, and a deadline.

The qualified prototype mechanism on this target uses an independent parent to create a suspended child, a debugger worker to attach and verify hooks at `_dyld_start`, and the parent to confirm exit and reaping. This mechanism stays inside PDX Native; it is not a universal implementation mandate or a claim that suspension alone establishes ordering. The normal, missing-hook, lost-record, and worker-loss scenarios all confirmed final disposal.

Earlier access and presentation-guard failures remain evidence. A superseded debugger-owned failure left an unreaped defunct process entry; it remains recorded as unconfirmed disposal rather than being overwritten by the final results. Independent direct-child ownership resolved that flaw for all final cases.

Other builds and platforms, full registration/loader coverage, engine validation/runtime behavior, owner-process loss, and recurring maintenance cost remain unqualified. They do not block this bounded interface decision. Existing portability/maintenance and release-acceptance work retain their respective gates; no production support is approved by closure.

Retained prototype: `/Users/jackson/Developer/typed-pdxscript-prototype/spikes/config-information-extraction/early-observation-prototype/`, branch `prototype/sdk-483-early-observations`, experiment commit `4188faf564b8609fde747da09bd4b8db8045b315`. See `findings.md`, `interface.md`, and the immutable per-run evidence. The attached review summary preserves the review-time snapshot; the ticket resolution records acceptance.
