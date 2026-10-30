# Difficulty and personal progress

## Meaning of the numbers

The **computer level** is a stable, versioned configuration. It is not a human rating and is not a replica of Lichess server levels. Fairy-Stockfish 14 runs with one thread and 32 MB hash; each search ends at the first of its depth/node limits. CPU speed affects response time rather than the intended search budget. Engine and profile changes require a new progress-scale version before existing records can be compared without qualification.

| Level | Skill | Nodes | Depth limit | Internal reference value |
| --- | --- | --- | --- | --- |
| 1 | -20 | 100 | 1 | 600 |
| 2 | -10 | 300 | 2 | 850 |
| 3 | -5 | 1,000 | 3 | 1,100 |
| 4 | 0 | 3,000 | 5 | 1,350 |
| 5 | 5 | 10,000 | 7 | 1,600 |
| 6 | 10 | 30,000 | 10 | 1,850 |
| 7 | 15 | 100,000 | 14 | 2,100 |
| 8 | 20 | 300,000 | 18 | 2,350 |

Reference values are deliberately chosen app-scale anchors. They are **not calibrated Elo estimates** of these engine configurations or of people who beat them. Opponent deviation is 200 on this internal scale. The app calls the resulting number a **Local progress score**, keeps the explanation beside it, and reports `calibrated: false` in its protocol. Personal results by level remain available independently of this experimental score.

## Glicko-2

The implementation follows [Mark Glickman's March 2022 worked example and equations](https://www.glicko.net/glicko/glicko2.pdf); the published 1500/200/0.06 example is a regression fixture. Initial player score/deviation/volatility are 1500/350/0.06, tau is 0.5, and one completed game is one rating period. Glickman's paper recommends larger rating periods for best statistical performance; per-game settlement is a deliberate responsive-app adaptation, not a claim that a few games establish accurate playing strength. No time-based inactivity adjustment is applied in this first version.

The UI reports an approximate ±2 RD interval and a question mark while RD exceeds 110. A narrower numerical interval describes uncertainty under this app's model; it does not establish external Elo accuracy. Ratings are reconstructed from immutable completed scored games, so reopening/exporting a game never settles it again.

Separate pools use player name, variant, and estimated time-control group. Untimed is separate; the other groups use starting seconds plus 40 increments: Bullet <180, Blitz <480, Rapid <1500, otherwise Classical. Local values are not comparable across pools. Renaming a player via the setup field selects/creates a profile rather than rewriting another profile's past games.

Scored games start from the variant's standard arrangement (a numbered Chess960 start is allowed), keep the selected opponent level, and disable takebacks and live analysis. Custom-position games and imports are practice/review. Practice results appear in the per-level table but never alter the Glicko-2 score; the table is a personal record, not a calibrated difficulty recommendation. Resignation/time/checkmate settle a result once. Cancelling an engine search, closing/reopening, reviewing, importing or exporting do not create a result. Engine failures retain the position and never substitute a random move.
