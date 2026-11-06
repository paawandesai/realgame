"""Fetches the world's scanned ground textures (CC0, Poly Haven) and writes
them as one stacked JPEG per map for the game (graphics plan, realistic
look; Paawan chose free scanned photo textures on 2026-11-04).

    python3 tools/textures/fetch.py            # writes crates/bevy-client/web/tex/

Layers, in order (the shader's layer index): grass, soil, rock, snow (the
ground), then bark, leaves, planks, brick, metal (models).
Each map is a vertical strip of 1024 x 1024 tiles: terrain-albedo.jpg
(colour) and terrain-normal.jpg (OpenGL normal maps). The game stacks them
into a texture array. Re-run only to change a texture; the outputs are
committed so builds need no network.
"""
import io
import json
import sys
import urllib.request
from pathlib import Path

from PIL import Image

LAYERS = [
    ("grass", "sparse_grass"),
    ("soil", "forest_ground_04"),
    ("rock", "aerial_rocks_02"),
    ("snow", "snow_field_aerial"),
    # Models (2026-11-05, Paawan: everything in the same realistic look).
    ("bark", "bark_brown_02"),
    ("leaves", "forest_leaves_02"),
    ("planks", "brown_planks_03"),
    ("brick", "red_brick_03"),
    ("metal", "rusty_metal_02"),
]
SIZE = 1024
OUT = Path(__file__).resolve().parents[2] / "crates/bevy-client/web/tex"


def get(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "realgame-texture-fetch/1.0"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read()


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    maps = {"albedo": ("Diffuse", []), "normal": ("nor_gl", [])}
    credits = []
    for name, asset in LAYERS:
        files = json.loads(get(f"https://api.polyhaven.com/files/{asset}"))
        for key, (ph, tiles) in maps.items():
            url = files[ph]["2k"]["jpg"]["url"]
            img = Image.open(io.BytesIO(get(url))).convert("RGB").resize((SIZE, SIZE), Image.LANCZOS)
            tiles.append(img)
        credits.append(f"{name}: {asset} https://polyhaven.com/a/{asset} (CC0)")
        print(name, asset, file=sys.stderr)
    for key, (_, tiles) in maps.items():
        strip = Image.new("RGB", (SIZE, SIZE * len(tiles)))
        for i, t in enumerate(tiles):
            strip.paste(t, (0, i * SIZE))
        strip.save(OUT / f"terrain-{key}.jpg", quality=86, optimize=True, progressive=False)
    (OUT / "CREDITS.txt").write_text(
        "Scanned textures from Poly Haven, CC0 (public domain).\n"
        "Fetched and stacked by tools/textures/fetch.py.\n\n" + "\n".join(credits) + "\n"
    )


if __name__ == "__main__":
    main()
