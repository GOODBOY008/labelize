"""Shared chrome for the Cloudflare project website."""
from html import escape

PUBLIC_ORIGIN = "https://labelize.764629910.workers.dev"
REPOSITORY = "https://github.com/GOODBOY008/labelize"

# Small interface icons from Lucide (ISC), drawn at the same stroke as the rules.
# Keep the paths local so navigating and switching languages works offline.
ICONS = {
    "menu": '<path d="M3 6h18M3 12h18M3 18h18"/>',
    "globe": '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c5 5 5 13 0 18-5-5-5-13 0-18Z"/>',
    "github": '<path d="M9 19c-4.3 1.3-4.3-2.1-6-2.5M15 22v-3.4a3 3 0 0 0-.8-2.3c2.7-.3 5.6-1.3 5.6-6A4.6 4.6 0 0 0 18.5 7a4.3 4.3 0 0 0-.1-3.3s-1.1-.3-3.6 1.2a12.2 12.2 0 0 0-6.5 0C5.8 3.4 4.7 3.7 4.7 3.7A4.3 4.3 0 0 0 4.6 7a4.6 4.6 0 0 0-1.3 3.3c0 4.7 2.9 5.7 5.6 6A3 3 0 0 0 8 18.5V22"/>',
    "code": '<path d="m8 7-5 5 5 5m8-10 5 5-5 5m-3-14-2 18"/>',
    "terminal": '<rect x="2" y="4" width="20" height="16" rx="2"/><path d="m6 9 3 3-3 3m6 0h5"/>',
    "api": '<path d="M7 3v4m10 10v4M7 12v5h10V9"/><circle cx="7" cy="9" r="2.5"/><circle cx="17" cy="6" r="2.5"/>',
    "play": '<path d="m8 5 10 7-10 7Z"/>',
    "copy": '<rect x="8" y="8" width="12" height="13" rx="2"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h3"/>',
    "check": '<path d="m4 12 5 5L20 6"/>',
    "file": '<path d="M5 3h9l5 5v13H5zM14 3v6h5M8 13h8m-8 4h6"/>',
    "download": '<path d="M12 3v11m-4-4 4 4 4-4M4 16v4h16v-4"/>',
}


def icon(name):
    return (f'<svg class="site-icon" aria-hidden="true" focusable="false" viewBox="0 0 24 24" '
            f'fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" '
            f'stroke-linejoin="round">{ICONS[name]}</svg>')


def site_path(locale, path):
    return ("/zh" if locale == "zh" else "") + path


def page(locale, title, description, path, body, *, section="home", extra_head=""):
    zh = locale == "zh"
    language = "zh-CN" if zh else "en"
    other_locale = "en" if zh else "zh"
    english_path = path.removeprefix("/zh") if zh else path
    alternate = site_path(other_locale, english_path)
    docs = site_path(locale, "/docs/")
    home = site_path(locale, "/")
    current = ' aria-current="page"' if section == "docs" else ""
    brand = f'<img src="/assets/logo-icon.svg" width="34" height="31" alt=""><span>Labelize</span>'
    description = escape(description, quote=True)
    title = escape(title)
    legacy = '<script src="/assets/legacy.js"></script>' if path == "/" else ""
    return f'''<!doctype html>
<html lang="{language}"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
{legacy}
<title>{title}</title>
<meta name="description" content="{description}"><meta name="theme-color" content="#f9faf8">
<link rel="canonical" href="{PUBLIC_ORIGIN}{path}">
<link rel="alternate" hreflang="en" href="{PUBLIC_ORIGIN}{english_path}">
<link rel="alternate" hreflang="zh-CN" href="{PUBLIC_ORIGIN}{site_path("zh", english_path)}">
<link rel="alternate" hreflang="x-default" href="{PUBLIC_ORIGIN}{english_path}">
<meta property="og:type" content="website"><meta property="og:site_name" content="Labelize">
<meta property="og:locale" content="{"zh_CN" if zh else "en_US"}">
<meta property="og:title" content="{title}"><meta property="og:description" content="{description}">
<meta property="og:url" content="{PUBLIC_ORIGIN}{path}"><meta property="og:image" content="{PUBLIC_ORIGIN}/assets/social-preview.png">
<meta name="twitter:card" content="summary_large_image">
<link rel="icon" href="/assets/logo-icon.svg" type="image/svg+xml">
<link rel="stylesheet" href="/assets/site.css"><script src="/assets/site.js" defer></script>{extra_head}
</head><body>
<a class="site-skip" href="#main-content">{"跳至正文" if zh else "Skip to content"}</a>
<header class="site-header"><div class="site-width site-bar">
<a class="site-brand" href="{home}" aria-label="Labelize · {"首页" if zh else "Home"}">{brand}</a>
<button class="site-nav-toggle" type="button" aria-controls="site-links" aria-expanded="false" hidden>{icon("menu")}<span>{"菜单" if zh else "Menu"}</span></button>
<nav class="site-nav" id="site-links" aria-label="{"主导航" if zh else "Main navigation"}">
<a href="{docs}"{current}>{"文档" if zh else "Docs"}</a><a href="/playground/">Playground</a><a href="/playground/#api">{"API 控制台" if zh else "API Console"}</a>
<a class="site-repository" href="{REPOSITORY}">{icon("github")}GitHub</a>
<a class="site-locale" data-site-locale="{other_locale}" href="{alternate}" lang="{other_locale}" hreflang="{other_locale}">{icon("globe")}{"English" if zh else "中文"}</a>
</nav></div></header>
<main id="main-content" tabindex="-1">{body}</main>
<footer class="site-footer"><div class="site-width footer-inner">
<a class="site-brand" href="{home}">{brand}</a>
<nav aria-label="{"页脚" if zh else "Footer"}"><a href="{docs}">{"文档" if zh else "Docs"}</a><a href="{REPOSITORY}/releases">{"版本发布" if zh else "Releases"}</a><a href="{REPOSITORY}/discussions">{"社区讨论" if zh else "Discussions"}</a><a href="{REPOSITORY}/blob/main/LICENSE">{"许可证" if zh else "License"}</a></nav>
</div></footer></body></html>'''
