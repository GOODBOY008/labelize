"""Generate public, individually addressable guides from the playground's Markdown."""
import html
import json
import re
import sys
from pathlib import Path

from links import playground_url
from shell import page

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from tools.docs.build import compile_pages

GROUPS = {
    'Start here': '快速开始',
    'Integrate': '项目集成',
    'Reference': '技术参考',
    'Contribute': '参与贡献',
}

# Short navigation labels. The guide heading and body always come from Markdown.
GUIDES = {
    'getting-started': ('Your first label', '第一张标签'),
    'playground': ('Playground', '在线 Playground'),
    'languages': ('Languages and examples', '打印语言与示例'),
    'troubleshooting': ('Troubleshooting', '故障排查'),
    'http': ('HTTP service', 'HTTP 服务'),
    'api': ('Public HTTP API', '公共 HTTP API'),
    'cli': ('Command line', '命令行'),
    'docker': ('Docker', 'Docker 部署'),
    'rust': ('Rust library', 'Rust 库'),
    'print-templates': ('Print templates', '打印模板'),
    'javascript': ('JavaScript / WebAssembly', 'JavaScript / WebAssembly'),
    'android': ('Android', 'Android 集成'),
    'usage': ('Usage guide', '使用指南'),
    'vector': ('Vector output', '矢量输出'),
    'zpl': ('ZPL commands', 'ZPL 指令'),
    'datamatrix': ('Data Matrix', 'Data Matrix 条码'),
    'rendering': ('Rendering notes', '渲染说明'),
    'tspl-baselines': ('TSPL baselines', 'TSPL 参考基准'),
    'architecture': ('Architecture', '项目架构'),
    'testing': ('Testing', '测试指南'),
    'corpus': ('Test corpus', '测试样本'),
    'thresholds': ('Diff thresholds', '差异阈值'),
    'publishing': ('Docker publishing', 'Docker 镜像发布'),
    'index': ('Documentation index', '文档索引'),
    'tutorials': ('Integration tutorials', '集成教程'),
}
DOC_HEAD = ('<link rel="stylesheet" href="/assets/docs.css">'
            '<script src="/assets/docs.js" defer></script>')


def text(locale, en, zh):
    return zh if locale == 'zh' else en


def docs_path(locale):
    return '/zh/docs/' if locale == 'zh' else '/docs/'


def label(guide, locale):
    return text(locale, *GUIDES.get(guide['id'], (guide['title'], guide['title'])))


def search_box(locale):
    heading = text(locale, 'Search documentation', '搜索文档')
    return f'''<div class="doc-search" data-locale="{locale}" hidden>
      <label for="guide-query">{heading}</label>
      <div class="doc-search-input"><svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">
        <circle cx="10" cy="10" r="6.5"/><path d="m15 15 5 5"/></svg>
        <input id="guide-query" type="search" autocomplete="off" placeholder="{heading}"></div>
      <div class="doc-search-results" hidden>
        <p class="doc-search-status" role="status"></p>
        <ul class="doc-search-list"></ul>
      </div>
    </div>'''


def group_list(guides, locale, current=None):
    sections = []
    for group, zh_name in GROUPS.items():
        items = []
        for guide in guides:
            if guide['group'] != group:
                continue
            active = ' aria-current="page"' if current == guide['id'] else ''
            items.append(f'<li><a href="{docs_path(locale)}{guide["id"]}/"{active}>'
                         f'{html.escape(label(guide, locale))}</a></li>')
        sections.append(f'<section><h2>{text(locale, group, zh_name)}</h2>'
                        f'<ul>{"".join(items)}</ul></section>')
    return ''.join(sections)


def landing(guides, locale):
    root = docs_path(locale)
    title = text(locale, 'Documentation', '文档中心')
    desc = text(locale, 'Render your first label, integrate the engine into your application, or look up a printer command.',
                '从渲染第一张标签开始，了解如何将引擎接入应用，或直接查阅打印指令。')
    action = text(locale, 'Render your first label', '渲染第一张标签')
    caption = text(locale, 'Order label', '订单标签')
    language_note = '<p class="doc-language-note">技术文档正文为英文。</p>' if locale == 'zh' else ''
    body = f'''<div class="site-width doc-landing">
      <div class="doc-intro"><div class="doc-intro-text"><h1>{title}</h1>
        <p>{desc}</p>{search_box(locale)}</div>
        <a class="doc-first-guide" href="{root}getting-started/">
          <div class="doc-sheet"><img width="800" height="400" src="/assets/demo/zpl.png" alt="{caption}"></div>
          <span>{action}</span></a>
      </div>
      {language_note}
      <nav class="doc-directory" aria-label="{title}">{group_list(guides, locale)}</nav>
    </div>'''
    return page(locale, title + ' | Labelize', desc, root, body, section='docs', extra_head=DOC_HEAD)


