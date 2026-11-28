"""Render the two project homepages using the same verified printer examples."""
from html import escape
import json

from shell import icon, page, site_path

STUDIO = "https://github.com/GOODBOY008/labelize-studio"
CLI = """cargo install labelize --features cli

labelize convert order.zpl --width 100 --height 50 --dpmm 8"""
HTTP = """docker run -d --rm -p 8080:8080 goodboy008/labelize

curl 'http://localhost:8080/convert?width=100&height=50&dpmm=8' \\
  -H 'Content-Type: application/zpl' \\
  --data-binary @order.zpl -o order.png"""
RUST = """use labelize::{render_png, RenderOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let job = std::fs::read("order.zpl")?;
    let options = RenderOptions {
        width_mm: Some(100.0), height_mm: Some(50.0), dpmm: 8,
        ..Default::default()
    };
    std::fs::write("order.png", render_png(&job, options)?.bytes)?;
    Ok(())
}"""


def excerpt(source):
    lines = source.replace("\x02", "␂").splitlines()
    first = next((i for i, line in enumerate(lines) if "Order" in line), 0)
    return "\n".join(lines[first:first + 3])


def demo_stage(locale, demos):
    zh = locale == "zh"
    initial = demos["zpl"]
    buttons = "".join(
        f'<button type="button" data-language="{key}" aria-pressed="{str(key == "zpl").lower()}">{key.upper()}</button>'
        for key in demos)
    payload = {key: {**demo, "source": excerpt(demo["source"])} for key, demo in demos.items()}
    serialized = json.dumps(payload, separators=(",", ":")).replace("<", "\\u003c")
    return f'''<figure class="render-demo" aria-label="{"五种打印机语言的渲染示例" if zh else "Render example in five printer languages"}">
<div class="demo-toolbar"><span class="demo-heading"><i class="registration" aria-hidden="true"></i>{"标签预览" if zh else "Label preview"}</span><span class="output-type">PNG</span></div>
<div class="print-bed">
<div class="print-sheet"><img id="demo-image" src="{initial["image"]}" width="800" height="400" alt="{"订单 000123 的热敏标签，包含商品名 Thermal labels、Code 128 条码和二维码" if zh else "Thermal label for order 000123 with the product Thermal labels, a Code 128 barcode and a QR code"}"></div>
<span class="paper-size">100 × 50 mm</span>
<div class="source-slip"><span class="source-file">{icon("file")}<span id="demo-file">order.zpl</span></span>
<pre tabindex="0" aria-label="{"源码片段" if zh else "Source excerpt"}"><code id="demo-source">{escape(excerpt(initial["source"]))}</code></pre>
</div></div>
<div class="demo-controls"><div class="language-options" role="group" aria-label="{"打印机语言" if zh else "Printer language"}" hidden>{buttons}</div>
<a id="demo-open" href="{escape(initial["url"], quote=True)}">{icon("play")}{"打开此示例" if zh else "Open example"}</a></div>
<script type="application/json" id="demo-content">{serialized}</script>
</figure>'''


def integrations(locale):
    zh = locale == "zh"
    items = [
        ("cli", "terminal", "命令行" if zh else "Command line", "在终端中转换文件。" if zh else "Convert files in your terminal.",
         "CLI 教程" if zh else "CLI guide", "shell", CLI),
        ("http", "api", "HTTP API", "为应用部署渲染服务。" if zh else "Self-host a rendering service.",
         "HTTP 教程" if zh else "HTTP guide", "shell", HTTP),
        ("rust", "code", "Rust SDK", "将引擎嵌入 Rust 应用。" if zh else "Embed the engine in a Rust app.",
         "Rust SDK 教程" if zh else "Rust SDK guide", "src/main.rs", RUST)]
    choices = []
    panels = []
    for slug, glyph, title, description, guide, filename, code in items:
        choices.append(f'''<a class="integration-choice" data-choice="{slug}" href="#example-{slug}">
<span class="choice-icon">{icon(glyph)}</span><span><strong>{title}</strong><small>{description}</small></span>
</a>''')
        rust_setup = ('<p class="rust-setup">' +
                      ('先运行' if zh else 'First run') +
                      ' <code>cargo add labelize</code></p>') if slug == "rust" else ""
        panels.append(f'''<div class="example-panel" id="example-{slug}" data-panel="{slug}">
<div class="example-toolbar"><span>{icon(glyph)} {filename}</span><button class="copy-example" type="button" hidden>{icon("copy")}<span>{"复制" if zh else "Copy"}</span></button></div>
{rust_setup}<pre tabindex="0"><code>{escape(code)}</code></pre>
<div class="example-links"><a href="/assets/demo/order.zpl" download="order.zpl">{icon("download")}order.zpl</a><a href="{site_path(locale, f"/docs/{slug}/")}">{guide}</a></div></div>''')
    return f'''<section class="integrations site-section" id="integrate" aria-labelledby="integrate-heading"><div class="site-width">
<div class="section-heading"><h2 id="integrate-heading">{"用你熟悉的方式接入。" if zh else "Use it in your stack."}</h2>
<p>{"下载示例标签，通过命令行、服务或 SDK 生成自己的第一张 PNG。" if zh else "Download the sample label and render your first PNG from the terminal, a service or your application."}</p></div>
<div class="integration-grid"><div class="integration-choices" aria-label="{"接入方式" if zh else "Integration options"}">{"".join(choices)}</div>
<div class="example-panels">{"".join(panels)}</div></div></div></section>'''


