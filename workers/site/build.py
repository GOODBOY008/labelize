"""Build the Cloudflare project's static pages and assets into workers/dist."""
from pathlib import Path
import shutil
import xml.etree.ElementTree as ET

from demo import load_demos
from docsite import write_docs
from home import render_home
from shell import PUBLIC_ORIGIN

SITE = Path(__file__).resolve().parent
ROOT = SITE.parents[1]
DIST = SITE.parent / "dist"


def build():
    if DIST.exists():
        shutil.rmtree(DIST)
    assets = DIST / "assets"
    shutil.copytree(SITE / "assets", assets)
    for name in ("logo-icon.svg", "social-preview.png"):
        shutil.copyfile(ROOT / "docs" / name, assets / name)

    demos = load_demos()
    (assets / "demo" / "order.zpl").write_text(demos["zpl"]["source"], encoding="ascii")
    for locale in ("en", "zh"):
        directory = DIST if locale == "en" else DIST / locale
        directory.mkdir(exist_ok=True)
        (directory / "index.html").write_text(render_home(locale, demos), encoding="utf-8")
    write_docs(DIST)

    pages = ["/" + file.relative_to(DIST).as_posix().removesuffix("index.html")
             for file in DIST.rglob("index.html")]
    sitemap = ET.Element("urlset", xmlns="http://www.sitemaps.org/schemas/sitemap/0.9")
    for path in sorted([*pages, "/playground/"]):
        ET.SubElement(ET.SubElement(sitemap, "url"), "loc").text = PUBLIC_ORIGIN + path
    ET.ElementTree(sitemap).write(DIST / "sitemap.xml", encoding="utf-8", xml_declaration=True)
    print(f"Built {len(pages)} pages in {DIST}")


if __name__ == "__main__":
    build()
