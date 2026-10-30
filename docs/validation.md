# Validation record

Verified 30 October 2026, Windows 11 x64, version 0.1.0.

- All **827 upstream receipt files**, every catalogue asset and the engine binary passed SHA-256 validation. A Git archive of the committed source retained all 827 exact hashes, including source line endings.
- **15 Rust tests** passed. They include Glickman's published numerical example, all 960 unique starts and the standard #518 arrangement, stale/illegal moves, corrupt-data preservation, single checkmate settlement, scored/practice restrictions, Crazyhouse pocket consumption, Atomic explosion victory, King of the Hill and Three-check victories, Antichess king promotion, stationary-king Chess960 castling, repetition claims and time expiry.
- The **actual bundled Fairy-Stockfish executable** played legal human/computer sequences in all nine game types. Finish, PGN, save/reopen, history and post-game analysis checks passed.
- The **managed/native smoke** passed the generated protobuf boundary, real engine, PGN/score settlement and JSON view projection.
- **Four Chromium workflow tests** passed: play/replay/resign/analysis/progress/history; all variants with the computer as White; every 2D/3D piece set and board with no missing assets; practice takeback, keyboard notation and narrow layouts. A saved-game reload also verifies that move-list scrolling does not scroll the whole page away from the board. No Firefox/Safari testing is claimed; this is a WebView2 desktop app.
- TypeScript, Vite and the WinUI desktop build passed. The Jupiter style check reported no violations. npm audit reported zero vulnerabilities.
- The normal release command produced the installer and source ZIP. The 215 MB bundled offline WebView2 prerequisite had a valid **Microsoft Corporation Authenticode signature**. .NET and Windows App SDK runtimes are included with the application.
- The installer created **Lichess Local** in the Windows Start Menu and a registered uninstaller. Launching its actual Start shortcut opened the installed executable. Native inspection confirmed the board, restored game/history, clocks, local score and unofficial identification. High-DPI window sizing and the saved-game startup scroll issue were corrected and the installed version rechecked.
- In-place upgrade, uninstall and reinstall returned success. Existing saved-game and appearance files remained **byte-for-byte unchanged** through that lifecycle. The uninstaller removed the executable and Start shortcut; reinstall restored them. A subsequent installed launch restored the retained data.

The final packaging-only correction fixes the exact shakmaty source-tag link in third-party notices. It changes neither binaries nor game data; the installed notice and release payload were updated together.

The missing-WebView2 branch is included in setup and its Microsoft-signed standalone installer is bundled, but this machine already had WebView2; a clean machine without that shared runtime was not tested. No calibrated human Elo, exact equivalence to hosted Lichess computer levels, exhaustive Lichess UI parity, or externally verified rating accuracy is claimed. PGN import reviews the first game's main line, and clocks pause while the application is closed.
