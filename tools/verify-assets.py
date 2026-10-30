import hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
manifest=json.loads((ROOT/'upstream-manifest.json').read_text())
for item in manifest['files']:
    path=(ROOT/item['path']).resolve()
    if not path.is_relative_to(ROOT):raise RuntimeError('Manifest path escapes repository')
    if hashlib.sha256(path.read_bytes()).hexdigest()!=item['sha256']:raise RuntimeError(f"Asset checksum failed: {item['path']}")
engine=ROOT/'artifacts/engine/fairy-stockfish.exe'
if hashlib.sha256(engine.read_bytes()).hexdigest()!=manifest['sources']['fairy-stockfish']['sha256']:raise RuntimeError('Engine checksum failed')
catalog=json.loads((ROOT/'web/public/assets/catalog.json').read_text())
missing=[]
for name in catalog['pieces']:
    for color in 'wb':
        for role in 'PNBRQK':
            file=ROOT/f'web/public/assets/piece/{name}/{color}{role}.svg'
            if not file.exists() and not file.with_suffix('.webp').exists() and not (file.parent/f'{color}.svg').exists():missing.append(str(file.relative_to(ROOT)))
for name in catalog['pieces3d']:
    for color in ('White','Black'):
        for role in ('Pawn','Knight','Bishop','Rook','Queen','King'):
            file=ROOT/f'web/public/assets/staunton/piece/{name}/{color}-{role}.webp'
            if not file.exists():missing.append(str(file.relative_to(ROOT)))
for group,folder in [('boards','board'),('boards3d','staunton/board')]:
    for board in catalog[group]:
        if not (ROOT/f"web/public/assets/{folder}/{board['file']}").exists():missing.append(board['file'])
if missing:raise RuntimeError('Catalogue assets missing: '+', '.join(missing))
print(f"Verified {len(manifest['files'])} upstream files, engine binary, and all catalogue entries.")
