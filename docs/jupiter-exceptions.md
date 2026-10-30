# Jupiter exceptions

30 October 2026 · Standard 1.1.

1. **Embedded playing workspace.** The main chess content, its task controls and move/progress panes use local HTML/CSS/TypeScript inside WebView2 to reuse Lichess's Chessground directly, as requested by Ian. This is a content-rendering exception to native-control defaults. The native WinUI shell retains menus, file pickers, window lifecycle, installer identity and shared Appearance controls. Its web view receives the actual Jupiter.UI theme/accent resource values at runtime; no separate interface palette is maintained. Board/piece palettes remain content assets.
2. **Frontend chessops projections.** chessops parses PGN/opening data and projects positions for review. It never authorizes a live move or persists authoritative game state. Rust/shakmaty validates every move, including imports, preserving one authoritative domain owner. This avoids copying Lichess's TypeScript rules into C# while keeping native engine/persistence ownership.

The app follows the Windows 11 x64 stack, generated protobuf/C ABI boundary, shared native Jupiter.UI library, installer and Start Menu requirements. NumberBox is not used in WinUI. The HTML Chess960 index uses the browser's numeric input.
