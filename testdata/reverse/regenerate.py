#!/usr/bin/env python3
"""Explicitly recapture reverse goldens. Requires Pillow and network access.

Run from any directory after `cargo build --features cli`. All references must
match an independent raster oracle before any fixtures are replaced. This is
never called by normal tests or CI, and has no local-renderer fallback.
"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import urllib.request

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "testdata/reverse"
CASES = [("shipping", 8, 128), ("rgba", 8, 128), ("threshold", 12, 200)]


def swatches():
    # Hand-specified ink decisions, independent of the Rust luminance function.
    # Rows: white/black/127/128; 199/200/red/green; blue/yellow/clear/alpha127;
    # alpha128/alpha56/half-alpha red/half-alpha green.
    colors = [
        (255, 255, 255, 255), (0, 0, 0, 255), (127, 127, 127, 255), (128, 128, 128, 255),
        (199, 199, 199, 255), (200, 200, 200, 255), (255, 0, 0, 255), (0, 255, 0, 255),
        (0, 0, 255, 255), (255, 255, 0, 255), (0, 0, 0, 0), (0, 0, 0, 127),
        (0, 0, 0, 128), (0, 0, 0, 56), (255, 0, 0, 128), (0, 255, 0, 128),
    ]
    masks = {
        128: [0, 1, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0],
        200: [0, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0],
    }
    source = Image.new("RGBA", (205, 137), "white")
    expected = {threshold: Image.new("RGBA", source.size, "white") for threshold in masks}
    for i, color in enumerate(colors):
        x, y = 6 + 48 * (i % 4), 6 + 30 * (i // 4)
        box = (x, y, x + 43, y + 23)
        ImageDraw.Draw(source).rectangle(box, fill=color)
        for threshold, mask in masks.items():
            ImageDraw.Draw(expected[threshold]).rectangle(box, fill="black" if mask[i] else "white")
    # Asymmetric edge marks expose clipping, rotations and odd-width padding.
    for image in [source, *expected.values()]:
        draw = ImageDraw.Draw(image)
        draw.line((0, 0, 0, 136), fill="black")
        for x in range(0, 205, 2):
            draw.point((x, 136), fill="black")
        draw.rectangle((201, 128, 204, 132), fill="black")
    return source, expected


def regenerate(binary):
    source, oracles = swatches()
    shipping_path = ROOT / "testdata/labels/amazon.png"
    shipping = Image.open(shipping_path).convert("RGBA")
    assert all(r == g == b and a == 255 for r, g, b, a in shipping.getdata())
    # The shipping reference is grayscale: its independent oracle is simply
    # gray < 128, with no color/alpha implementation shared with the encoder.
    shipping_oracle = shipping.convert("L").point(lambda gray: 0 if gray < 128 else 255).convert("RGBA")
    references = []
    with tempfile.TemporaryDirectory(prefix="labelize-reverse-") as tmp:
        stage = Path(tmp)
        source.save(stage / "rgba_input.png")
        for name, dpmm, threshold in CASES:
            input_path = shipping_path if name == "shipping" else stage / "rgba_input.png"
            oracle = shipping_oracle if name == "shipping" else oracles[threshold]
            for language in ("zpl", "tspl"):
                subprocess.run([
                    str(binary), "encode", str(input_path), "--language", language,
                    "--dpmm", str(dpmm), "--threshold", str(threshold),
                    "--output", str(stage / f"{name}.{language}"),
                ], check=True)
            # The API sizes its inch canvas using whole DPI (203 / 304).
            # Ask for half a dot beyond the desired size to avoid boundary
            # rounding, keep the response unmodified, and assert exact dims.
            dpi = int(dpmm * 25.4)
            w, h = oracle.size
            url = f"https://api.labelary.com/v1/printers/{dpmm}dpmm/labels/{(w + .5) / dpi:.9f}x{(h + .5) / dpi:.9f}/0/"
            zpl = (stage / f"{name}.zpl").read_bytes()
            request = urllib.request.Request(url, data=zpl, headers={
                "Accept": "image/png", "Content-Type": "application/x-www-form-urlencoded",
            })
            with urllib.request.urlopen(request, timeout=60) as response:
                png = response.read()
            reference = Image.open(io.BytesIO(png)).convert("RGBA")
            assert reference.size == oracle.size, (name, reference.size, oracle.size)
            assert reference.tobytes() == oracle.tobytes(), f"{name}: Labelary disagrees with the independent pixel oracle"
            (stage / f"{name}.png").write_bytes(png)
            references.append({
                "name": name, "dpmm": dpmm, "threshold": threshold,
                "source": "../labels/amazon.png" if name == "shipping" else "rgba_input.png",
                "url": url, "zpl_sha256": hashlib.sha256(zpl).hexdigest(),
                "reference_sha256": hashlib.sha256(png).hexdigest(),
            })
            print(f"{name}: Labelary matches every pixel at {w}x{h}", flush=True)
            time.sleep(.4)
        (stage / "references.json").write_text(json.dumps(references, indent=2) + "\n")
        for path in stage.iterdir():
            shutil.copyfile(path, DEST / path.name)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/labelize")
    regenerate(parser.parse_args().binary.resolve())
