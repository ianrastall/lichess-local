"""Pin and verify offline assets. Never fetch anything when the installed app runs."""
import hashlib, io, json, re, tarfile, subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PUBLIC = ROOT / "web/public"
CACHE = ROOT / "artifacts/downloads"
LILA = "b8849a9d79288a67fc5250be8e470ad5a5954fa4"
ENGINE_REV = "f3e6969d11d1bec17eba26e7ae0e629ad4af71dd"
EXPECTED = json.loads((ROOT / "upstream-manifest.json").read_text()) if (ROOT / "upstream-manifest.json").exists() else None

def fetch(url, dest):
    dest.parent.mkdir(parents=True, exist_ok=True)
    if not dest.exists():
        print("Downloading", url, flush=True)
        subprocess.run(["curl.exe", "--fail", "--location", "--silent", "--show-error", "--retry", "3", "--output", str(dest.with_suffix(dest.suffix + ".part")), url], check=True)
        dest.with_suffix(dest.suffix + ".part").replace(dest)
    return dest.read_bytes()

def sha(data): return hashlib.sha256(data).hexdigest()

def main():
    receipt = {"schema": 1, "sources": {}, "files": []}
    PUBLIC.mkdir(parents=True, exist_ok=True)
    upstream = fetch(f"https://codeload.github.com/lichess-org/lila/tar.gz/{LILA}", CACHE / f"lila-{LILA}.tar.gz")
    if EXPECTED and sha(upstream) != EXPECTED['sources']['lila']['archiveSha256']: raise RuntimeError('Pinned Lila archive checksum mismatch')
    receipt["sources"]["lila"] = {"revision": LILA, "archiveSha256": sha(upstream)}
    with tarfile.open(fileobj=io.BytesIO(upstream), mode="r:gz") as archive:
        for entry in archive:
            relative = entry.name.split("/", 1)[-1]
            if not entry.isfile(): continue
            destination = None
            if relative.startswith("public/piece/"): destination = PUBLIC / "assets" / relative.removeprefix("public/")
            elif relative.startswith("public/images/board/"): destination = PUBLIC / "assets" / relative.removeprefix("public/images/")
            elif relative.startswith("public/images/staunton/"): destination = PUBLIC / "assets" / relative.removeprefix("public/images/")
            elif relative.startswith("public/sound/standard/"): destination = PUBLIC / "assets" / relative.removeprefix("public/")
            elif relative in ("COPYING.md", "LICENSE"): destination = PUBLIC / "notices" / ("lila-" + relative)
            elif relative in ("modules/pref/src/main/Theme.scala", "modules/pref/src/main/PieceSet.scala"):
                destination = ROOT / "tools/upstream" / Path(relative).name
            if destination is None: continue
            if not destination.resolve().is_relative_to(ROOT): raise RuntimeError("Unsafe archive path")
            data = archive.extractfile(entry).read()
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
            receipt["files"].append({"path": destination.relative_to(ROOT).as_posix(), "source": "lila", "upstreamPath": relative, "sha256": sha(data)})
    piece_source = (ROOT / "tools/upstream/PieceSet.scala").read_text()
    parts = piece_source.split("object PieceSet3d")
    pieces = re.findall(r'PieceSet\("([^\"]+)"', parts[0])
    pieces3d = re.findall(r'PieceSet\("([^\"]+)"', parts[1])
    theme_source = (ROOT / "tools/upstream/Theme.scala").read_text()
    parts = theme_source.split("object Theme3d")
    boards = [{"id": name, "file": file, "kind": "2d"} for name, file in re.findall(r'Theme\("([^\"]+)", "([^\"]+)"', parts[0])]
    boards3d = [{"id": name, "file": file, "kind": "3d"} for name, file in re.findall(r'Theme\("([^\"]+)", "([^\"]+)"', parts[1])]
    # Metadata is derived from the pinned upstream catalogues; no CDN is used.
    (PUBLIC / "assets/catalog.json").write_text(json.dumps({"revision": LILA, "pieces": pieces, "pieces3d": pieces3d, "boards": boards, "boards3d": boards3d}, indent=2))
    revision = EXPECTED['sources']['chess-openings']['revision'] if EXPECTED else json.loads(fetch("https://api.github.com/repos/lichess-org/chess-openings/commits/master", CACHE / "openings-revision.json"))["sha"]
    receipt["sources"]["chess-openings"] = {"revision": revision, "license": "CC0-1.0"}
    for file in "a.tsv b.tsv c.tsv d.tsv e.tsv COPYING.txt".split():
        data = fetch(f"https://raw.githubusercontent.com/lichess-org/chess-openings/{revision}/{file}", CACHE / f"openings-{revision}-{file}")
        dest = PUBLIC / ("notices/chess-openings-COPYING.txt" if file == "COPYING.txt" else "openings/" + file)
        dest.parent.mkdir(parents=True, exist_ok=True); dest.write_bytes(data)
        receipt["files"].append({"path": dest.relative_to(ROOT).as_posix(), "source": "chess-openings", "upstreamPath": file, "sha256": sha(data)})
    engine_url = "https://github.com/fairy-stockfish/Fairy-Stockfish/releases/download/fairy_sf_14/fairy-stockfish_x86-64.exe"
    engine = fetch(engine_url, CACHE / "fairy-stockfish-14-x64.exe")
    if EXPECTED and sha(engine) != EXPECTED['sources']['fairy-stockfish']['sha256']: raise RuntimeError('Pinned engine checksum mismatch')
    engine_dir = ROOT / "artifacts/engine"; engine_dir.mkdir(parents=True, exist_ok=True)
    (engine_dir / "fairy-stockfish.exe").write_bytes(engine)
    receipt["sources"]["fairy-stockfish"] = {"revision": ENGINE_REV, "tag": "fairy_sf_14", "url": engine_url, "sha256": sha(engine), "sourceUrl": f"https://github.com/fairy-stockfish/Fairy-Stockfish/tree/{ENGINE_REV}"}
    license_data = fetch(f"https://raw.githubusercontent.com/fairy-stockfish/Fairy-Stockfish/{ENGINE_REV}/Copying.txt", CACHE / "fairy-Copying.txt")
    (PUBLIC / "notices/Fairy-Stockfish-Copying.txt").write_bytes(license_data)
    (ROOT / "upstream-manifest.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"Prepared {len(pieces)} 2D / {len(pieces3d)} 3D piece sets, {len(boards)} 2D / {len(boards3d)} 3D boards.")

if __name__ == "__main__": main()
