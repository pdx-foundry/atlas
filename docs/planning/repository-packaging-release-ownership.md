# Atlas repository, packaging, and release ownership

Status: accepted by Jackson on 2026-09-16 after final shared-understanding confirmation. The resolution comment on the linked decision is canonical; this document is its supporting artifact. This is a planning decision, not completed infrastructure or a publication.

Decision: [Decide Atlas repository, packaging, and release ownership](https://linear.app/unnamed-system/issue/SDK-479/decide-atlas-repository-packaging-and-release-ownership).

## Ownership and repositories

A shared GitHub organization will be the home for the Paradox modding-tool family, with Jackson as its initial owner. Its name remains a setup decision. New projects belong there; transferring existing repositories is separate work that checks publishing and documentation settings.

Use separate public repositories for Atlas and PDX Native. Proposed repository names are `pdx-atlas` and `pdx-native`; final paths depend on organization setup. Use a private `pdx-evidence` repository in the same organization for retained evidence.

| Owner | Source and release responsibility |
| --- | --- |
| PDX Native | Native operations, target adapters, platform/build handling, qualification tools, and native source releases |
| Atlas | Extraction jobs, fixtures, evidence interpretation, rule assembly, snapshot contract, and snapshot publication |
| Consumer | Snapshot selection, integration, derived types/indexes, authoring policy, and consumer releases |
| Jackson | Initial organization administration, publication approval, and evidence retention responsibility |

The evidence store holds artifacts for both producers. PDX Native owns what establishes native qualification; Atlas owns what establishes rule claims. Storage in one place does not merge those responsibilities. Consumer dependency graphs contain snapshots, not producer tools or the game.

The SDK experiment stays in the SDK repository. The offline contract applies to that experiment without requiring production SDK adoption or publication. Atlas remains independently useful if the SDK is retired.

## Producer installation and execution

Initially build PDX Native from an exact pinned source release. Document build prerequisites and record source revisions, dependency versions, built artifact identities, and tool versions in extraction provenance. Prebuilt native binaries are deferred until repeated setup warrants them; no native package registry or implementation language is selected here.

Atlas owns the repeatable extraction entry point and pins the native dependency. Extraction runs are maintainer-started jobs on the existing Mac and Windows machines. Hosted checks validate snapshots and build public packages without installing or running Stellaris. Automatic machine scheduling is deferred.

Existing qualification policy remains in force: begin with pinned Apple Silicon 4.5 beta evidence; stable support requires the agreed Mac/Windows qualification. Exact game and relevant content identities remain producer provenance. PDX Native handles platform/version operations; Atlas does not acquire platform-specific implementations or releases.

## Snapshot delivery

Publish immutable, versioned JSON snapshot archives with manifests and checksums on the Atlas repository's GitHub Releases. Use GitHub's immutable-release facility. Keep snapshot, contract, source-release, game-applicability, and payload identities separate. The consumer snapshot includes all required offline rule/schema references and compact evidence summaries; bulk evidence is excluded.

Consumer maintainers import a particular release and verify its expected digest, contract compatibility, and applicability. They retain the selected snapshot and version/digest record in their consumer source repository and include the data in any published source package. A Rust compiler can embed the bytes with `include_bytes!`. Any generated consumer indexes remain traceable to the canonical snapshot.

Ordinary compiler builds and mod compilation do not fetch Atlas data. Installed tools require neither Stellaris nor PDX Native. Authors select a tool version and declared game target; unsupported targets are explicit rather than mapped to the nearest snapshot.

No npm or Rust data package is required initially. A later convenience package must preserve the same canonical bytes and identities rather than become another rule authority. The 369 MB historical extraction spike is not the consumer payload, and its size does not estimate snapshot size.

## Candidate review and publication

PDX Native source releases, Atlas source releases and snapshots, and consumer releases have independent lifecycles. Each extraction records exact inputs; compatibility is verified rather than inferred from simultaneous version numbers.

The proposed operating sequence makes the agreed human publication gate concrete:

1. Run extraction against fixed, qualified inputs and retain its evidence with hashes and provenance.
2. Assemble deterministic snapshot bytes and validate the contract, reference closure, evidence references, applicability, coverage/gaps, and correction records.
3. Prepare a review showing changed answers, applicability, coverage gaps, corrections, native/method qualifications, and check results. Preserve failed or contradictory observations relevant to the conclusions.
4. Jackson approves publication of those reviewed bytes. A publication job verifies their digest rather than silently rerunning extraction against new inputs.
5. Consumer maintainers update their pinned snapshot and run their own compatibility/integration checks before publishing the consumer.

These steps allocate responsibility; full first-release acceptance remains with [Define release acceptance and assemble the Atlas specification](https://linear.app/unnamed-system/issue/SDK-480/define-release-acceptance-and-assemble-the-atlas-specification). Mechanical validity alone does not prove game claims or complete coverage. Native qualification does not establish Atlas semantic coverage.

For a correction, Atlas owns the replacement snapshot and correction notice. Preserve historical bytes and identify affected snapshots/rules, the reason, and any withdrawal. Consumers deliver corrections in new tool releases. An offline tool cannot know about notices it has not received. An ordinary game update supersedes earlier applicability without declaring the old claims false.

## Evidence storage, access, and retention

Keep evidence manifests in the private evidence repository and bulk archives as release assets. Keep a second local copy outside disposable worktrees and temporary directories. Use immutable archives with content hashes and archive-relative paths; original machine paths may remain provenance, but must not be the only retrieval location.

Public snapshots retain stable evidence identities, hashes, exact supporting locations, and compact summaries. Full-artifact locators can require maintainer access; do not include credentials or expiring signed download URLs as permanent identities. Missing permission to fetch private evidence is an access limitation, not an unknown game rule.

Retain evidence supporting published claims, corrections, and method qualification for the project's lifetime, including superseded and withdrawn claims. Do not remove shared evidence while any retained claim depends on it. Retain the current spike intact as historical planning evidence. Unreferenced scratch output may be removed after confirming it is not supporting retained claims or unresolved investigations.

Verify copied archive hashes and perform an extraction/restore check before treating a remote copy as preserved. If storage moves, preserve identities and update the mapping to the new location. A retention promise and a hash alone do not establish that an artifact is recoverable.

## Public material and licensing

Use MIT for Jackson's original source and authored snapshot material, matching the SDK. Preserve copyright and permission notices for reused material, including the MIT-licensed config fork. This licensing choice covers the project's own contributions, not rights belonging to Paradox or other contributors.

Public material consists of original code, rule representations, authored explanations, and selected compact evidence summaries. Select files and fields by their actual contents and provenance; a `.json` extension does not establish that content is an original rule representation. Atlas does not ship a vanilla/mod content catalogue.

Keep the raw historical archive private. It contains game text/content observations, saves, logs, disassembly, and native artifacts as well as original sources. Public distribution of such raw categories was not approved or established by the evidence reviewed here. Any proposed exception needs a concrete rights basis for that material, rather than treating the whole archive as MIT.

Evidence inspected:

- The SDK license is MIT, copyright Jackson Yeager 2026.
- The config fork license is MIT, copyright tboby 2018. This does not establish rights to game content incorporated into historical dumps or observations.
- Paradox's [User Agreement](https://legal.paradoxplaza.com/eula), updated January 21, 2026, sections 1 and 5, distinguishes original contributions from Paradox/third-party material. It does not provide blanket permission to publish this mixed archive. No conclusion that all derived artifacts are prohibited or cleared is made here.

## Preservation handoff

[Preserve Atlas planning and extraction evidence](https://linear.app/unnamed-system/issue/SDK-486/preserve-atlas-planning-and-extraction-evidence) is the bounded prerequisite before disposable paths are cleaned up and before final specification acceptance relies on durable evidence links. It preserves decision evidence; it is not production Atlas implementation.

The task should:

1. Preserve the current Atlas glossary/planning files and the complete `spikes/config-information-extraction/` directory, currently about 369 MB and untracked in the prototype repository.
2. Preserve the temporary extraction-spike handoff and follow the spike's cited standalone/worktree research and native-adapter evidence pointers. Inventory dependencies outside the spike instead of assuming the directory is self-contained.
3. Record source revisions, licenses, input identities, archive hashes, and an original-path-to-archive-path index. Identify missing inputs and distinguish retained evidence replay from a fresh game run. Do not claim reproducibility of a missing historical executable or environment.
4. Make a durable local copy first if organization setup is pending. Add the private remote copy once its destination exists, verify both, and attach durable locators to the Atlas project and relevant decisions. Originals remain until preservation is verified.

Starting pointers:

- `/Users/jackson/Developer/pdx-atlas/`
- `/Users/jackson/Developer/typed-pdxscript-prototype/spikes/config-information-extraction/`
- `/var/folders/f8/kl_xk4y94nl5z0pzs7jhn9t00000gn/T/stellaris-config-information-extraction-spike-handoff.md`
- `/Users/jackson/Documents/Codex/2026-09-15/stellaris-engine-rule-research/outputs/`
- `/Users/jackson/.codex/worktrees/8b6b/pdx-sdk/docs/research/engine-derived-rules/`
- Source/evidence references in the resolved architecture and supported-build decisions, including native adapter evidence from the SDK checkout, retained Git objects, and Linear attachments.

## Remaining setup and scope

Organization naming and creation, exact repository paths, existing-repository transfers, public repository setup, release automation, and production publishing remain subsequent work. The organization name is explicitly open; the ownership, storage, and delivery policies do not depend on a particular spelling.

Native prebuilt distribution, automatic extraction scheduling, convenience data packages, and compiler implementation are deferred. Existing extraction/interface/maintenance prototypes and first-release acceptance remain distinct from this packaging decision.

## Implementation references

- [GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
- [GitHub organizations](https://docs.github.com/en/organizations/collaborating-with-groups-in-organizations/about-organizations)
- [Rust include_bytes!](https://doc.rust-lang.org/std/macro.include_bytes.html)
- [Cargo package inclusion](https://doc.rust-lang.org/cargo/reference/manifest.html#the-exclude-and-include-fields)
