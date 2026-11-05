#!/usr/bin/env python3
"""Compile canonical Markdown into the offline playground bundle; --check in CI."""
import argparse
import html
import json
import re
from pathlib import Path
from urllib.parse import unquote, urlsplit

import markdown

ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / 'docs'
OUT = ROOT / 'src/playground/docs.html'
PAGES = [
    ('Start here', 'GETTING_STARTED.md', 'getting-started'),
    ('Start here', 'PLAYGROUND.md', 'playground'),
    ('Start here', 'LANGUAGES.md', 'languages'),
    ('Start here', 'TROUBLESHOOTING.md', 'troubleshooting'),
    ('Integrate', 'tutorials/http-service.md', 'http'),
    ('Integrate', 'API.md', 'api'),
    ('Integrate', 'tutorials/cli.md', 'cli'),
    ('Integrate', 'tutorials/docker.md', 'docker'),
    ('Integrate', 'tutorials/rust-library.md', 'rust'),
    ('Integrate', 'tutorials/javascript-wasm.md', 'javascript'),
    ('Integrate', 'tutorials/android.md', 'android'),
    ('Reference', 'USAGE.md', 'usage'),
    ('Reference', 'VECTOR_OUTPUT.md', 'vector'),
    ('Reference', 'ZPL_COMMANDS_REFERENCE.md', 'zpl'),
    ('Reference', 'DATAMATRIX.md', 'datamatrix'),
    ('Reference', 'RENDERING_NOTES.md', 'rendering'),
    ('Reference', 'TSPL_BASELINES.md', 'tspl-baselines'),
    ('Contribute', 'ARCHITECTURE.md', 'architecture'),
    ('Contribute', 'TESTING.md', 'testing'),
    ('Contribute', 'TEST_CORPUS.md', 'corpus'),
    ('Contribute', 'DIFF_THRESHOLDS.md', 'thresholds'),
    ('Contribute', 'DOCKER_PUBLISHING.md', 'publishing'),
    ('Contribute', 'README.md', 'index'),
    ('Integrate', 'tutorials/README.md', 'tutorials'),
]
ROUTES = {(DOCS / path).resolve(): slug for _, path, slug in PAGES}


def example(match):
    language, width, height, dpmm, source = match.groups()
    return ('<div class="doc-example"><pre><code class="language-' + language + '">' +
            html.escape(source) + '</code></pre><button type="button" class="doc-try" '
            f'data-language="{language}" data-width="{width}" data-height="{height}" '
            f'data-dpmm="{dpmm}">Try in playground</button></div>')


def build():
    pages = []
    for group, file, slug in PAGES:
        source = (DOCS / file).read_text()
        pattern = r'^```(zpl|epl|tspl|cpcl|dpl) playground width=([\d.]+) height=([\d.]+) dpmm=(6|8|12|24)\n(.*?)^```'
        examples = [dict(language=lang, width=float(w), height=float(h), dpmm=int(d), source=code)
                    for lang, w, h, d, code in re.findall(pattern, source, re.M | re.S)]
        source = re.sub(pattern, example, source, flags=re.M | re.S)
        md = markdown.Markdown(extensions=['fenced_code', 'tables', 'toc', 'sane_lists'],
                               extension_configs={'toc': {'permalink': False}})
        body = md.convert(source)
        title = re.search(r'<h1[^>]*>(.*?)</h1>', body).group(1)
        headings = [(level, anchor, re.sub('<[^>]+>', '', text)) for level, anchor, text in
                    re.findall(r'<h([23]) id="([^"]+)">(.*?)</h[23]>', body)]
        def link(match):
            attr, target = match.groups()
            # Application navigation and the shared schema are runtime routes,
            # rather than Markdown files or section anchors.
            if attr == 'href' and target in ('#api', '#playground', '/openapi.json'):
                return match.group(0)
            url = urlsplit(html.unescape(target))
            if url.scheme or url.netloc:
                return match.group(0)
            path = ((DOCS / file).parent / unquote(url.path)).resolve() if url.path else (DOCS / file).resolve()
            if not path.exists():
                raise ValueError(f'{file}: missing link target {target}')
            if attr == 'href' and path in ROUTES:
                dest = '#docs/' + ROUTES[path] + ('/' + url.fragment if url.fragment else '')
            else:
                repo_path = path.relative_to(ROOT).as_posix()
                dest = 'https://github.com/GOODBOY008/labelize/' + ('raw/' if attr == 'src' else 'blob/') + 'main/' + repo_path
                if url.fragment:
                    dest += '#' + url.fragment
            return f'{attr}="{html.escape(dest, quote=True)}"'
        body = re.sub(r'(href|src)="([^"]+)"', link, body)
        pages.append(dict(id=slug, group=group, title=html.unescape(re.sub('<[^>]+>', '', title)),
                          file=file, html=body, headings=headings, examples=examples,
                          search=html.unescape(re.sub('<[^>]+>', ' ', body))))
    # Every internal deep link must resolve to a real page and heading.
    by_id = {p['id']: p for p in pages}
    for page in pages:
        for slug, anchor in re.findall(r'href="#docs/([^"/]+)(?:/([^"]+))?"', page['html']):
            if slug not in by_id or (anchor and f'id="{anchor}"' not in by_id[slug]['html']):
                raise ValueError(f'{page["file"]}: broken docs link {slug}/{anchor}')
    data = json.dumps(pages, ensure_ascii=False, separators=(',', ':')).replace('<', '\\u003c').replace('&', '\\u0026')
    return '<!-- Generated by tools/docs/build.py; edit docs/*.md instead. -->\n<script id="docs-data" type="application/json">' + data + '</script>\n'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = build()
    if args.check:
        if not OUT.exists() or OUT.read_text() != result:
            raise SystemExit('Documentation bundle is stale. Run python tools/docs/build.py and commit src/playground/docs.html.')
    else:
        OUT.write_text(result)
    print(f'Documentation: {len(PAGES)} pages and internal links verified')
