# PDX Atlas

Atlas implements a game-free config claim ledger and coverage CLI. For ledger changes, read
[the contract](docs/coverage/ledger.md) and run the checks in [README.md](README.md).
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
Path: `/Users/jackson/Developer/pdx-sdk/packages/sdk-testing/prototype`
Path: `/Users/jackson/Developer/pdx-sdk/.scratch/sdk-testing`

This package within `pdx-sdk` contains prototyping tools for a "Playwright for Stellaris" project. The prototyping tools should be useful for native operations, like launching the game, memory access, and calling game engine functions.

### Typed PDXScript Prototype
Path: `/Users/jackson/Developer/typed-pdxscript-prototype`

This folder is a prototype for a "TypeScript, but for Paradox Language" project. It aims to extend the modding language with actual typing and a few other features. The compiler will be the first production consumer for Atlas.

## Wayfinder Guidelines

The Wayfinder Skill was written with web development in mind. It may prescribe creating an interactive HTML file during a prototype task. This obviously isn't very useful for our situation, so it should be skipped. Instead, if the prototype involves a *consumer/mod author-facing decision*, create demo code files that showcase the options in question or the agent's proposed design.
