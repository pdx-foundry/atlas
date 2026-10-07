# Milestone 4 capture: 2026-10-06 (SDK-625)

One live run on M452 (Stellaris 4.5.2) establishes **22,668 / 58,032 (39.061208%)** Atlas-owned
config claims. The [simplification capture](simplification.md) on M451-hotfix established
16,341 (28.158602%). No claim loses coverage. The ledger is byte-identical to that capture: 175
files, 61,146 claim occurrences and the same 19 source diagnostics. Coverage stays independent of
agreement with CWT, and recorded answers earn none.

The snapshot has 56,061 rules, 24,276 gaps and 11,171 subjects, including 2,999 command
arguments and 234 derived names.

## Where the 6,327 claims come from

| Source | Claims |
| --- | ---: |
| Command arguments: established keys of `command_grammar` (`field_existence` 2,431, `value_form` 1,321, `cardinality_maximum` 329, `push_scope` 145) | 4,226 |
| Command roots: `forms` answers the command's own `value_form` (1,132); `child_scopes` answers `push_scope` (104) | 1,236 |
| Registry fields: block, `Float`, scoped-numeric and reference value forms (390), `replace_scopes` and `push_scope` (65), repeat behavior (33), existence (1) | 489 |
| Callback entry scopes of on_actions and game rules (SDK-704, after the previous capture) | 245 |
| Type naming lines joined to derived names | 131 |
| **Total** | **6,327** |

R6 case 2 holds across the run: 1,545 commands keep an `arguments` gap (1,024 with unresolved
keys, 521 with partial keys), and only claims under a key that Native establishes join an
argument subject. A claim under any other key keeps the command's gap.

## Per-area coverage

| Area | Covered | Atlas-owned | Share | Previous |
| --- | ---: | ---: | ---: | ---: |
| Effects | 7,430 | 13,085 | 56.78% | 3,398 |
| Triggers | 4,829 | 6,708 | 71.99% | 3,401 |
| Modifiers | 1,559 | 2,243 | 69.51% | 1,559 |
| Scopes and links | 754 | 825 | 91.39% | 752 |
| Localisation | 626 | 922 | 67.90% | 626 |
| On_actions | 336 | 1,514 | 22.19% | 276 |
| Game rules | 406 | 884 | 45.93% | 221 |
| Defines | 3,904 | 5,570 | 70.09% | 3,904 |
| Registry types and other files | 2,824 | 26,281 | 10.75% | 2,204 |
| **Total** | **22,668** | **58,032** | **39.06%** | **16,341** |

Areas group the ledger's `by_file` figures: Modifiers is `modifiers.cwt` and
`modifier_categories.cwt`; Scopes and links is `scopes.cwt`, `links.cwt` and `scope_links.cwt`;
Localisation is `localisation.cwt` and `localisation_links.cwt`; Defines is `common/defines/*`.
Every other file is a registry type or another config file.

## What stays uncredited by design

- **Shared weight grammar.** `modifier_rule.cwt` writes weight keys as shared aliases
  (`alias[modifier_rule:factor]`, 136 claims, 119 of them engine facts). Native answers one weight
  block for each field, with five reader identities, so these claims have no Native subject. Weight
  credit is at field level: a weight field's value form and nested grammar.
- **Naming questions with several values.** 28 naming questions state different values in one
  type, such as `desc = "$_desc"` and `desc = desc` in missions; they join no derived name. 106
  naming claims under `subtype[…]` are conditional and also unjoined. 131 of the 389 unconditioned
  naming claims are credited.
- **Fixture conclusions.** The two Atlas fixture games publish parser outcomes apart from static
  answers. The parser and diagnostic checks of SDK-542, SDK-544, SDK-545, SDK-549 and SDK-550 are
  Native tests and earn no Atlas credit.

The [gap triage](gap-triage.md#sdk-625-final-run) measures the unowned gaps again on this run.

## Capture and checks

| Input or result | Identity |
| --- | --- |
| Executable SHA-256 (Native's M452, Stellaris 4.5.2 (9776), Apple Silicon) | `c621723d9c8e0c1cd153319208d30a9dfbb9e63675be86f9d0ae7debeaa7fe1b` |
| Native revision (`main`) | `573e35f29fea8f54d32d48c90ff40c074e34b5f4` |
| Atlas revision (clean tree) | `b4e107c05f76555200047e22b709586a5fb31e04` |
| Config revision | `85747602a614ad7daa8cc66453777ecb023463a8` |
| Config content SHA-256 | `5c79cabb8d1b25e40994c0d134f5623bc2aef2a6b964059aabfc4834e6d039cc` |
| Live rule snapshot SHA-256 | `8c84d9c547d5b042b560081088246a3a4f2523cdf65e07aa97cddab0b36dca6d` |
| `ledger.json` / `coverage.json` | `ba2bb263…f67947` / `e323dcda…cab6d4` (pinned in full in the gate fixture) |
| `comparison.json` | `c438440f287ec0692dedfa4c547e8bf8483e197ba4577bad36ddb4cbcd958b5d` |

A later review fix keys the naming join by file as well as type; recomputed from the same live
snapshot, `coverage.json` is byte-identical.

The live CLI exited 0. It asked every question once, with methods unchanged during the run,
confirmed disposal of both fixture games and the loaded-modifier game, and took 7 minutes 39
seconds. Ledger and comparison exit 2 for the retained source diagnostics. The ignored
`snapshot_live` test ran a second live recording; its full live and recorded snapshots match after
source basis, snapshot identity and operation availability are normalized. Recorded support marks
the unrecorded operations (`check_script`, `dynamic_names`, `modifier_category_keys`,
`modifier_nodes`, `script_expansions`) unsupported.

Tests use `tests/fixtures/native/m452`, recorded by the same Atlas extraction on Native
`573e35f`, with the four-entry loaded-modifier reduction that its README states. SDK-608's
callback hand checks carry to M452, because the `callbacks/v2` answers are identical on both
builds (see the [ledger](ledger.md#callback-entry-scopes-sdk-704)).

## Reproduce

```sh
cargo run --release --locked -- snapshot "$STELLARIS_PATH" .scratch/m4/answers .scratch/m4/rules.json
cargo run --release --locked -- ledger --config "$PDX_CONFIG_PATH" --snapshot .scratch/m4/rules.json --output .scratch/m4/ledger
cargo run --release --locked -- compare "$PDX_CONFIG_PATH" .scratch/m4/rules.json .scratch/m4/comparison \
  --script-docs "$PDX_CONFIG_PATH/../script-docs/v4.5.0" \
  --defines "$STELLARIS_PATH/common/defines/00_defines.txt" \
  --defines "$STELLARIS_PATH/common/defines/00_defines_additional_content.txt" \
  --answers .scratch/m4/answers
ATLAS_RULE_SNAPSHOT=.scratch/m4/rules.json cargo test --locked --test full_config -- --ignored
cargo test --release --locked --test snapshot_live -- --ignored
```

Run the live commands alone on the host: Native allows one game at a time, and heavy static work
beside a fixture game can cost the fixture its loader return.
