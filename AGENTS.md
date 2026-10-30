# Lichess Local

Follow ../AGENTS.md and ../_standards/. Work on main. This is an independent offline Windows app, unofficial and not connected to Lichess accounts. Preserve Boardroom, Lichess Puzzles and Chess Nerd as independent projects. Keep interface copy functional.

Rust owns game state, move validation, engine control and persistence. Chessground is a local presentation component. All commands carry a revision; stale engine replies must never alter a newer game. Keep network access blocked in the installed game view. Ship and verify an installer and a Start menu entry.

Keep all upstream assets' individual licenses, source revisions and checksums. Never call a local rating a Lichess/FIDE rating. Do not assign human Elo claims to engine difficulty levels without calibration evidence.