def example_link(match, locale):
    language, width, height, dpmm, source = match.groups()
    url = playground_url(source, language, float(width), float(height), int(dpmm))
    cta = text(locale, 'Try in Playground', '在 Playground 中运行')
    return (f'<div class="doc-example"><pre><code class="language-{language}">{html.escape(source)}</code></pre>'
            f'<a class="doc-try" lang="{locale}" href="{html.escape(url, quote=True)}">{cta}</a></div>')


def description(guide):
    for para in re.findall(r'<p>(.*?)</p>', guide['html'], re.S):
        plain = ' '.join(html.unescape(re.sub('<[^>]+>', '', para)).split())
        if len(plain) > 38:
            return plain if len(plain) <= 160 else plain[:157].rsplit(' ', 1)[0] + '…'
    return guide['title'] + ' · Labelize documentation'


def document(guide, guides, locale):
    root = docs_path(locale)
    path = root + guide['id'] + '/'
    docs = text(locale, 'Documentation', '文档中心')
    title = guide['title']
    toc_title = text(locale, 'On this page', '本页目录')
    toc_items = ''.join(f'<li><a href="#{html.escape(anchor, quote=True)}">{heading}</a></li>'
                        for level, anchor, heading in guide['headings'] if level == '2')
    toc = ''
    if toc_items:
        toc = (f'<nav class="doc-toc" aria-label="{toc_title}"><details><summary>{toc_title}</summary>'
               f'<ul lang="en">{toc_items}</ul></details></nav>')
    content = guide['html'].replace('<table>', '<div class="doc-table" tabindex="0"><table>').replace('</table>', '</table></div>')
    # Long tables and code samples scroll independently of the reading column.
    source_title = text(locale, 'View source on GitHub', '在 GitHub 查看源文件')
    breadcrumb = text(locale, 'Breadcrumbs', '当前位置')
    body = f'''<div class="site-width doc-guide">
      <div class="doc-topbar"><nav class="doc-crumbs" aria-label="{breadcrumb}"><ol>
        <li><a href="{root}">{docs}</a></li>
        <li><a href="{path}" aria-current="page">{html.escape(label(guide, locale))}</a></li>
      </ol></nav>{search_box(locale)}</div>
      <div class="doc-layout">
        <nav class="doc-sidenav" aria-label="{text(locale, 'All guides', '全部指南')}">
          {group_list(guides, locale, guide['id'])}
        </nav>
        <div class="doc-reading"><article lang="en">{content}</article>
          <p class="doc-source"><a href="https://github.com/GOODBOY008/labelize/blob/main/docs/{guide['file']}">{source_title}</a></p>
        </div>{toc}
      </div>
      <p class="doc-copy-status" role="status"></p>
    </div>'''
    return page(locale, title + ' | Labelize Docs', description(guide), path, body, section='docs', extra_head=DOC_HEAD)


def write_html(out_dir, path, markup):
    destination = out_dir / path.strip('/') / 'index.html'
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(markup, encoding='utf-8')


def write_docs(out_dir):
    out_dir = Path(out_dir)
    for locale in ('en', 'zh'):
        root = docs_path(locale)
        def doc_url(slug, anchor):
            return root + slug + '/' + ('#' + anchor if anchor else '')

        guides = compile_pages(
            doc_url=doc_url, render_example=lambda match: example_link(match, locale),
            app_links={'#api': '/playground/#api', '#playground': '/playground/',
                       '/playground/': '/playground/'},
        )
        write_html(out_dir, root, landing(guides, locale))
        for guide in guides:
            write_html(out_dir, doc_url(guide['id'], ''), document(guide, guides, locale))
        if locale == 'en':
            index = [dict(id=g['id'], title=g['title'], group=g['group'], groupZh=GROUPS[g['group']],
                          en=label(g, 'en'), zh=label(g, 'zh'), text=' '.join(g['search'].split()))
                     for g in guides]
            asset = out_dir / 'assets' / 'docs-search.json'
            asset.parent.mkdir(parents=True, exist_ok=True)
            asset.write_text(json.dumps(index, ensure_ascii=False, separators=(',', ':')), encoding='utf-8')
