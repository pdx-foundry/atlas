# PDX Native and Atlas support policy

Status: accepted by Jackson on 2026-09-16 after final shared-understanding confirmation. This is a local working record; the resolution comment on [Choose supported builds and the update-maintenance policy](https://linear.app/unnamed-system/issue/SDK-476/choose-supported-builds-and-the-update-maintenance-policy) is canonical. The decision is closed; native qualification and the charted experiments remain separate work.

## Ownership

**PDX Native** is the agreed name of the shared native foundation. It owns all platform and game-version operations: installation and target discovery, adapter selection, native analysis, signatures and layouts, injection and hooks, update detection, and native capability qualification.

Atlas requests stable engine operations and observations. It owns fixtures, evidence interpretation, supported rule conclusions, and coverage. Atlas extraction and its rule model are platform-independent. Exact native identities accompany evidence automatically; provenance does not become Atlas platform/version control flow. The SDK consumes offline rules.

Use one shared rule model. Do not build platform-specific rule variants before a real difference is established. Investigate apparent discrepancies first; a confirmed difference prompts a later design decision. Platform-independent config informs the expectation of common rules but is not proof of engine equivalence.

## Initial targets and support window

Start PDX Native qualification from a pinned native Apple Silicon Stellaris 4.5 beta, reusing the existing ARM64 experiments. Capture and verify the exact executable and relevant producer content before the experiment. Historical evidence does not establish that an installed or newly selected beta is the same target.

Allow Atlas preview snapshots supported by that exact beta evidence. Keep targets fixed within each experiment; there is no promise to follow every beta update. Requalify against stable 4.5 before declaring stable support.

The first stable scope requires Apple Silicon and Windows qualification through PDX Native, with Mac as the primary evidence-production environment. Reuse Windows experiments and shared evidence where justified; do not duplicate every probe by default. Linux and Intel Mac are outside the initial support promise. The platform on which an offline consumer runs does not determine the platform on which evidence must be produced.

Actively maintain one qualified stable game release at a time. Retain older snapshots and their original applicability. A new patch is not supported automatically. Physical release ownership and sequencing remain with the existing packaging/release decision; no Atlas platform-specific release branches were agreed.

## Qualification, identity, and updates

PDX Native owns target and capability status. Distinguish a qualified operation on an identified target, a target or operation outside the declared support scope, and an in-scope target or capability whose qualification is incomplete. None of these establishes whether a script construct is allowed by the game. An incomplete observation is not a successful empty result.

Detect changed executable identity and relevant content fingerprints, not just a displayed version. PDX Native must not silently reuse unqualified native assumptions after a change. Qualification tooling may examine the new target; ordinary extraction receives explicit unavailable or unqualified outcomes, without an Atlas native workaround.

Requalify affected evidence through relevant checks or demonstrated unchanged dependencies. Keep native implementation changes separate from changes to game behavior. Shared method validation and evidence are permitted; this does not require a separate live probe for every rule.

Content identity qualifies producer observations and fixtures. It does not turn Atlas into a catalogue specialized to a user's mod or project. Consumers still own project inputs and rule application.

## Snapshot corrections

A normal game update supersedes a snapshot without invalidating supported facts about its original target. If evidence behind a published claim proves invalid, retain the historical artifact, mark the affected claim or snapshot withdrawn, and publish a correction. Withdraw the whole snapshot when the defect undermines its promised coverage or evidence broadly. Exact offline representation and delivery of these notices belong to the consumer-contract and release decisions.

## Maintenance goal and required evidence

Aim for automated routine extraction and brief human review, without repeated manual analysis of individual rules. Record agent work separately. Larger repairs are exceptional investments requiring a deliberate decision, not hidden routine costs. No numerical labor estimate or maintenance-cost guarantee has been established.

Separate initial tooling, routine updates, and exceptional engine changes. Within each, measure PDX Native work, Atlas fixture/evidence/semantic/coverage work, and consumer-contract effects separately. Preserve measured human attention, agent effort, interventions, and unavailable measurements. Game-run elapsed times and output counts are not labor estimates.

[Measure Atlas extraction portability and update-maintenance effort](https://linear.app/unnamed-system/issue/SDK-485/measure-atlas-extraction-portability-and-update-maintenance-effort) is the bounded follow-up. It compares corresponding Mac ARM64 and Windows 4.5 targets, then two distinct builds on one platform. Freeze Atlas extraction logic; keep routine native porting inside PDX Native. A genuinely new engine concept may require an explicit interface amendment.

The experiment follows the reference-observation and early-observation prototypes and feeds final specification acceptance. It assesses the maintenance goal; this policy does not declare the goal achieved. Exact second-build access must be established before execution. No second ARM64 executable was established by the evidence reviewed here.

Reuse the completed Apple Silicon 4.5 and Windows 4.5/4.4.6 adapter experiments. They demonstrate bounded native scenarios and adapter fit, not Atlas extraction coverage, early-hook qualification, or economical maintenance. The paused testing framework remains outside this work.