def studio(locale, image):
    zh = locale == "zh"
    return f'''<section class="studio-section site-section"><div class="site-width studio-layout">
<div class="studio-copy"><h2>{"在 Studio 中设计标签。" if zh else "Design labels in Studio."}</h2>
<p>{"使用 Labelize Studio 桌面设计器摆放字段、条码和图形，导出打印模板。应用填入每笔订单的数据，即可用同一份打印指令生成预览或送往打印机。" if zh else "Lay out fields, barcodes and artwork in the Labelize Studio desktop designer. Export a template, fill it with each order and use the same printer job for preview and print."}</p>
<div class="studio-links"><a class="site-button secondary" href="{STUDIO}">{"了解 Studio" if zh else "Explore Studio"}</a><a class="text-link" href="{site_path(locale, "/docs/print-templates/")}">{"模板集成指南" if zh else "Template guide"}</a></div></div>
<div class="template-flow"><div class="order-values">
<span class="flow-filename">{icon("code")}order.json</span>
<dl><dt>order_no</dt><dd>000123</dd><dt>product</dt><dd>Thermal labels</dd></dl></div>
<div class="template-paper"><img src="{image}" width="800" height="400" loading="lazy" alt="{"填入订单数据后渲染的标签" if zh else "Label rendered with these order values"}"></div>
</div></div></section>'''


def render_home(locale, demos):
    zh = locale == "zh"
    tagline = "把打印指令变成看得见的标签。" if zh else "Turn printer code into a label you can see."
    title = f"Labelize — {tagline}"
    description = ("Labelize 是一款可离线运行、自托管的开源 Rust 引擎，将 ZPL、EPL、TSPL、CPCL 和 DPL 标签渲染为 PNG 或 PDF。"
                   if zh else "Labelize is an open-source Rust engine that renders ZPL, EPL, TSPL, CPCL and DPL labels as PNG or PDF. Run it offline, embed it or self-host it.")
    headline = ('<span>把打印指令变成</span> <span class="accent">看得见的标签。</span>' if zh else
                '<span>Turn printer code</span> <span>into a label</span> <span class="accent">you can see.</span>')
    intro = ("把五种打印机语言渲染为 PNG 或 PDF。Labelize 是一款开源 Rust 引擎，可离线运行、嵌入应用或自托管。"
             if zh else "Render five printer languages to PNG or PDF with an open-source Rust engine. Run it offline, embed it in your app or host it yourself.")
    hero = f'''<section class="hero site-width" aria-labelledby="hero-title">
<div class="hero-copy"><h1 id="hero-title">{headline}</h1><p>{intro}</p>
<div class="hero-actions"><a class="site-button primary" href="{site_path(locale, "/docs/getting-started/")}">{"快速开始" if zh else "Get started"}</a>
<a class="site-button secondary" href="/playground/">{icon("play")}{"打开 Playground" if zh else "Open Playground"}</a></div></div>
{demo_stage(locale, demos)}</section>'''
    return page(locale, title, description, site_path(locale, "/"),
                hero + integrations(locale) + studio(locale, demos["zpl"]["image"]),
                extra_head='<script src="/assets/home.js" defer></script>')
