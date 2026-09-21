# First tradition rule snapshot measurement

SDK-558's live `pdx-atlas snapshot` command asked Native for the two tradition registries,
their discovered root fields, and the bounded fixture outcomes. The exact installed executable
SHA-256 was `3d4c8a7046d87175ce7e3b513b1a2ce589050d654d332744518a49d13ac82216`
(Native's M45-observe target). The live command confirmed disposal for every game session.

| Input or result | Identity |
| --- | --- |
| Config revision | `76be5782683d09a9dbe304a512690eed6a2fa425` |
| Config content SHA-256 | `ad0637833d99e28c8f8eb1a4cd5f835d6b92d65d1183b1a16a062ce76f98b8df` |
| Atlas rule snapshot SHA-256 | `0a8ab99981752a2e841a30147442e18a864102d0c264fc733248f9f7f2218542` |
| Recorded snapshot SHA-256 | `ce3dc77f37095efc1760932c1afa5bc65bd1cfca24a4313302f5d647469764fc` |
| Live Atlas-owned coverage | **20 / 57,282 (0.034915%)** |
| Recorded Atlas-owned coverage | **0 / 57,282 (0%)** |

The 20 credited claims, all in `common/traditions.cwt`, are:

- Type existence and loader path for `tradition` and `tradition_category` (four claims).
- `tradition` field existence: `unlocks_agenda`, `on_enabled`,
  `custom_tooltip_with_modifiers`, `modifier`, `ai_weight`, `tradition_swap`,
  `triggered_modifier`, `custom_tooltip`, and `possible` (nine claims).
- `tradition_category` field existence: `tree_template`, `potential`,
  `traditions`, `ai_weight`, `finish_bonus`, `desc`, and `adoption_bonus`
  (seven claims).

Reader value-form claims and bounded parser outcomes remain in the snapshot, but applicable
reference or nested-grammar gaps block complete credit for the corresponding config questions.
Engine-only fields remain Atlas-only questions. The ledger reports eight existing source
diagnostics in this config revision, so the denominator covers inventoried claims rather than
a claim of complete config parsing. The `ledger` command writes both reports and exits 2 for
those diagnostics; snapshot production itself exited successfully.

The live and recorded snapshots contain the same records after normalizing Native `basis`,
its source keys, and the snapshot identity. Their identities differ because the records include
their evidence basis. A recorded source is never qualified for current-engine coverage.
