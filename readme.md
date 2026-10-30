# Lichess Local

An unofficial Windows app for playing offline against Fairy-Stockfish, using Lichess board components and artwork. No account, server, or connection is required to play. This app is not affiliated with or endorsed by Lichess.

Windows 11 24H2 or later, x64. Follows Jupiter style standard 1.1 with the documented [embedded chess-workspace exceptions](docs/jupiter-exceptions.md).

## Playing

Launch **Lichess Local** from Start. Select your player name, variant, side, computer level and clock, then choose **New game**. Standard chess, Chess960, Crazyhouse, King of the Hill, Three-check, Antichess, Atomic, Horde and Racing Kings are supported.

Move by clicking or dragging pieces, or enter SAN/UCI notation in **Keyboard move**. The expandable **Text board** provides named square buttons. Crazyhouse exposes pocket pieces. Promotions offer the variant's legal choices. Right-drag draws arrows. Arrow keys and Home/End navigate the game; F flips the board; Escape clears a premove. Takebacks are available in practice games. Completed games can be replayed, analysed locally, saved as PGN, and reopened from **Saved games**. PGN import opens the first game for review and can continue from a selected position as a practice game.

The active game and clocks are saved between launches. Clocks pause while the app is closed. Finished results are settled once. New game asks before resigning an unfinished game. Other players' progress is separate. Custom starting positions are practice games.

## Difficulty and progress

Eight **local levels** use fixed engine skill, depth and node limits. They do not claim the same strength as Lichess website levels or a human Elo rating. Your results against each level provide a direct record of the difficulty you can handle.

Optional scored games update an experimental **Local progress score** with Glicko-2. The reference values are an explicitly defined internal app scale, not empirically calibrated human ratings. Uncertainty and provisional status remain visible. Each player/variant/time-control group has a separate history. Takebacks and live engine analysis are unavailable in scored games; practice games do not change the score. See [the rating contract](docs/ratings.md).

## Appearance

The pinned Lichess catalogues supply all 42 2D and 11 3D piece sets, 25 2D and 19 3D board themes. Original artwork retains its individual credits and licenses. Additional solid palettes, custom colors, and two quiet whole-board textures are included. Texture grain is generated across the whole board rather than repeated per square.

**File > Settings > Appearance** uses Jupiter's shared theme and system/custom accent controls. Board colors are independent.

## Build and delivery

Requires .NET SDK 10.0.113, Rust 1.97.0/MSVC, Node 24, Python 3.12+, Inno Setup 6, and the shared `../jupiter-ui` source. Build-time downloads require a connection; the installed app does not.

```powershell
pwsh -File build.ps1
```

The normal build verifies upstream checksums, runs Rust tests and live engine/managed binding/browser checks, builds the local web view and WinUI shell, then produces `artifacts/installer/LichessLocal-0.1.0-Setup.exe` and a source ZIP containing both this app and the exact shared Jupiter.UI working files used for the build. The installer bundles .NET/Windows App SDK prerequisites and the Microsoft-signed offline WebView2 installer. It creates a Start entry and registered uninstaller; upgrades/uninstall preserve game data at `%LOCALAPPDATA%/Jupiter/LichessLocal`.

Application code is AGPL-3.0-or-later. [Upstream receipts](upstream-manifest.json), original asset licenses and third-party notices are shipped. [Architecture](docs/architecture.md), [scope](docs/project-brief.md), [validation](docs/validation.md).
