import json,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
target=ROOT/'web/public/notices';target.mkdir(parents=True,exist_ok=True)
components=[('@lichess-org/chessground','LICENSE','https://github.com/lichess-org/chessground'),('chessops','LICENSE.txt','https://github.com/niklasf/chessops')]
entries=[]
for name,license_file,url in components:
    package=ROOT/'web/node_modules'/name
    version=json.loads((package/'package.json').read_text())['version']
    shutil.copyfile(package/license_file,target/(name.replace('/','-').replace('@','')+'-LICENSE.txt'))
    entries.append(f'{name} {version}\nSource: {url}\nLicense: GPL-3.0-or-later\n')
entries += ['shakmaty 0.30.2\nSource: https://github.com/niklasf/shakmaty/tree/shakmaty-0.30.2\nLicense: GPL-3.0-or-later\n','Fairy-Stockfish 14\nSource: https://github.com/fairy-stockfish/Fairy-Stockfish/tree/f3e6969d11d1bec17eba26e7ae0e629ad4af71dd\nLicense: GPL-3.0-or-later\n','Lichess artwork: see lila-COPYING.md for each set’s original author/license. Some sets are noncommercial.\n','Lichess opening names: CC0-1.0; see chess-openings-COPYING.txt.\n','Glicko-2 algorithm: Mark E. Glickman, https://www.glicko.net/glicko/glicko2.pdf\n','Lichess Local source: https://github.com/ianrastall/lichess-local\nApplication code: AGPL-3.0-or-later.\n','Microsoft WebView2 runtime: separately licensed Microsoft component; offline installer distributed unmodified with a valid Microsoft Authenticode signature.\n']
(target/'THIRD-PARTY.txt').write_text('\n'.join(entries),encoding='utf-8')
print('Staged original component licenses and source links.')
