# PDX Atlas

Atlas implements a game-free config claim ledger and coverage CLI. For ledger changes, read
[the contract](docs/coverage/ledger.md) and run the checks in [README.md](README.md).

The project's architecture is located [here](/Users/jackson/Developer/pdx-foundry/native/docs/design/architecture.md).

Coverage measures established answers to config questions; agreement with CWT is not a requirement.

Use paired `<module>.rs` and `<module>/` files instead of `mod.rs`.

## Useful References

### Native Integration Knowledge
Before game launch, cleanup, injection, engine calls, memory analysis, or native target changes,
read `/Users/jackson/Developer/pdx-native/docs/engine-knowledge.md`. Native owns reusable engine
methods. Atlas owns the rule conclusions and extraction fixtures.

### CWTools Config
Path: `/Users/jackson/Developer/cwtools-stellaris-config`

This repository contains the current `.cwt` files used by the SDK for code generation. This is what Atlas aims to replace.

### SDK Testing Prototypes
Path: `/Users/jackson/Developer/pdx-foundry/native/.local`

This repository contains `.local`, an untracked folder containing prototypes created during the planning phase. If a Linear ticket mentions a prototype ticket, the outcome may be in this folder.

## Wayfinder Guidelines

The Wayfinder Skill was written with web development in mind. It may prescribe creating an interactive HTML file during a prototype task. This obviously isn't very useful for our situation, so it should be skipped. Instead, if the prototype involves a *consumer/mod author-facing decision*, create demo code files that showcase the options in question or the agent's proposed design.
