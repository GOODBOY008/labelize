#!/usr/bin/env python3
"""Convert one directory sequentially using the public or a local Labelize API."""

import argparse
import math
import os
import sys
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode, urlsplit
from urllib.request import Request, urlopen


def positive(raw):
    value = float(raw)
    if not math.isfinite(value) or value <= 0:
        raise argparse.ArgumentTypeError("expected a positive finite number")
    return value


def resolution(raw):
    value = int(raw)
    if not 1 <= value <= 2147483647:
        raise argparse.ArgumentTypeError("dpmm must be a positive 32-bit integer")
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="directory containing label files")
    parser.add_argument("output", type=Path, help="directory for converted files")
    parser.add_argument("--base-url", default=os.environ.get("LABELIZE_BASE_URL", "https://labelize.764629910.workers.dev"))
    parser.add_argument("--width", type=positive, default=100, help="canvas width in mm (default: 100)")
    parser.add_argument("--height", type=positive, default=60, help="canvas height in mm (default: 60)")
    parser.add_argument("--dpmm", type=resolution, default=8)
    parser.add_argument("--format", choices=["png", "pdf"], default="png")
    parser.add_argument("--timeout", type=positive, default=30, help="socket timeout in seconds (default: 30)")
    args = parser.parse_args()
    base = urlsplit(args.base_url)
    if base.scheme not in ("http", "https") or not base.netloc or base.query or base.fragment:
        parser.error("--base-url must be an HTTP(S) URL without a query or fragment")
    if not args.input.is_dir():
        parser.error("input must be a directory")
    languages = {".zpl": "zpl", ".epl": "epl", ".tspl": "tspl", ".cpcl": "cpcl", ".dpl": "dpl"}
    files = sorted(path for path in args.input.iterdir() if path.is_file() and path.suffix.lower() in languages)
    if not files:
        parser.error("no .zpl, .epl, .tspl, .cpcl or .dpl files found")
    args.output.mkdir(parents=True, exist_ok=True)
    query = urlencode({"width": args.width, "height": args.height, "dpmm": args.dpmm, "output": args.format})
    url = args.base_url.rstrip("/") + "/convert?" + query
    failures = 0
    for path in files:
        try:
            request = Request(url, data=path.read_bytes(), headers={"Content-Type": "application/" + languages[path.suffix.lower()]})
            with urlopen(request, timeout=args.timeout) as response:
                result = response.read()
            # Keep the source extension so order.zpl and order.epl cannot collide.
            destination = args.output / (path.name + "." + args.format)
            destination.write_bytes(result)
            print(f"{path.name} -> {destination}")
        except HTTPError as error:
            failures += 1
            print(f"{path.name}: HTTP {error.code}: {error.read().decode('utf-8', 'replace')}", file=sys.stderr)
        except (OSError, URLError) as error:
            failures += 1
            print(f"{path.name}: {error}", file=sys.stderr)
    print(f"Converted {len(files) - failures}/{len(files)} files; {failures} failed.")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
