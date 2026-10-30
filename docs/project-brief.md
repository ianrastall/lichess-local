# Lichess Local product brief

Confirmed direction, 30 October 2026. **Lichess Local** is an independent, unofficial offline Windows computer-opponent app. Boardroom retains its Maia-2 purpose and its separate history. Its possible conversational companion, inspired by Journeyman, is a future independent track; no LLM is required by Lichess Local.

The product target is the complete Lichess computer-game workflow: setup, playing and reviewing a game, with its board interactions, variants, piece/board selection and local save/replay. It does not require operating Lila's server stack. The local interface identifies itself as unofficial and never requests or creates a Lichess account. No online opponent, rating sync, multiplayer, tournaments or social features are included.

The first release provides the nine Lichess game types, eight local computer levels, standard/custom starts and all 960 Chess960 arrangements, clocks and increments, practice takebacks, resignation, available draw claims, local result/history persistence, PGN export/import, and post-game analysis. It bundles the pinned Lichess piece/board catalogues with added color palettes and whole-board material textures. Sound is optional. Native settings use Jupiter.UI.

“Full” describes the product target, not a claim of byte-for-byte equivalence to Lichess's hosted computer service. Opponent levels use this app's documented search limits; there is no claim that Level N equals the hosted service's Level N. Existing website histories, downloads, pipelines and Boardroom remain untouched.

Progress is deliberately local. Personal results by level are the primary evidence for selecting difficulty. An optional Glicko-2 progress score operates on a separate internal reference scale with displayed uncertainty. There is no externally calibrated Lichess/FIDE rating claim. Benchmarked human-strength calibration would require an independent measured dataset and cannot be inferred from engine configuration or a library's advertised Elo range.

Delivery is an installable Windows 11 x64 app with a Start Menu entry, bundled offline prerequisites, a registered uninstaller, preserved game/settings data on upgrade/uninstall, and a source distribution. The binding desktop architecture is WinUI/C# and a Rust core; Chessground runs as a local embedded content component.
