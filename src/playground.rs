/// Self-contained HTML playground page served at `GET /`.
/// All HTML/CSS/JS is inlined — no external dependencies.
///
/// Features:
/// - Light/dark theme: `prefers-color-scheme` auto-detection, header toggle,
///   `localStorage` persistence, applied pre-paint to avoid a flash
/// - i18n (English / 简体中文): `navigator.language` auto-detection, header
///   selector, `localStorage` persistence; static text via `data-i18n*`
///   attributes, dynamic strings via `t()`/`fmt()`
/// - Live auto-render (debounced, opt-out) that keeps the last good preview
/// - Share permalink: label code + settings encoded in the URL hash
/// - Built-in sample labels, preview zoom (fit/percent), copy PNG to
///   clipboard, caret Ln/Col indicator, Ctrl+S = download PNG
/// - Inch presets and custom dimensions in inches or mm; dimensions stay in
///   mm for POST /convert and convert to inches for Labelary comparison.
///   Render always produces PNG; PDF is lazy.
pub const PLAYGROUND_HTML: &str = r##"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Free ZPL Viewer — ZPL, EPL, TSPL, CPCL &amp; DPL to PNG | Labelize</title>
<meta name="description" content="Render ZPL, EPL, TSPL, CPCL and DPL label code to PNG or PDF right in your browser. Free online thermal-label preview with syntax validation, formatting and an optional Labelary diff — open source, no sign-up.">
<meta name="robots" content="index,follow">
<link rel="canonical" href="https://labelize.764629910.workers.dev/">
<meta property="og:type" content="website">
<meta property="og:site_name" content="Labelize">
<meta property="og:title" content="Free ZPL Viewer — ZPL, EPL, TSPL, CPCL &amp; DPL to PNG | Labelize">
<meta property="og:description" content="Render ZPL, EPL, TSPL, CPCL and DPL label code to PNG or PDF in your browser — preview, validate and format thermal-label data for free.">
<meta property="og:url" content="https://labelize.764629910.workers.dev/">
<meta property="og:image" content="https://raw.githubusercontent.com/GOODBOY008/labelize/main/docs/social-preview.png">
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="Free ZPL Viewer — ZPL, EPL, TSPL, CPCL &amp; DPL to PNG | Labelize">
<meta name="twitter:description" content="Render ZPL, EPL, TSPL, CPCL and DPL label code to PNG or PDF in your browser — preview, validate and format thermal-label data for free.">
<meta name="twitter:image" content="https://raw.githubusercontent.com/GOODBOY008/labelize/main/docs/social-preview.png">
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@graph": [
    {
      "@type": "WebApplication",
      "name": "Labelize",
      "url": "https://labelize.764629910.workers.dev/",
      "applicationCategory": "DeveloperApplication",
      "operatingSystem": "Any",
      "isAccessibleForFree": true,
      "description": "Free online ZPL, EPL, TSPL, CPCL and DPL label viewer. Render thermal-label code to PNG or PDF, validate syntax and format label data.",
      "featureList": [
        "ZPL (Zebra) viewer and renderer",
        "EPL (Eltron) label preview",
        "TSPL/TSC label preview",
        "CPCL (Zebra mobile) label preview",
        "DPL (Datamax-O'Neil) label preview",
        "Render label code to PNG or PDF",
        "Syntax validation and one-command-per-line formatting",
        "Pixel diff against the Labelary reference renderer"
      ],
      "license": "https://github.com/GOODBOY008/labelize/blob/main/LICENSE",
      "author": { "@type": "Organization", "name": "GOODBOY008", "url": "https://github.com/GOODBOY008" }
    },
    {
      "@type": "FAQPage",
      "mainEntity": [
        {
          "@type": "Question",
          "name": "Is this ZPL viewer free?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. Labelize is free to use, MIT-licensed open source, and requires no sign-up." }
        },
        {
          "@type": "Question",
          "name": "Which label languages are supported?",
          "acceptedAnswer": { "@type": "Answer", "text": "ZPL (Zebra), EPL (Eltron), TSPL/TSC, CPCL (Zebra mobile) and DPL (Datamax-O'Neil). Labels render to PNG or PDF at selectable sizes and resolutions." }
        },
        {
          "@type": "Question",
          "name": "Is there an API?",
          "acceptedAnswer": { "@type": "Answer", "text": "Yes. The hosted service exposes POST /convert, /validate and /format endpoints, and the same engine ships as a CLI, Rust library, WebAssembly package and Docker image for self-hosting." }
        },
        {
          "@type": "Question",
          "name": "Does my label data leave my browser?",
          "acceptedAnswer": { "@type": "Answer", "text": "The hosted playground renders labels server-side and stores nothing. The optional Compare with Labelary feature sends your code to the external labelary.com service and can be disabled." }
        }
      ]
    }
  ]
}
</script>
<link rel="icon" href="data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%22374%20246%20520%20471%22%3E%3Cdefs%3E%3ClinearGradient%20id%3D%22lzA%22%20x1%3D%220.08%22%20y1%3D%220.02%22%20x2%3D%220.95%22%20y2%3D%220.98%22%3E%3Cstop%20offset%3D%220%25%22%20stop-color%3D%22%2310a7f5%22%2F%3E%3Cstop%20offset%3D%2252%25%22%20stop-color%3D%22%23167ce9%22%2F%3E%3Cstop%20offset%3D%22100%25%22%20stop-color%3D%22%234734e9%22%2F%3E%3C%2FlinearGradient%3E%3ClinearGradient%20id%3D%22lzB%22%20x1%3D%220%22%20y1%3D%220%22%20x2%3D%221%22%20y2%3D%221%22%3E%3Cstop%20offset%3D%220%25%22%20stop-color%3D%22%236425f3%22%2F%3E%3Cstop%20offset%3D%22100%25%22%20stop-color%3D%22%237548ef%22%2F%3E%3C%2FlinearGradient%3E%3C%2Fdefs%3E%3Cpath%20d%3D%22M%20463%2C570%20463%2C577%20467%2C580%20641%2C603%20646%2C599%20648%2C591%20647%2C584%20643%2C582%20477%2C560%20469%2C560%20466%2C562%20Z%20M%20869%2C551%20861%2C546%20852%2C544%20833%2C543%20764%2C534%20744%2C537%20729%2C545%20709%2C565%20698%2C585%20670%2C686%20659%2C701%20649%2C708%20641%2C711%20655%2C713%20683%2C713%20695%2C711%20715%2C704%20741%2C689%20756%2C677%20870%2C563%20871%2C554%20Z%20M%20476%2C525%20477%2C532%20483%2C534%20672%2C558%20679%2C554%20682%2C545%20681%2C539%20675%2C536%20608%2C529%20487%2C513%20481%2C514%20478%2C517%20Z%20M%20855%2C282%20832%2C274%20549%2C250%20526%2C250%20517%2C252%20499%2C260%20488%2C268%20473%2C285%20464%2C303%20380%2C563%20378%2C573%20379%2C607%20387%2C631%20401%2C653%20424%2C673%20448%2C684%20620%2C709%20633%2C708%20645%2C702%20657%2C689%20661%2C680%20661%2C677%20469%2C649%20447%2C638%20435%2C626%20428%2C615%20421%2C592%20422%2C564%20491%2C329%20495%2C321%20503%2C313%20512%2C309%20529%2C309%20822%2C336%20833%2C340%20841%2C351%20842%2C359%20798%2C529%20838%2C534%20890%2C352%20890%2C333%20884%2C313%20875%2C299%20864%2C288%20Z%22%20fill%3D%22url%28%23lzA%29%22%20fill-rule%3D%22evenodd%22%2F%3E%3Cpath%20d%3D%22M%20869%2C551%20861%2C546%20852%2C544%20833%2C543%20764%2C534%20744%2C537%20729%2C545%20709%2C565%20698%2C585%20691%2C606%20689%2C618%20670%2C686%20659%2C701%20649%2C708%20641%2C711%20655%2C713%20683%2C713%20695%2C711%20715%2C704%20741%2C689%20756%2C677%20870%2C563%20871%2C554%20Z%22%20fill%3D%22url%28%23lzB%29%22%20fill-rule%3D%22evenodd%22%2F%3E%3Cpath%20d%3D%22M%20462%2C573%20465%2C579%20472%2C581%20487%2C582%20501%2C585%20516%2C586%20635%2C603%20643%2C602%20646%2C599%20648%2C592%20648%2C585%20644%2C582%20591%2C576%20478%2C560%20467%2C561%20465%2C563%20Z%20M%20476%2C524%20476%2C531%20481%2C534%20674%2C558%20679%2C555%20682%2C545%20682%2C540%20676%2C536%20593%2C527%20489%2C513%20481%2C514%20478%2C517%20Z%22%20fill%3D%22%236077aa%22%20fill-rule%3D%22evenodd%22%2F%3E%3Cpath%20d%3D%22M%20791%2C379%20763%2C377%20760%2C381%20750%2C425%20730%2C499%20731%2C503%20760%2C506%20763%2C503%20794%2C386%20794%2C381%20Z%20M%20742%2C375%20731%2C374%20728%2C377%20721%2C408%20697%2C497%20699%2C500%20709%2C501%20712%2C499%20743%2C379%20Z%20M%20709%2C372%20689%2C370%20686%2C373%20655%2C493%20658%2C496%20677%2C497%20680%2C493%20680%2C489%20685%2C474%20692%2C443%20710%2C377%20Z%20M%20667%2C368%20654%2C366%20649%2C368%20618%2C487%20619%2C491%20635%2C493%20638%2C490%20666%2C378%20668%2C374%20Z%20M%20630%2C364%20614%2C363%20612%2C365%20603%2C403%20581%2C483%20581%2C486%20584%2C488%20598%2C489%20601%2C485%20631%2C369%20Z%20M%20592%2C360%20571%2C359%20569%2C361%20560%2C399%20538%2C479%20540%2C483%20558%2C485%20563%2C483%20594%2C367%20594%2C361%20Z%20M%20549%2C356%20527%2C354%20523%2C358%20492%2C471%20492%2C476%20494%2C478%20517%2C480%20520%2C478%20546%2C377%20551%2C362%20551%2C358%20Z%22%20fill%3D%22%23111a39%22%20fill-rule%3D%22evenodd%22%2F%3E%3C%2Fsvg%3E">
<script>
/* Apply the saved (or system) theme before first paint to avoid a flash. */
(function () {
  var t = null;
  try { t = localStorage.getItem("labelize-theme"); } catch (e) {}
  if (t !== "light" && t !== "dark") {
    t = window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }
  document.documentElement.setAttribute("data-theme", t);
})();
</script>
<script>
/* Deployment config. The HTTP server substitutes a JSON object for the
   placeholder between the braces below (from LABELIZE_* env vars).
   Consumers that serve the page verbatim (npm `lz_playground_html`,
   workers) leave the placeholder, which parses as an empty config and
   keeps every feature enabled. */
window.__LABELIZE_CONFIG__ = {/*__CONFIG__*/};
</script>
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

  :root {
    color-scheme: dark;
    --bg: #101218;
    --surface: #191c24;
    --surface2: #222631;
    --editor: #161920;
    --canvas: #12151c;
    --border: #2b303c;
    --grid-dot: #2a303b;
    --accent: #739aff;
    --accent-hover: #8aabff;
    --accent-soft: #222e49;
    --text: #e8ebf2;
    --text-dim: #a0a8b8;
    --text-muted: #727d91;
    --error: #f28b91;
    --success: #75cba7;
    --overlay: rgba(22,25,32,0.88);
    --err-bg: #322229;
    --sel: rgba(115,154,255,0.23);
    --shadow: 0 12px 32px rgba(0,0,0,0.24), 0 1px 4px rgba(0,0,0,0.16);
    --verdict-ok: #75cba7;
    --verdict-warn: #e5c77a;
    --verdict-mod: #e8ad7b;
    --verdict-bad: #f28b91;
    --badge-fg: #151820;
    --radius: 12px;
    --font-mono: "SFMono-Regular",Consolas,"Liberation Mono",monospace;
    --font-ui: -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
  }

  :root[data-theme="light"] {
    color-scheme: light;
    --bg: #f6f7f9;
    --surface: #ffffff;
    --surface2: #f5f6f8;
    --editor: #ffffff;
    --canvas: #f2f4f7;
    --border: #e2e6ed;
    --grid-dot: #d7dde6;
    --accent: #3568d4;
    --accent-hover: #2856b7;
    --accent-soft: #edf2ff;
    --text: #202735;
    --text-dim: #626e81;
    --text-muted: #8792a4;
    --error: #c13f4c;
    --success: #20825d;
    --overlay: rgba(246,248,251,0.9);
    --err-bg: #fff1f2;
    --sel: rgba(53,104,212,0.16);
    --shadow: 0 12px 32px rgba(35,49,73,0.10), 0 1px 4px rgba(35,49,73,0.08);
    --verdict-ok: #c4ead7;
    --verdict-warn: #f7e5ac;
    --verdict-mod: #f8d4b5;
    --verdict-bad: #f8ccd0;
    --badge-fg: #263445;
  }

  html, body { height: 100%; }
  body {
    display: flex; flex-direction: column; overflow: hidden;
    background: var(--bg); color: var(--text);
    font: 14px/1.5 var(--font-ui); -webkit-font-smoothing: antialiased;
  }
  button, input, select, textarea { font: inherit; }
  button, a, input, select { -webkit-tap-highlight-color: transparent; }
  button, a, input, select { transition: background-color .15s, border-color .15s, color .15s; }
  :focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
  button:disabled { opacity: .45; cursor: not-allowed; }
  button svg, a svg { flex-shrink: 0; }
  ::selection { background: var(--sel); }
  * { scrollbar-width: thin; scrollbar-color: var(--border) transparent; }

  /* App navigation */
  .app-header {
    flex-shrink: 0; display: flex; align-items: center; gap: 24px;
    min-height: 68px; padding: 12px 32px;
    background: var(--surface); border-bottom: 1px solid var(--border);
  }
  .logo { display: flex; align-items: center; gap: 10px; }
  .logo-icon { display: block; width: 29px; height: 29px; }
  .logo-text { font-size: 19px; font-weight: 700; letter-spacing: -.5px; }
  .nav-label { border-left: 1px solid var(--border); padding-left: 24px; color: var(--text-dim); font-size: 13px; }
  .header-actions { display: flex; align-items: center; gap: 10px; margin-left: auto; }
  .header-link { color: var(--text-dim); text-decoration: none; padding: 7px 9px; font-size: 13px; }
  .header-link:hover { color: var(--text); }
  select {
    appearance: none; background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%23848c9b' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m7 10 5 5 5-5'/%3E%3C/svg%3E");
    background-repeat: no-repeat; background-position: right 10px center;
    background-color: var(--surface); color: var(--text); border: 1px solid var(--border);
    border-radius: 7px; height: 38px; padding: 0 34px 0 12px; font-size: 13px; cursor: pointer;
  }
  select:hover, input[type="number"]:hover { border-color: var(--text-muted); }
  .hdr-select { max-width: 190px; }
  #lang { background-color: transparent; border-color: transparent; }
  #theme-btn {
    display: inline-flex; align-items: center; justify-content: center;
    height: 36px; width: 36px; border: 1px solid var(--border); border-radius: 8px;
    background: var(--surface); color: var(--text-dim); cursor: pointer;
  }
  #theme-btn:hover { background: var(--surface2); color: var(--text); }
  #theme-btn svg { width: 17px; height: 17px; }
  #theme-btn .icon-sun { display: none; }
  :root[data-theme="dark"] #theme-btn .icon-sun { display: block; }
  :root[data-theme="dark"] #theme-btn .icon-moon { display: none; }

  /* Workspace */
  main {
    display: flex; flex-direction: column; flex: 1; min-height: 0;
    width: 100%; max-width: 1680px; margin: 0 auto; padding: 26px 32px 16px; gap: 20px;
  }
  .workspace-intro { flex-shrink: 0; display: flex; align-items: center; justify-content: space-between; gap: 20px; }
  .workspace-intro h1 { font-size: 26px; font-weight: 650; letter-spacing: -.8px; line-height: 1.3; }
  .workspace-intro p { color: var(--text-dim); font-size: 13px; margin-top: 5px; }
  .intro-actions { display: flex; align-items: center; gap: 10px; flex-shrink: 0; }
  .settings-bar {
    display: flex; align-items: flex-end; flex-wrap: wrap; gap: 14px 20px; flex-shrink: 0;
    padding: 17px 20px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius);
  }
  .setting { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .setting > label { color: var(--text-dim); font-size: 12px; font-weight: 500; }
  #fmt { min-width: 104px; }
  #size-preset { min-width: 154px; }
  #dpmm { min-width: 112px; }
  #custom-size { display: flex; align-items: flex-end; gap: 10px; }
  #custom-size .setting { width: 82px; }
  #custom-size .unit-setting { width: 80px; }
  #custom-size select { width: 100%; }
  input[type="number"] {
    min-width: 0; width: 100%; height: 38px; padding: 0 10px;
    background: var(--surface); color: var(--text); border: 1px solid var(--border); border-radius: 7px; font-size: 13px;
  }
  .size-sep { align-self: flex-end; height: 38px; display: flex; align-items: center; color: var(--text-muted); }
  .settings-toggles { display: flex; align-items: center; gap: 20px; height: 38px; margin-left: auto; }
  .toggle { display: inline-flex; align-items: center; gap: 8px; font-size: 12px; color: var(--text-dim); cursor: pointer; white-space: nowrap; }
  .toggle input {
    appearance: none; position: relative; width: 30px; height: 18px; border-radius: 12px;
    background: var(--text-muted); border: 0; cursor: pointer; flex-shrink: 0;
  }
  .toggle input::before {
    content: ""; position: absolute; left: 3px; top: 3px; width: 12px; height: 12px;
    border-radius: 50%; background: #fff; transition: transform .15s;
  }
  .toggle input:checked { background: var(--accent); }
  .toggle input:checked::before { transform: translateX(12px); }
  .workspace-panels { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(0,1fr) minmax(0,1fr); gap: 20px; }
  .editor-panel, .preview-panel {
    display: flex; flex-direction: column; min-width: 0; min-height: 0; overflow: hidden;
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius);
    box-shadow: 0 1px 2px rgba(20,30,48,.025);
  }
  .panel-header {
    display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-shrink: 0;
    height: 58px; padding: 10px 18px; border-bottom: 1px solid var(--border);
  }
  .panel-title { display: flex; align-items: center; gap: 9px; min-width: 0; }
  .panel-title svg { color: var(--text-muted); flex-shrink: 0; }
  .panel-title h2 { font-size: 13px; font-weight: 600; white-space: nowrap; }

  /* Source editor */
  .editor-body { flex: 1; min-height: 0; display: flex; background: var(--editor); overflow: hidden; }
  #line-numbers, #zpl-input { font: 13px/22px var(--font-mono); padding-top: 20px; padding-bottom: 24px; tab-size: 2; }
  #line-numbers {
    flex-shrink: 0; min-width: 48px; padding-left: 14px; padding-right: 12px;
    text-align: right; color: var(--text-muted); overflow: hidden; user-select: none;
  }
  #zpl-input {
    display: block; flex: 1; min-width: 0; min-height: 0; width: 100%; padding-left: 4px; padding-right: 20px;
    background: transparent; color: var(--text); border: 0; outline: none; resize: none; overflow: auto; caret-color: var(--accent);
  }
  .editor-body:focus-within { box-shadow: inset 0 0 0 2px var(--sel); }
  .editor-meta {
    display: flex; align-items: center; gap: 12px; min-height: 34px; padding: 6px 18px;
    border-top: 1px solid var(--border); color: var(--text-muted); font-size: 11px; flex-shrink: 0;
  }
  .syntax-status { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-dim); }
  .syntax-status:empty { display: none; }
  .syntax-status.syn-ok::before, .syntax-status.syn-err::before { content: ""; display: inline-block; width: 6px; height: 6px; margin-right: 6px; border-radius: 50%; }
  .syntax-status.syn-ok::before { background: var(--success); }
  .syntax-status.syn-err::before { background: var(--error); }
  #caret-pos { margin-left: auto; white-space: nowrap; flex-shrink: 0; font-variant-numeric: tabular-nums; }
  .editor-actions { flex-shrink: 0; display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 14px 18px; border-top: 1px solid var(--border); }
  #file-input { display: none; }
  .ghost-btn, .fmt-btn, .compare-btn, .dl-btn {
    display: inline-flex; align-items: center; justify-content: center; gap: 7px; min-height: 36px;
    padding: 7px 12px; border: 1px solid var(--border); border-radius: 7px;
    background: var(--surface); color: var(--text-dim); font-size: 12px; font-weight: 500;
    text-decoration: none; white-space: nowrap; cursor: pointer;
  }
  .ghost-btn:hover:not(:disabled), .fmt-btn:hover:not(:disabled), .compare-btn:hover:not(:disabled), .dl-btn:hover:not(:disabled) {
    color: var(--text); background: var(--surface2); border-color: var(--text-muted);
  }
  .fmt-btn { border-color: transparent; min-height: 32px; padding: 5px 8px; }
  #render-btn {
    display: inline-flex; align-items: center; justify-content: center; gap: 8px; min-height: 38px; padding: 8px 16px;
    border: 1px solid transparent; border-radius: 7px; background: var(--accent); color: #fff;
    font-size: 12px; font-weight: 600; cursor: pointer; white-space: nowrap;
  }
  :root[data-theme="dark"] #render-btn { color: #101a30; }
  #render-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .shortcut { margin-left: 8px; font-size: 10px; font-weight: 400; opacity: .75; }

  /* Preview canvas and exports */
  .preview-panel { position: relative; }
  #preview-scroll {
    display: flex; flex-direction: column; align-items: center; flex: 1; min-height: 0;
    overflow: auto; padding: 24px; gap: 20px;
    background: radial-gradient(var(--grid-dot) .8px, transparent .8px) 0 0 / 16px 16px, var(--canvas);
  }
  #preview-scroll.zoomed { align-items: flex-start; }
  #preview-img {
    display: none; max-width: none; flex-shrink: 0; height: auto; margin: auto;
    border: 1px solid #d5dae2; border-radius: 2px; background: #fff; box-shadow: var(--shadow);
  }
  #preview-img.visible { display: block; }
  #preview-scroll.zoomed #preview-img { margin: 0; }
  .empty-state { margin: auto; text-align: center; max-width: 300px; padding: 20px 0; }
  .empty-art { display: block; margin: 0 auto 22px; color: var(--text-muted); }
  .empty-state h3 { font-size: 16px; font-weight: 600; margin-bottom: 7px; }
  .empty-state p { color: var(--text-dim); font-size: 13px; line-height: 1.7; }
  .empty-shortcut { margin-top: 18px; }
  kbd { color: var(--text-dim); font: 11px var(--font-ui); padding: 5px 8px; border-radius: 5px; border: 1px solid var(--border); background: var(--surface); }
  #zoom-ctrl { display: none; align-items: center; gap: 2px; }
  #zoom-ctrl.visible { display: flex; }
  #zoom-ctrl button {
    display: inline-flex; align-items: center; justify-content: center; width: 28px; height: 28px;
    background: transparent; color: var(--text-dim); border: 0; border-radius: 5px; cursor: pointer; font-size: 16px;
  }
  #zoom-ctrl button:hover { background: var(--surface2); color: var(--text); }
  #zoom-label { color: var(--text-dim); font-size: 11px; min-width: 38px; text-align: center; font-variant-numeric: tabular-nums; }
  #loading { display: none; position: absolute; inset: 58px 0 0; z-index: 10; align-items: center; justify-content: center; flex-direction: column; gap: 12px; background: var(--overlay); }
  #loading.active { display: flex; }
  .spinner { width: 28px; height: 28px; border: 2px solid var(--border); border-top-color: var(--accent); border-radius: 50%; animation: spin .7s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .loading-text { color: var(--text-dim); font-size: 13px; }
  #error-banner { display: none; width: 100%; color: var(--error); background: var(--err-bg); border: 1px solid var(--error); border-radius: 8px; padding: 14px; font: 12px/1.7 var(--font-mono); overflow-wrap: anywhere; white-space: pre-wrap; }
  #error-banner.visible { display: block; }
  .preview-actions { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px; flex-shrink: 0; padding: 12px 18px; border-top: 1px solid var(--border); }
  .compare-btn { min-height: 38px; padding: 7px 12px; color: var(--accent); background: var(--accent-soft); }
  .compare-btn:disabled { opacity: .65; }
  .compare-note { font-size: 10px; padding: 2px 5px; background: var(--surface); border-radius: 4px; }
  #download-bar { display: none; align-items: center; justify-content: flex-end; gap: 7px; width: 100%; }
  #download-bar.visible { display: flex; }
  .dl-btn { color: var(--text); padding: 7px 10px; font-size: 11px; }
  .dl-btn-pdf { background: var(--surface2); }
  .icon-only { width: 34px; padding: 7px; }
  .mini-spinner { display: none; width: 13px; height: 13px; border: 2px solid var(--border); border-top-color: var(--accent); border-radius: 50%; animation: spin .7s linear infinite; }
  .dl-btn-pdf.loading .mini-spinner { display: inline-block; }
  .dl-btn-pdf.loading .dl-icon-pdf { display: none; }
  .status-bar { display: flex; align-items: center; gap: 12px; min-height: 30px; padding: 6px 18px; border-top: 1px solid var(--border); color: var(--text-muted); font-size: 10px; flex-shrink: 0; }
  #status-size { margin-left: auto; }
  .status-ok { color: var(--success); }
  .status-err { color: var(--error); }
  #status-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  #status-size, #status-time { flex-shrink: 0; white-space: nowrap; font-variant-numeric: tabular-nums; }
  #toast { position: fixed; left: 50%; bottom: 38px; z-index: 20; max-width: calc(100vw - 32px); width: max-content; padding: 11px 16px; border: 1px solid var(--border); border-radius: 8px; background: var(--surface); color: var(--text); box-shadow: var(--shadow); font-size: 12px; text-align: center; opacity: 0; pointer-events: none; transform: translate(-50%,8px); transition: opacity .2s, transform .2s; }
  #toast.show { opacity: 1; transform: translate(-50%,0); }

  /* Reference comparison */
  #compare-section { display: none; width: 100%; }
  #compare-section.visible { display: block; }
  .verdict-row { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; background: var(--surface); border: 1px solid var(--border); border-radius: 8px; padding: 12px; }
  .verdict-badge { font-size: 10px; font-weight: 700; letter-spacing: .5px; padding: 4px 8px; border-radius: 4px; color: var(--badge-fg); }
  .v-perfect, .v-good { background: var(--verdict-ok); }
  .v-minor { background: var(--verdict-warn); }
  .v-moderate { background: var(--verdict-mod); }
  .v-high { background: var(--verdict-bad); }
  .verdict-meta { color: var(--text-dim); font-size: 11px; }
  .compare-grid { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 14px; }
  .compare-col { flex: 1 1 130px; min-width: 0; border: 1px solid var(--border); border-radius: 8px; overflow: hidden; background: var(--surface); }
  .col-title { display: block; padding: 9px 10px; color: var(--text-dim); font-size: 10px; border-bottom: 1px solid var(--border); }
  .compare-col img { display: block; width: 100%; background: #fff; }

  /* Quiet footer disclosure */
  details.about { flex-shrink: 0; color: var(--text-dim); font-size: 11px; }
  details.about summary { display: flex; align-items: center; justify-content: center; gap: 8px; list-style: none; padding: 6px 20px 14px; cursor: pointer; user-select: none; }
  details.about summary::-webkit-details-marker { display: none; }
  details.about summary::before { content: "+"; font-size: 14px; }
  details.about[open] summary::before { content: "−"; }
  details.about[open] { background: var(--surface); border-top: 1px solid var(--border); }
  details.about[open] summary { padding: 10px 20px; border-bottom: 1px solid var(--border); }
  .about-body { max-width: 1000px; margin: auto; padding: 20px 32px; max-height: 38vh; overflow: auto; font-size: 12px; line-height: 1.8; }
  .about-body h1 { color: var(--text); font-size: 15px; margin-bottom: 8px; }
  .about-body h2 { color: var(--text); font-size: 13px; margin: 16px 0 4px; }
  .about-body p { margin-bottom: 8px; }
  .about-body ul { margin: 8px 0 8px 20px; }
  .about-body dt { color: var(--text); font-weight: 600; margin-top: 10px; }
  .about-body code { background: var(--surface2); border-radius: 3px; padding: 2px 4px; }
  .about-body a { color: var(--accent); text-decoration: none; }
  .about-body a:hover { text-decoration: underline; }
  .about-links { margin-top: 14px; }

  @media (max-width: 1100px) {
    .app-header { padding: 12px 24px; }
    main { padding: 22px 24px 14px; gap: 16px; }
    .workspace-panels { gap: 16px; }
    .settings-bar { gap: 14px; }
    .settings-toggles { gap: 14px; }
    .preview-actions { padding: 12px; }
    .dl-btn { padding: 7px 8px; }
  }
  @media (max-width: 900px), (max-height: 640px) {
    html, body { height: auto; min-height: 100%; }
    body { overflow: auto; }
    main { flex: none; }
    .workspace-panels { flex: none; min-height: 540px; }
  }
  @media (max-width: 900px) {
    .workspace-intro { align-items: flex-start; flex-direction: column; gap: 16px; }
    .intro-actions { width: 100%; }
    .workspace-panels { grid-template-columns: minmax(0,1fr); }
    .editor-panel { height: 470px; }
    .preview-panel { height: 620px; }
    .settings-toggles { margin-left: 0; }
  }
  @media (max-width: 600px) {
    .app-header { min-height: 60px; padding: 10px 16px; gap: 12px; }
    .logo { gap: 7px; }
    .logo-icon { width: 25px; height: 25px; }
    .logo-text { font-size: 17px; }
    .nav-label, .header-link { display: none; }
    .header-actions { gap: 5px; }
    #lang { max-width: 106px; padding-left: 8px; }
    main { padding: 24px 16px 12px; }
    .workspace-intro { align-items: flex-start; flex-direction: column; gap: 16px; }
    .workspace-intro h1 { font-size: 24px; }
    .intro-actions { flex-wrap: wrap; }
    .intro-actions .compare-btn { flex: 1 1 100%; }
    #samples { flex: 1; max-width: none; min-width: 0; }
    .intro-actions .ghost-btn { min-height: 38px; }
    .settings-bar { padding: 16px; gap: 14px 12px; }
    .settings-bar > .setting { flex: 1 1 calc(50% - 12px); }
    .settings-bar #fmt, .settings-bar #size-preset, .settings-bar #dpmm { min-width: 0; width: 100%; }
    #custom-size { flex: 1 1 100%; gap: 8px; }
    #custom-size .setting { flex: 1; width: auto; }
    #custom-size .unit-setting { flex: 0 0 72px; }
    .settings-toggles { flex: 1 1 100%; gap: 22px; }
    .panel-header { padding: 10px 14px; }
    .editor-actions { padding: 12px 14px; }
    .editor-meta, .status-bar { padding-left: 14px; padding-right: 14px; }
    .editor-panel { height: 430px; }
    .preview-panel { height: 570px; }
    #preview-scroll { padding: 20px; }
    #line-numbers { min-width: 38px; padding-left: 8px; padding-right: 9px; }
    #zpl-input { padding-right: 12px; }
    .preview-actions { gap: 8px; }
    #download-bar { margin-left: 0; }
    .shortcut { display: none; }
    details.about summary { font-size: 10px; }
    .about-body { padding: 16px; }
  }
  @media (prefers-reduced-motion: reduce) {
    *, *::before, *::after { transition: none !important; animation-duration: .01ms !important; animation-iteration-count: 1 !important; }
  }
</style>
</head>
<body>
<noscript>
  <div style="max-width:760px;margin:0 auto;padding:24px 20px;font-family:system-ui,sans-serif;line-height:1.6;color:var(--text)">
    <h1 style="font-size:20px">Labelize — free online ZPL, EPL, TSPL, CPCL &amp; DPL label viewer</h1>
    <p>
      Labelize renders thermal-printer label code — <strong>ZPL</strong> (Zebra),
      <strong>EPL</strong> (Eltron), <strong>TSPL/TSC</strong>,
      <strong>CPCL</strong> (Zebra mobile) and <strong>DPL</strong>
      (Datamax-O'Neil) — to PNG images or PDF documents.
      This playground needs JavaScript for the live editor and preview; for headless use,
      call the free REST API instead:
    </p>
    <ul>
      <li><code>POST /convert</code> — label code in, PNG or PDF out</li>
      <li><code>POST /validate</code> — parse report with per-field diagnostics</li>
      <li><code>POST /format</code> — reflow label code one command per line</li>
    </ul>
    <p>
      The same engine also ships as a CLI, a Rust crate, a WebAssembly package and a
      Docker image. Source code and docs:
      <a href="https://github.com/GOODBOY008/labelize">github.com/GOODBOY008/labelize</a>
    </p>
  </div>
</noscript>

<header class="app-header">
  <div class="logo">
    <svg class="logo-icon" viewBox="374 246 520 471" xmlns="http://www.w3.org/2000/svg" aria-hidden="true" focusable="false">
      <defs>
        <linearGradient id="lzA" x1="0.08" y1="0.02" x2="0.95" y2="0.98"><stop offset="0%" stop-color="#10a7f5"/><stop offset="52%" stop-color="#167ce9"/><stop offset="100%" stop-color="#4734e9"/></linearGradient>
        <linearGradient id="lzB" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="#6425f3"/><stop offset="100%" stop-color="#7548ef"/></linearGradient>
      </defs>
      <path d="M 463,570 463,577 467,580 641,603 646,599 648,591 647,584 643,582 477,560 469,560 466,562 Z M 869,551 861,546 852,544 833,543 764,534 744,537 729,545 709,565 698,585 670,686 659,701 649,708 641,711 655,713 683,713 695,711 715,704 741,689 756,677 870,563 871,554 Z M 476,525 477,532 483,534 672,558 679,554 682,545 681,539 675,536 608,529 487,513 481,514 478,517 Z M 855,282 832,274 549,250 526,250 517,252 499,260 488,268 473,285 464,303 380,563 378,573 379,607 387,631 401,653 424,673 448,684 620,709 633,708 645,702 657,689 661,680 661,677 469,649 447,638 435,626 428,615 421,592 422,564 491,329 495,321 503,313 512,309 529,309 822,336 833,340 841,351 842,359 798,529 838,534 890,352 890,333 884,313 875,299 864,288 Z" fill="url(#lzA)" fill-rule="evenodd"/>
      <path d="M 869,551 861,546 852,544 833,543 764,534 744,537 729,545 709,565 698,585 691,606 689,618 670,686 659,701 649,708 641,711 655,713 683,713 695,711 715,704 741,689 756,677 870,563 871,554 Z" fill="url(#lzB)" fill-rule="evenodd"/>
      <path d="M 462,573 465,579 472,581 487,582 501,585 516,586 635,603 643,602 646,599 648,592 648,585 644,582 591,576 478,560 467,561 465,563 Z M 476,524 476,531 481,534 674,558 679,555 682,545 682,540 676,536 593,527 489,513 481,514 478,517 Z" fill="#6077aa" fill-rule="evenodd"/>
      <path d="M 791,379 763,377 760,381 750,425 730,499 731,503 760,506 763,503 794,386 794,381 Z M 742,375 731,374 728,377 721,408 697,497 699,500 709,501 712,499 743,379 Z M 709,372 689,370 686,373 655,493 658,496 677,497 680,493 680,489 685,474 692,443 710,377 Z M 667,368 654,366 649,368 618,487 619,491 635,493 638,490 666,378 668,374 Z M 630,364 614,363 612,365 603,403 581,483 581,486 584,488 598,489 601,485 631,369 Z M 592,360 571,359 569,361 560,399 538,479 540,483 558,485 563,483 594,367 594,361 Z M 549,356 527,354 523,358 492,471 492,476 494,478 517,480 520,478 546,377 551,362 551,358 Z" fill="#111a39" fill-rule="evenodd"/>
    </svg>
    <span class="logo-text">Labelize</span>
  </div>
  <span class="nav-label" data-i18n="nav.playground">Playground</span>
  <nav class="header-actions" aria-label="Resources and preferences" data-i18n-aria="nav.resources">
    <a class="header-link" href="https://github.com/GOODBOY008/labelize/blob/main/docs/USAGE.md" target="_blank" rel="noopener" data-i18n="nav.docs">Documentation</a>
    <a class="header-link" href="https://github.com/GOODBOY008/labelize" target="_blank" rel="noopener">GitHub ↗</a>
    <select id="lang" class="hdr-select" aria-label="Language" data-i18n-aria="hdr.lang">
    <option value="en">English</option>
    <option value="zh">简体中文</option>
  </select>
    <button id="theme-btn" type="button" title="Toggle theme" aria-label="Toggle theme"
          data-i18n-title="hdr.theme" data-i18n-aria="hdr.theme">
    <svg class="icon-sun" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="12" cy="12" r="5"/>
      <line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/>
      <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
      <line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/>
      <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
    </svg>
    <svg class="icon-moon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
    </svg>
  </button>
  </nav>
</header>

<main>
  <section class="workspace-intro" aria-labelledby="workspace-title">
    <div>
      <h1 id="workspace-title" data-i18n="workspace.title">Label playground</h1>
      <p data-i18n="workspace.description">Write, preview, and export your thermal labels.</p>
    </div>
    <div class="intro-actions">
      <select id="samples" class="hdr-select" aria-label="Samples" data-i18n-aria="hdr.samples">
    <option value="" selected disabled data-i18n="hdr.samples">Samples</option>
    <option value="shipping" data-i18n="sample.shipping">Shipping label</option>
    <option value="barcodes" data-i18n="sample.barcodes">Barcodes &amp; 2D</option>
    <option value="graphics" data-i18n="sample.graphics">Shapes &amp; graphics</option>
    <option value="epl" data-i18n="sample.epl">EPL showcase</option>
    <option value="tspl" data-i18n="sample.tspl">TSPL showcase</option>
    <option value="cpcl" data-i18n="sample.cpcl">CPCL showcase</option>
    <option value="dpl" data-i18n="sample.dpl">DPL showcase</option>
  </select>
      <input id="file-input" type="file" accept=".zpl,.epl,.tspl,.cpcl,.dpl">
      <button id="open-file-btn" class="ghost-btn">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
        </svg>
        <span data-i18n="btn.open">Open File</span>
      </button>
      <button id="compare-btn" class="compare-btn" type="button">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><path d="M4 7h16m-4-4 4 4-4 4M20 17H4m4-4-4 4 4 4"/></svg> <span data-i18n="btn.compare">Compare with Labelary</span>
        <span id="compare-note" class="compare-note" hidden data-i18n="compare.zplOnly">ZPL only</span>
      </button>
    </div>
  </section>

  <section class="settings-bar" aria-label="Label settings" data-i18n-aria="panel.settings">
    <div class="setting">
      <label for="fmt" data-i18n="field.format">Format</label>
      <select id="fmt">
          <option value="zpl" selected>ZPL</option>
          <option value="epl">EPL</option>
          <option value="tspl">TSPL</option>
          <option value="cpcl">CPCL</option>
          <option value="dpl">DPL</option>
        </select>
    </div>
    <div class="setting">
      <label for="size-preset" data-i18n="field.size">Label size</label>
      <select id="size-preset">
          <option value="4x6" selected data-i18n="size.4x6">4 &times; 6 in</option>
          <option value="4x4" data-i18n="size.4x4">4 &times; 4 in</option>
          <option value="4x3" data-i18n="size.4x3">4 &times; 3 in</option>
          <option value="2x4" data-i18n="size.2x4">2 &times; 4 in</option>
          <option value="2x2" data-i18n="size.2x2">2 &times; 2 in</option>
          <option value="3.5x1.5" data-i18n="size.35x15">3.5 &times; 1.5 in</option>
          <option value="custom" data-i18n="size.custom">Custom&hellip;</option>
        </select>
    </div>
    <div id="custom-size">
      <div class="setting">
        <label for="label-width" data-i18n="field.width">Width</label>
        <input id="label-width" type="number" value="101.6" min="0" step="any" required>
      </div>
      <span class="size-sep" aria-hidden="true">×</span>
      <div class="setting">
        <label for="label-height" data-i18n="field.height">Height</label>
        <input id="label-height" type="number" value="152.4" min="0" step="any" required>
      </div>
      <div class="setting unit-setting">
        <label for="size-unit" data-i18n="field.unit">Unit</label>
        <select id="size-unit" aria-label="Size unit" data-i18n-aria="field.unit">
          <option value="mm" selected data-i18n="unit.mm">mm</option>
          <option value="in" data-i18n="unit.in">in</option>
        </select>
      </div>
    </div>
    <div class="setting">
      <label for="dpmm" data-i18n="field.resolution">Resolution</label>
      <select id="dpmm">
          <option value="6">6 dpmm</option>
          <option value="8" selected>8 dpmm</option>
          <option value="12">12 dpmm</option>
          <option value="24">24 dpmm</option>
        </select>
    </div>
    <div class="settings-toggles">
      <label class="toggle" for="auto-render" id="auto-wrap" title="Re-render automatically as you type" data-i18n-title="tip.auto">
        <input type="checkbox" id="auto-render" checked>
        <span data-i18n="btn.auto">Live preview</span>
      </label>
      <label class="toggle" for="antialias" id="aa-wrap" title="Emit 8-bit grayscale output instead of 1-bit black/white" data-i18n-title="tip.aa">
        <input type="checkbox" id="antialias">
        <span data-i18n="btn.aa">Antialias</span>
      </label>
    </div>
  </section>

  <div class="workspace-panels">
    <section class="editor-panel" aria-labelledby="source-heading">
      <div class="panel-header">
        <div class="panel-title"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m8 7-5 5 5 5m8-10 5 5-5 5m-3-13-2 16"/></svg><h2 id="source-heading" data-i18n="panel.code">Source code</h2></div>
        <button id="format-btn" class="fmt-btn" type="button" data-i18n="btn.format" data-i18n-title="tip.format" title="Reflow one command per line">Format code</button>
      </div>
      <div class="editor-body">
        <pre id="line-numbers" aria-hidden="true">1</pre>
        <textarea id="zpl-input" aria-label="Label source code" data-i18n-aria="panel.sourceLabel" wrap="off" spellcheck="false" autocomplete="off" autocorrect="off" autocapitalize="off">^XA
^FX Top section with logo, name and address.
^CF0,60
^FO50,50^GB100,100,100^FS
^FO75,75^FR^GB100,100,100^FS
^FO93,93^GB40,40,40^FS
^FO220,50^FDIntershipping, Inc.^FS
^CF0,30
^FO220,115^FD1000 Shipping Lane^FS
^FO220,155^FDShelbyville TN 38102^FS
^FO220,195^FDUnited States (USA)^FS
^FO50,250^GB700,3,3^FS
^FX Second section with recipient address and permit information.
^CFA,30
^FO50,300^FDJohn Doe^FS
^FO50,340^FD100 Main Street^FS
^FO50,380^FDSpringfield TN 39021^FS
^FO50,420^FDUnited States (USA)^FS
^CFA,15
^FO600,300^GB150,150,3^FS
^FO638,340^FDPermit^FS
^FO638,390^FD123456^FS
^FO50,500^GB700,3,3^FS
^FX Third section with bar code.
^BY5,2,270
^FO100,550^BC^FD12345678^FS
^FO100,850^BY2,3,60^BQ,,2^FDQA,https://github.com/GOODBOY008/labelize^FS
^XZ</textarea>
      </div>
      <div class="editor-meta">
        <span id="syntax-status" class="syntax-status" aria-live="polite"></span>
        <span id="caret-pos" aria-live="off"></span>
      </div>
      <div class="editor-actions">
        <button id="share-btn" class="ghost-btn" title="Copy a permalink to this label" data-i18n-title="tip.share">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
          <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>
          <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>
        </svg>
        <span data-i18n="btn.share">Share</span>
      </button>
        <button id="render-btn">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M7 4v16l14-8z"/></svg> <span data-i18n="btn.render">Render</span> <span class="shortcut">Ctrl+&#9166;</span>
      </button>
      </div>
    </section>

    <section class="preview-panel" aria-labelledby="preview-heading">
      <div class="panel-header">
        <div class="panel-title"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 7h8M8 11h5M8 15v3m3-3v3m2-3v3m3-3v3"/></svg><h2 id="preview-heading" data-i18n="panel.preview">Label preview</h2></div>
        <div id="zoom-ctrl">
      <button id="zoom-out" type="button" title="Zoom out" data-i18n-title="tip.zoomOut">&minus;</button>
      <span id="zoom-label">Fit</span>
      <button id="zoom-in" type="button" title="Zoom in" data-i18n-title="tip.zoomIn">+</button>
      <button id="zoom-fit" type="button" title="Fit label to preview" aria-label="Fit label to preview" data-i18n-title="tip.zoomFit" data-i18n-aria="tip.zoomFit"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5"/></svg></button>
    </div>
      </div>
      <div id="loading">
      <div class="spinner"></div>
      <span class="loading-text" id="loading-text" data-i18n="loading.render">Rendering&hellip;</span>
    </div>
      <div id="preview-scroll">
        <div class="empty-state" id="empty-state">
          <svg class="empty-art" width="76" height="88" viewBox="0 0 76 88" fill="none" aria-hidden="true">
            <rect x="10" y="4" width="56" height="76" rx="7" fill="var(--surface)" stroke="var(--border)" stroke-width="1.5"/>
            <path d="M23 22h30M23 30h20" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <path d="M23 47v15m5-15v15m3-15v15m6-15v15m4-15v15m6-15v15m5-15v15" stroke="currentColor" stroke-width="2"/>
            <path d="M27 70h22" stroke="var(--border)" stroke-width="2" stroke-linecap="round"/>
          </svg>
          <h3 data-i18n="empty.heading">Preview your label</h3>
          <p data-i18n-html="empty.title">Choose your settings, then press <strong>Render</strong> to see the result.</p>
          <div class="empty-shortcut"><kbd>Ctrl + Enter</kbd></div>
        </div>
        <div id="error-banner" role="alert"></div>
        <img id="preview-img" alt="Rendered label" data-i18n-alt="alt.preview">
        <div id="compare-section">
        <div class="verdict-row">
          <span id="verdict-badge" class="verdict-badge"></span>
          <span id="verdict-meta" class="verdict-meta"></span>
        </div>
        <div class="compare-grid">
          <div class="compare-col">
            <span class="col-title" data-i18n="compare.ref">Labelary &mdash; reference</span>
            <img id="compare-labelary" alt="Labelary reference render" data-i18n-alt="alt.labelary">
          </div>
          <div class="compare-col">
            <span class="col-title">Labelize</span>
            <img id="compare-labelize" alt="Labelize render" data-i18n-alt="alt.labelize">
          </div>
          <div class="compare-col">
            <span class="col-title" data-i18n="compare.diff">Diff &mdash; red = differing pixels</span>
            <img id="compare-diff" alt="Diff overlay" data-i18n-alt="alt.diff">
          </div>
        </div>
      </div>
      </div>
      <div id="download-bar" class="preview-actions">
        <a id="dl-png" class="dl-btn dl-btn-png" href="#" download="label.png" title="Download PNG" aria-label="Download PNG" data-i18n-title="dl.png" data-i18n-aria="dl.png">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          <span>PNG</span>
        </a>

        <button id="copy-img-btn" class="dl-btn dl-btn-png icon-only" title="Copy PNG to clipboard"
                data-i18n-title="tip.copyImg" aria-label="Copy PNG to clipboard" data-i18n-aria="tip.copyImg">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <rect x="9" y="9" width="13" height="13" rx="2"/>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
          </svg>
        </button>

        <button id="dl-pdf" class="dl-btn dl-btn-pdf" title="Download PDF" aria-label="Download PDF" data-i18n-title="dl.pdf" data-i18n-aria="dl.pdf">
          <svg class="dl-icon-pdf" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          <span class="mini-spinner"></span>
          <span>PDF</span>
        </button>
      </div>
      <div class="status-bar">
        <span id="status-text" aria-live="polite"></span>
        <span id="status-size"></span>
        <span id="status-time"></span>
      </div>
      <div id="toast" role="status"></div>
    </section>
  </div>
</main>

<details class="about">
  <summary data-i18n="about.summary">About Labelize · FAQ · Open source (MIT)</summary>
  <div class="about-body">
    <h1 data-i18n-html="about.h1">Labelize — free online ZPL, EPL, TSPL, CPCL &amp; DPL label viewer</h1>
    <p data-i18n-html="about.pitch">Labelize renders thermal-printer label code — <strong>ZPL</strong> (Zebra), <strong>EPL</strong> (Eltron), <strong>TSPL/TSC</strong>, <strong>CPCL</strong> (Zebra mobile) and <strong>DPL</strong> (Datamax-O'Neil) — to PNG images or PDF documents. Paste your label code above, press <strong>Render</strong>, or explore the built-in samples.</p>
    <ul data-i18n-html="about.features"></ul>
    <h2 data-i18n="about.faq">Frequently asked questions</h2>
    <dl>
      <dt data-i18n="about.q1">Is this ZPL viewer free?</dt>
      <dd data-i18n-html="about.a1">Yes. Labelize is free to use, MIT-licensed open source, and requires no sign-up.</dd>
      <dt data-i18n="about.q2">Which label languages are supported?</dt>
      <dd data-i18n-html="about.a2">ZPL (Zebra), EPL (Eltron), TSPL/TSC, CPCL (Zebra mobile) and DPL (Datamax-O'Neil). Labels render to PNG or PDF at selectable sizes and resolutions.</dd>
      <dt data-i18n="about.q3">Is there an API?</dt>
      <dd data-i18n-html="about.a3">Yes — <code>POST /convert</code>, <code>/validate</code> and <code>/format</code> on this host, or self-host the CLI, Rust crate, WebAssembly package or Docker image.</dd>
      <dt data-i18n="about.q4">Does my label data leave my browser?</dt>
      <dd data-i18n-html="about.a4">The playground renders labels server-side and stores nothing. The optional <em>Compare with Labelary</em> feature sends your code to the external labelary.com service and can be disabled.</dd>
    </dl>
    <p class="about-links">
      <a href="https://github.com/GOODBOY008/labelize" data-i18n="about.gh">GitHub source</a> ·
      <a href="https://www.npmjs.com/package/@goodboy008/labelize-wasm" data-i18n="about.npm">npm package</a> ·
      <a href="https://crates.io/crates/labelize" data-i18n="about.crates">crates.io</a>
    </p>
  </div>
</details>

<script>
(function () {
  "use strict";

  /* ── Persistence (private-mode safe) ── */
  var store = {
    get: function (k) { try { return localStorage.getItem(k); } catch (e) { return null; } },
    set: function (k, v) { try { localStorage.setItem(k, v); } catch (e) {} }
  };

  /* ── Deployment config (server-injected; see window.__LABELIZE_CONFIG__) ── */
  var CFG = window.__LABELIZE_CONFIG__ || {};
  var labelaryCompare = CFG.labelaryCompare !== false;

  /* ── i18n ──
     Static text is tagged with data-i18n / data-i18n-html / data-i18n-title /
     data-i18n-aria / data-i18n-alt and swapped by applyI18n(); dynamic strings
     go through t(key) and fmt(t(key), {placeholder: value}). */
  var I18N = {
    en: {
      "nav.playground": "Playground",
      "nav.resources": "Resources and preferences",
      "nav.docs": "Documentation",
      "workspace.title": "Label playground",
      "workspace.description": "Write, preview, and export your thermal labels.",
      "panel.settings": "Label settings",
      "panel.sourceLabel": "Label source code",
      "field.resolution": "Resolution",
      "empty.heading": "Preview your label",
      "tip.format": "Reflow one command per line",
      "app.title": "Free ZPL Viewer — ZPL, EPL, TSPL, CPCL & DPL to PNG | Labelize",
      "app.tagline": "ZPL, EPL, TSPL, CPCL & DPL Label Playground",
      "about.summary": "About Labelize · FAQ · Open source (MIT)",
      "about.h1": "Labelize — free online ZPL, EPL, TSPL, CPCL &amp; DPL label viewer",
      "about.pitch": "Labelize renders thermal-printer label code — <strong>ZPL</strong> (Zebra), <strong>EPL</strong> (Eltron), <strong>TSPL/TSC</strong>, <strong>CPCL</strong> (Zebra mobile) and <strong>DPL</strong> (Datamax-O'Neil) — to PNG images or PDF documents. Paste your label code above, press <strong>Render</strong>, or explore the built-in samples.",
      "about.features": "<li>Live preview with zoom, PNG/PDF download and shareable permalinks</li><li>Syntax validation and one-command-per-line formatting</li><li>Optional pixel diff against the Labelary reference renderer</li><li>Free REST API: <code>POST /convert</code>, <code>/validate</code>, <code>/format</code></li><li>Same engine as CLI, HTTP microservice, Rust crate, WebAssembly package or Docker image</li>",
      "about.faq": "Frequently asked questions",
      "about.q1": "Is this ZPL viewer free?",
      "about.a1": "Yes. Labelize is free to use, MIT-licensed open source, and requires no sign-up.",
      "about.q2": "Which label languages are supported?",
      "about.a2": "ZPL (Zebra), EPL (Eltron), TSPL/TSC, CPCL (Zebra mobile) and DPL (Datamax-O'Neil). Labels render to PNG or PDF at selectable sizes and resolutions.",
      "about.q3": "Is there an API?",
      "about.a3": "Yes — <code>POST /convert</code>, <code>/validate</code> and <code>/format</code> on this host, or self-host the CLI, Rust crate, WebAssembly package or Docker image.",
      "about.q4": "Does my label data leave my browser?",
      "about.a4": "The playground renders labels server-side and stores nothing. The optional <em>Compare with Labelary</em> feature sends your code to the external labelary.com service and can be disabled.",
      "about.gh": "GitHub source",
      "about.npm": "npm package",
      "about.crates": "crates.io",
      "hdr.samples": "Load a sample",
      "hdr.lang": "Language",
      "hdr.theme": "Toggle theme",
      "panel.code": "Source code",
      "panel.preview": "Label preview",
      "field.format": "Format",
      "field.size": "Label size",
      "field.width": "Width",
      "field.height": "Height",
      "field.unit": "Unit",
      "unit.in": "in",
      "unit.mm": "mm",
      "size.4x6": "4 × 6 in",
      "size.4x4": "4 × 4 in",
      "size.4x3": "4 × 3 in",
      "size.2x4": "2 × 4 in",
      "size.2x2": "2 × 2 in",
      "size.35x15": "3.5 × 1.5 in",
      "size.custom": "Custom…",
      "sample.shipping": "Shipping label",
      "sample.barcodes": "Barcodes & 2D",
      "sample.graphics": "Shapes & graphics",
      "sample.epl": "EPL showcase",
      "sample.tspl": "TSPL showcase",
      "sample.cpcl": "CPCL showcase",
      "sample.dpl": "DPL showcase",
      "btn.format": "Format code",
      "syn.ok": "syntax OK",
      "syn.bad": "syntax error",
      "syn.oneLabel": "1 label",
      "syn.nLabels": "{n} labels",
      "btn.open": "Open File",
      "btn.compare": "Compare with Labelary",
      "compare.zplOnly": "ZPL only",
      "btn.render": "Render",
      "btn.auto": "Live preview",
      "btn.aa": "Antialias",
      "btn.share": "Share",
      "empty.title": "Choose your settings, then press <strong>Render</strong> to see the result.",
      "loading.render": "Rendering…",
      "loading.compare": "Comparing…",
      "dl.png": "Download PNG",
      "dl.pdf": "Download PDF",
      "compare.ref": "Labelary — reference",
      "compare.diff": "Diff — red = differing pixels",
      "status.ok": "OK",
      "status.err": "Error",
      "status.rendering": "Rendering…",
      "status.comparing": "Comparing…",
      "status.diff": "Diff {pct}%",
      "tip.compare": "Fetch the Labelary reference image and estimate the visual diff",
      "tip.compareEpl": "Labelary does not support EPL, TSPL, CPCL or DPL — ZPL only",
      "tip.auto": "Re-render automatically as you type",
      "tip.aa": "Emit 8-bit grayscale output instead of 1-bit black/white",
      "tip.share": "Copy a permalink to this label",
      "tip.copyImg": "Copy PNG to clipboard",
      "tip.zoomIn": "Zoom in",
      "tip.zoomOut": "Zoom out",
      "tip.zoomFit": "Fit label to preview",
      "zoom.fit": "Fit",
      "caret": "Ln {ln}, Col {col}",
      "toast.urlCopied": "Share link copied to clipboard",
      "toast.imgCopied": "PNG copied to clipboard",
      "toast.copyFail": "Copy failed: {msg}",
      "err.empty": "Editor is empty — paste some ZPL, EPL, TSPL, CPCL or DPL first.",
      "err.size": "Enter a positive number for both label width and height.",
      "err.server": "Server error {status}: {body}",
      "err.network": "Network error: {msg}",
      "err.epl": "Labelary does not support EPL — switch to ZPL to compare.",
      "err.tspl": "Labelary does not support TSPL — switch to ZPL to compare.",
      "err.cpcl": "Labelary does not support CPCL — switch to ZPL to compare.",
      "err.dpl": "Labelary does not support DPL — switch to ZPL to compare.",
      "err.canvas": "Canvas too large ({w}×{h} px) for comparison — lower the dpmm or label size.",
      "err.compare": "Compare failed: {msg}",
      "err.pdf": "PDF error {status}: {body}",
      "err.pdfNet": "PDF download error: {msg}",
      "err.rateLimit": "Labelary is rate-limited (~3 req/s) — wait a moment and retry",
      "err.l404": "Labelary rejected the request (HTTP 404) — unsupported label size or format",
      "err.lHttp": "Labelary request failed: HTTP {status}",
      "err.renderFail": "render failed: HTTP {status}",
      "v.perfect": "Pixel-identical to Labelary.",
      "v.good": "Sub-pixel / anti-alias level noise.",
      "v.minor": "Small font or position deltas.",
      "v.moderate": "Font engine, embedded graphics, or 2D barcode differences.",
      "v.high": "Missing encoder or large structural mismatch.",
      "alt.preview": "Rendered label",
      "alt.labelary": "Labelary reference render",
      "alt.labelize": "Labelize render",
      "alt.diff": "Diff overlay"
    },
    zh: {
      "nav.playground": "在线预览",
      "nav.resources": "资源与偏好设置",
      "nav.docs": "使用文档",
      "workspace.title": "标签工作台",
      "workspace.description": "编写、预览并导出你的热敏标签。",
      "panel.settings": "标签设置",
      "panel.sourceLabel": "标签源代码",
      "field.resolution": "分辨率",
      "empty.heading": "预览你的标签",
      "tip.format": "将每条指令整理为一行",
      "app.title": "Labelize — 免费在线 ZPL/EPL/TSPL/CPCL/DPL 标签查看器",
      "app.tagline": "ZPL、EPL、TSPL、CPCL 与 DPL 标签在线预览",
      "about.summary": "关于 Labelize · FAQ · 开源（MIT）",
      "about.h1": "Labelize — 免费在线 ZPL、EPL、TSPL、CPCL 与 DPL 标签查看器",
      "about.pitch": "Labelize 将热敏打印机标签代码 —— <strong>ZPL</strong>（Zebra）、<strong>EPL</strong>（Eltron）、<strong>TSPL/TSC</strong>、<strong>CPCL</strong>（Zebra 移动打印机）与 <strong>DPL</strong>（Datamax-O'Neil）—— 渲染为 PNG 图片或 PDF 文档。把标签代码粘贴到上方，点击<strong>渲染</strong>，或直接试试内置示例。",
      "about.features": "<li>实时预览：缩放、下载 PNG/PDF、可分享的固定链接</li><li>语法校验与一键格式化（每行一条指令）</li><li>可选：与 Labelary 参考渲染器逐像素对比</li><li>免费 REST API：<code>POST /convert</code>、<code>/validate</code>、<code>/format</code></li><li>同一引擎提供 CLI、HTTP 服务、Rust 库、WebAssembly 包与 Docker 镜像</li>",
      "about.faq": "常见问题",
      "about.q1": "这个 ZPL 查看器免费吗？",
      "about.a1": "免费。Labelize 是 MIT 许可的开源项目，无需注册。",
      "about.q2": "支持哪些标签语言？",
      "about.a2": "ZPL（Zebra）、EPL（Eltron）、TSPL/TSC、CPCL（Zebra 移动打印机）与 DPL（Datamax-O'Neil），可按选定尺寸和分辨率渲染为 PNG 或 PDF。",
      "about.q3": "有 API 吗？",
      "about.a3": "有 —— 在本站调用 <code>POST /convert</code>、<code>/validate</code> 与 <code>/format</code>，或自托管 CLI、Rust 库、WebAssembly 包或 Docker 镜像。",
      "about.q4": "我的标签数据会离开浏览器吗？",
      "about.a4": "在线预览在服务端渲染且不存储任何数据。可选的「与 Labelary 对比」功能会把代码发送到外部 labelary.com 服务，可予关闭。",
      "about.gh": "GitHub 源码",
      "about.npm": "npm 包",
      "about.crates": "crates.io",
      "hdr.samples": "加载示例",
      "hdr.lang": "语言",
      "hdr.theme": "切换主题",
      "panel.code": "源代码",
      "panel.preview": "标签预览",
      "field.format": "格式",
      "field.size": "标签尺寸",
      "field.width": "宽度",
      "field.height": "高度",
      "field.unit": "单位",
      "unit.in": "英寸",
      "unit.mm": "毫米",
      "size.4x6": "4 × 6 英寸",
      "size.4x4": "4 × 4 英寸",
      "size.4x3": "4 × 3 英寸",
      "size.2x4": "2 × 4 英寸",
      "size.2x2": "2 × 2 英寸",
      "size.35x15": "3.5 × 1.5 英寸",
      "size.custom": "自定义…",
      "sample.shipping": "物流标签",
      "sample.barcodes": "条码与二维码",
      "sample.graphics": "图形与绘图",
      "sample.epl": "EPL 示例",
      "sample.tspl": "TSPL 示例",
      "sample.cpcl": "CPCL 示例",
      "sample.dpl": "DPL 示例",
      "btn.format": "格式化",
      "syn.ok": "语法通过",
      "syn.bad": "语法错误",
      "syn.oneLabel": "1 个标签",
      "syn.nLabels": "{n} 个标签",
      "btn.open": "打开文件",
      "btn.compare": "与 Labelary 对比",
      "compare.zplOnly": "仅限 ZPL",
      "btn.render": "渲染",
      "btn.auto": "实时预览",
      "btn.aa": "抗锯齿",
      "btn.share": "分享",
      "empty.title": "设置好标签参数，点击<strong>渲染</strong>查看结果。",
      "loading.render": "渲染中…",
      "loading.compare": "对比中…",
      "dl.png": "下载 PNG",
      "dl.pdf": "下载 PDF",
      "compare.ref": "Labelary — 参考图",
      "compare.diff": "差异图 — 红色为不同像素",
      "status.ok": "完成",
      "status.err": "错误",
      "status.rendering": "渲染中…",
      "status.comparing": "对比中…",
      "status.diff": "差异 {pct}%",
      "tip.compare": "获取 Labelary 参考渲染图并估算视觉差异",
      "tip.compareEpl": "Labelary 不支持 EPL/TSPL/CPCL/DPL — 仅限 ZPL",
      "tip.auto": "输入时自动重新渲染",
      "tip.aa": "输出 8 位灰度（替代 1 位黑白）",
      "tip.share": "复制此标签的分享链接",
      "tip.copyImg": "复制 PNG 到剪贴板",
      "tip.zoomIn": "放大",
      "tip.zoomOut": "缩小",
      "tip.zoomFit": "适应预览区域",
      "zoom.fit": "适应",
      "caret": "行 {ln}，列 {col}",
      "toast.urlCopied": "分享链接已复制到剪贴板",
      "toast.imgCopied": "PNG 已复制到剪贴板",
      "toast.copyFail": "复制失败：{msg}",
      "err.empty": "编辑器为空 — 请先粘贴 ZPL、EPL、TSPL、CPCL 或 DPL 代码。",
      "err.size": "标签宽度和高度都必须是正数。",
      "err.server": "服务器错误 {status}：{body}",
      "err.network": "网络错误：{msg}",
      "err.epl": "Labelary 不支持 EPL — 请切换到 ZPL 后再对比。",
      "err.tspl": "Labelary 不支持 TSPL — 请切换到 ZPL 后再对比。",
      "err.cpcl": "Labelary 不支持 CPCL — 请切换到 ZPL 后再对比。",
      "err.dpl": "Labelary 不支持 DPL — 请切换到 ZPL 后再对比。",
      "err.canvas": "画布过大（{w}×{h} px），无法对比 — 请降低 dpmm 或缩小标签尺寸。",
      "err.compare": "对比失败：{msg}",
      "err.pdf": "PDF 错误 {status}：{body}",
      "err.pdfNet": "PDF 下载错误：{msg}",
      "err.rateLimit": "Labelary 已限流（约 3 次/秒） — 请稍后重试",
      "err.l404": "Labelary 拒绝了请求（HTTP 404） — 标签尺寸或格式不受支持",
      "err.lHttp": "Labelary 请求失败：HTTP {status}",
      "err.renderFail": "渲染失败：HTTP {status}",
      "v.perfect": "与 Labelary 像素级一致。",
      "v.good": "亚像素 / 抗锯齿级别的噪声。",
      "v.minor": "字体或位置的轻微偏差。",
      "v.moderate": "字体引擎、内嵌图形或二维条码的差异。",
      "v.high": "编码器缺失或结构性差异较大。",
      "alt.preview": "渲染后的标签",
      "alt.labelary": "Labelary 参考渲染图",
      "alt.labelize": "Labelize 渲染图",
      "alt.diff": "差异叠加图"
    }
  };

  var lang = (function () {
    var saved = store.get("labelize-lang");
    if (saved === "en" || saved === "zh") return saved;
    return String(navigator.language || "").toLowerCase().indexOf("zh") === 0 ? "zh" : "en";
  })();

  function t(key) {
    var dict = I18N[lang] || I18N.en;
    if (dict[key] != null) return dict[key];
    if (I18N.en[key] != null) return I18N.en[key];
    return key;
  }

  function fmt(str, args) {
    return String(str).replace(/\{(\w+)\}/g, function (m, k) {
      return args && args[k] != null ? args[k] : m;
    });
  }

  function firstLine(s) {
    s = String(s);
    var i = s.indexOf("\n");
    return (i < 0 ? s : s.slice(0, i)).slice(0, 120);
  }

  /* ── Built-in samples ── */
  var SAMPLES = {
    shipping: "^XA\n" +
      "^FX Top section with logo, name and address.\n" +
      "^CF0,60\n" +
      "^FO50,50^GB100,100,100^FS\n" +
      "^FO75,75^FR^GB100,100,100^FS\n" +
      "^FO93,93^GB40,40,40^FS\n" +
      "^FO220,50^FDIntershipping, Inc.^FS\n" +
      "^CF0,30\n" +
      "^FO220,115^FD1000 Shipping Lane^FS\n" +
      "^FO220,155^FDShelbyville TN 38102^FS\n" +
      "^FO220,195^FDUnited States (USA)^FS\n" +
      "^FO50,250^GB700,3,3^FS\n" +
      "^FX Second section with recipient address and permit information.\n" +
      "^CFA,30\n" +
      "^FO50,300^FDJohn Doe^FS\n" +
      "^FO50,340^FD100 Main Street^FS\n" +
      "^FO50,380^FDSpringfield TN 39021^FS\n" +
      "^FO50,420^FDUnited States (USA)^FS\n" +
      "^CFA,15\n" +
      "^FO600,300^GB150,150,3^FS\n" +
      "^FO638,340^FDPermit^FS\n" +
      "^FO638,390^FD123456^FS\n" +
      "^FO50,500^GB700,3,3^FS\n" +
      "^FX Third section with bar code.\n" +
      "^BY5,2,270\n" +
      "^FO100,550^BC^FD12345678^FS\n" +
      "^FO100,850^BY2,3,60^BQ,,2^FDQA,https://github.com/GOODBOY008/labelize^FS\n" +
      "^XZ",
    barcodes: "^XA\n" +
      "^CF0,32\n" +
      "^FO50,40^FDBarcodes & 2D^FS\n" +
      "^FO50,86^GB700,3,3^FS\n" +
      "^CF0,24\n" +
      "^FO50,120^BY4,2,90^BC^FD>61234567890^FS\n" +
      "^FO590,130^FDCode 128^FS\n" +
      "^FO50,290^BY4,2,90^B3N,Y,N,N^FDABC-1234^FS\n" +
      "^FO590,300^FDCode 39^FS\n" +
      "^FO50,460^BY4,2,90^BEN,,N^FD012345678901^FS\n" +
      "^FO590,470^FDEAN-13^FS\n" +
      "^FO60,640^BY6,2,0^BQ,,7^FDQA,https://github.com/GOODBOY008/labelize^FS\n" +
      "^FO300,650^BY5,2,0^BX,7,200^FD1234567890^FS\n" +
      "^FO470,830^FDQR / DataMatrix^FS\n" +
      "^XZ",
    graphics: "^XA\n" +
      "^CF0,32\n" +
      "^FO50,40^FDShapes & Boxes^FS\n" +
      "^FO50,86^GB700,3,3^FS\n" +
      "^FO60,120^GB200,140,4^FS\n" +
      "^FO90,150^FR^GB140,80,50^FS\n" +
      "^FO330,120^GC200,140,B^FS\n" +
      "^FO590,120^GD200,140,B^FS\n" +
      "^FO60,330^GB700,3,3^FS\n" +
      "^FO60,350^GB3,200,3^FS\n" +
      "^FO757,350^GB3,200,3^FS\n" +
      "^FO60,550^GB700,3,3^FS\n" +
      "^FO60,547^FR^GB700,6,3^FS\n" +
      "^CF0,24\n" +
      "^FO50,600^FDBorders, filled shapes, reverse fields^FS\n" +
      "^XZ",
    epl: "N\n" +
      "A20,24,0,3,1,1,N,\"EPL2 SHOWCASE\"\n" +
      "LO20,62,760,3\n" +
      "A20,84,0,2,1,1,N,\"Text, boxes, lines and barcodes\"\n" +
      "A20,140,0,4,1,1,N,\"Font 4 heading\"\n" +
      "A20,200,0,3,2,2,N,\"Font 3 double\"\n" +
      "A20,268,0,2,1,1,N,\"Font 2 - regular text\"\n" +
      "A20,298,0,1,1,1,N,\"Font 1 - small text\"\n" +
      "X20,350,3,780,560\n" +
      "A40,382,0,3,1,1,N,\"X box - frame between two corners\"\n" +
      "A40,436,0,2,1,1,N,\"LO lines, X boxes, barcodes below\"\n" +
      "B20,590,0,3,2,4,80,B,\"ABC-1234\"\n" +
      "A20,710,0,2,1,1,N,\"Code 39 with human readable line\"\n" +
      "B20,770,0,1,2,4,90,B,\"EPL-DEMO-2026\"\n" +
      "A20,910,0,2,1,1,N,\"Code 128 with human readable line\"\n" +
      "P1",
    tspl: "SIZE 101.6 mm,152.4 mm\n" +
      "GAP 3 mm,0 mm\n" +
      "DIRECTION 1\n" +
      "CLS\n" +
      "BOX 16,16,780,180,4\n" +
      "TEXT 40,40,\"5\",0,1,1,\"TSPL / TSC SHOWCASE\"\n" +
      "TEXT 40,120,\"3\",0,1,1,\"SIZE defines the canvas; GAP/DENSITY/SPEED are inert\"\n" +
      "BAR 40,230,740,3\n" +
      "TEXT 40,260,\"3\",0,1,1,\"BARCODE / QRCODE / DMATRIX / ITF14\"\n" +
      "BARCODE 40,320,\"128\",90,1,0,2,3,\"1Z999AA10123456784\"\n" +
      "QRCODE 560,300,M,7,A,0,\"https://github.com/GOODBOY008/labelize\"\n" +
      "DMATRIX 560,540,180,180,x5,\"FLOWSTOCK|SCANNER-DIAG|V1\"\n" +
      "BARCODE 40,480,\"ITF14\",110,1,0,3,6,\"0460123456789\"\n" +
      "BAR 40,640,740,3\n" +
      "REVERSE 40,660,520,44\n" +
      "TEXT 60,672,\"3\",0,1,1,\"REVERSE = white-on-black region\"\n" +
      "TEXT 40,760,\"3\",0,1,1,\"BAR / BOX / CIRCLE / ELLIPSE / ERASE / BITMAP\"\n" +
      "PRINT 1",
    cpcl:
      "! 0 203 203 400 1\n" +
      "PAGE-WIDTH 576\n" +
      "; CPCL showcase - Zebra mobile printers (QL/MZ series)\n" +
      "TEXT 7 0 20 30 CPCL SHOWCASE\n" +
      "SETMAG 2 2\n" +
      "TEXT 5 1 20 70 MOBILE LABEL\n" +
      "SETMAG 0 0\n" +
      "BOX 10 10 565 390 2\n" +
      "LINE 10 130 565 130 2\n" +
      "BARCODE 128 2 1 60 30 150 841234567890\n" +
      "B QR 430 150 U 5\n" +
      "MA,https://example.com/t/841234567890\n" +
      "ENDQR\n" +
      "CENTER 575\n" +
      "TEXT 4 0 0 350 PRINTED ON A ZEBRA QL SERIES\n" +
      "FORM\n" +
      "PRINT\n",
    dpl:
      "\u0002L\n" +
      "D11\n" +
      "1911A1200050010DPL SHOWCASE\n" +
      "121100106001000DATAMAX I-CLASS\n" +
      "1X1100004500000L400002\n" +
      "1E21040030002601234567890\n" +
      "1F21040055002604006381333931\n" +
      "1z00000001502600F1000000PDF417 DATA\n" +
      "1W1d4400001504000https://example.com/dpl\n\n" +
      "Q0001\n" +
      "E\n"
  };

  /* Sample language: keeps the format selector in sync when a sample loads. */
  var SAMPLE_FMT = { shipping: "zpl", barcodes: "zpl", graphics: "zpl", epl: "epl", tspl: "tspl", cpcl: "cpcl", dpl: "dpl" };

  /* Demo for each format: swapped in by the format selector while the editor
     still holds content the user never typed (see autoLoaded below). */
  var DEMOS = { zpl: SAMPLES.shipping, epl: SAMPLES.epl, tspl: SAMPLES.tspl, cpcl: SAMPLES.cpcl, dpl: SAMPLES.dpl };

  /* ── DOM refs ── */
  var btn          = document.getElementById("render-btn");
  var openFileBtn  = document.getElementById("open-file-btn");
  var fileInput    = document.getElementById("file-input");
  var input        = document.getElementById("zpl-input");
  var lineNumbers  = document.getElementById("line-numbers");
  var editorLineCount = 0;
  var caretPos     = document.getElementById("caret-pos");
  var loading      = document.getElementById("loading");
  var loadingText  = document.getElementById("loading-text");
  var emptyState   = document.getElementById("empty-state");
  var errBanner    = document.getElementById("error-banner");
  var previewImg   = document.getElementById("preview-img");
  var previewScroll= document.getElementById("preview-scroll");
  var dlBar        = document.getElementById("download-bar");
  var dlPng        = document.getElementById("dl-png");
  var dlPdf        = document.getElementById("dl-pdf");
  var copyImgBtn   = document.getElementById("copy-img-btn");
  var fmtSel       = document.getElementById("fmt");
  var sizePreset   = document.getElementById("size-preset");
  var widthInput   = document.getElementById("label-width");
  var heightInput  = document.getElementById("label-height");
  var sizeUnit     = document.getElementById("size-unit");
  var dpmmSel      = document.getElementById("dpmm");
  var statusText   = document.getElementById("status-text");
  var statusSize   = document.getElementById("status-size");
  var statusTime   = document.getElementById("status-time");
  var compareBtn     = document.getElementById("compare-btn");
  var compareNote    = document.getElementById("compare-note");
  var compareSection = document.getElementById("compare-section");
  var verdictBadge   = document.getElementById("verdict-badge");
  var verdictMeta    = document.getElementById("verdict-meta");
  var compareImgL    = document.getElementById("compare-labelary");
  var compareImgZ    = document.getElementById("compare-labelize");
  var compareImgD    = document.getElementById("compare-diff");
  var langSel      = document.getElementById("lang");
  var themeBtn     = document.getElementById("theme-btn");
  var samplesSel   = document.getElementById("samples");
  var autoChk      = document.getElementById("auto-render");
  var aaChk        = document.getElementById("antialias");
  var shareBtn     = document.getElementById("share-btn");
  var toastEl      = document.getElementById("toast");
  var zoomCtrl     = document.getElementById("zoom-ctrl");
  var zoomInBtn    = document.getElementById("zoom-in");
  var zoomOutBtn   = document.getElementById("zoom-out");
  var zoomFitBtn   = document.getElementById("zoom-fit");
  var zoomLabel    = document.getElementById("zoom-label");

  var pngBlobUrl = null;
  var lastPngBlob = null;
  var renderSeq = 0;
  var toastTimer = null;

  // Content last placed in the editor on the user's behalf (the initial demo,
  // a sample, or a format-switch demo). While the editor still holds exactly
  // this content, switching format swaps in the new format's demo; any manual
  // edit breaks the equality and the user's content is never replaced.
  var autoLoaded = null;

  /* ── Syntax check state ──
     The editor content is checked against POST /check (auto-detected
     language) after every change. While the user owns the content and has
     not pinned a format by hand, the format selector follows the detected
     language so renders always use the right parser. The pin survives
     edits — it only clears when content is replaced wholesale (sample,
     demo swap, file), which re-establishes a consistent format. */
  var syntaxEl       = document.getElementById("syntax-status");
  var formatBtn      = document.getElementById("format-btn");
  var fmtUserTouched = false;
  var lastReport     = null;
  var checkTimer     = null;
  var CHECK_DELAY    = 400;
  var LANG_NAMES     = { zpl: "ZPL", epl: "EPL", tspl: "TSPL", cpcl: "CPCL", dpl: "DPL" };

  function setStatus(text, cls) {
    statusText.textContent = text;
    statusText.className = cls || "";
  }

  function showToast(msg) {
    toastEl.textContent = msg;
    toastEl.classList.add("show");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function () { toastEl.classList.remove("show"); }, 2500);
  }

  function applyI18n() {
    document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
    document.title = t("app.title");

    var els = document.querySelectorAll(
      "[data-i18n],[data-i18n-html],[data-i18n-title],[data-i18n-aria],[data-i18n-alt]");
    for (var i = 0; i < els.length; i++) {
      var el = els[i], k;
      k = el.getAttribute("data-i18n");       if (k) el.textContent = t(k);
      k = el.getAttribute("data-i18n-html");  if (k) el.innerHTML = t(k);
      k = el.getAttribute("data-i18n-title"); if (k) el.setAttribute("title", t(k));
      k = el.getAttribute("data-i18n-aria");  if (k) el.setAttribute("aria-label", t(k));
      k = el.getAttribute("data-i18n-alt");   if (k) el.setAttribute("alt", t(k));
    }
    updateCompareState();
    updateCaret();
    applyZoom();
  }

  langSel.addEventListener("change", function () {
    lang = this.value === "zh" ? "zh" : "en";
    store.set("labelize-lang", lang);
    applyI18n();
  });

  /* ── Theme ── */
  var rootEl = document.documentElement;
  var schemeMq = window.matchMedia ? window.matchMedia("(prefers-color-scheme: light)") : null;

  themeBtn.addEventListener("click", function () {
    var next = rootEl.getAttribute("data-theme") === "light" ? "dark" : "light";
    rootEl.setAttribute("data-theme", next);
    store.set("labelize-theme", next);
  });

  // Follow OS theme changes while the user has not picked a theme manually.
  if (schemeMq) {
    var onSchemeChange = function () {
      if (store.get("labelize-theme") === null) {
        rootEl.setAttribute("data-theme", schemeMq.matches ? "light" : "dark");
      }
    };
    if (schemeMq.addEventListener) schemeMq.addEventListener("change", onSchemeChange);
    else if (schemeMq.addListener) schemeMq.addListener(onSchemeChange);
  }

  /* ── Settings ── */
  var SIZE_PRESETS = {
    "4x6":     [4,   6],
    "4x4":     [4,   4],
    "4x3":     [4,   3],
    "2x4":     [2,   4],
    "2x2":     [2,   2],
    "3.5x1.5": [3.5, 1.5]
  };

  // Keep the physical size independent of the displayed unit. Converting the
  // rounded input values back and forth would slowly change custom sizes.
  var sizeMm = { width: 101.6, height: 152.4 };
  var currentSizeUnit = "mm";

  function displaySize() {
    var scale = currentSizeUnit === "mm" ? 1 : 25.4;
    widthInput.value  = +(sizeMm.width / scale).toFixed(6);
    heightInput.value = +(sizeMm.height / scale).toFixed(6);
  }

  function validSize() {
    return Number.isFinite(sizeMm.width) && sizeMm.width > 0 &&
           Number.isFinite(sizeMm.height) && sizeMm.height > 0;
  }

  function sizeEdited() {
    var scale = currentSizeUnit === "mm" ? 1 : 25.4;
    if (this === widthInput) sizeMm.width = this.valueAsNumber * scale;
    else sizeMm.height = this.valueAsNumber * scale;
    sizePreset.value = "custom";
    scheduleAutoRender();
  }

  widthInput.addEventListener("input", sizeEdited);
  heightInput.addEventListener("input", sizeEdited);

  sizeUnit.addEventListener("change", function () {
    if (!validSize()) {
      this.value = currentSizeUnit;
      showToast(t("err.size"));
      return;
    }
    currentSizeUnit = this.value;
    displaySize();
  });

  sizePreset.addEventListener("change", function () {
    if (this.value !== "custom") {
      var wh = SIZE_PRESETS[this.value];
      sizeMm.width  = wh[0] * 25.4;
      sizeMm.height = wh[1] * 25.4;
      displaySize();
    }
    scheduleAutoRender();
  });

  function getParams() {
    if (!validSize()) return null;
    var fmtVal = fmtSel.value;
    var dpmm  = parseInt(dpmmSel.value, 10) || 8;
    var w_mm  = +sizeMm.width.toFixed(6);
    var h_mm  = +sizeMm.height.toFixed(6);
    return { fmt: fmtVal, dpmm: dpmm, w_mm: w_mm, h_mm: h_mm, aa: aaChk.checked };
  }

  function buildUrl(p, output) {
    var u = "/convert?width=" + p.w_mm + "&height=" + p.h_mm + "&dpmm=" + p.dpmm;
    if (p.aa) u += "&antialias=true";
    if (output) u += "&output=" + output;
    return u;
  }

  function ctFor(fmt) {
    if (fmt === "epl") return "application/epl";
    if (fmt === "tspl") return "application/tspl";
    if (fmt === "cpcl") return "application/cpcl";
    if (fmt === "dpl") return "application/dpl";
    return "application/zpl";
  }

  function clearPreview() {
    errBanner.classList.remove("visible");
    previewImg.classList.remove("visible");
    dlBar.classList.remove("visible");
    compareSection.classList.remove("visible");
    zoomCtrl.classList.remove("visible");
    emptyState.style.display = "none";
    if (pngBlobUrl) { URL.revokeObjectURL(pngBlobUrl); pngBlobUrl = null; }
  }

  function showError(msg) {
    clearPreview();
    emptyState.style.display = "none";
    errBanner.textContent = msg;
    errBanner.classList.add("visible");
    statusSize.textContent = "";
    statusTime.textContent = "";
    setStatus(t("status.err"), "status-err");
  }

  /* ── Zoom ── */
  var ZOOM_LEVELS = [25, 50, 75, 100, 150, 200, 300, 400];
  var zoomFit = true;
  var zoomLevel = 100;

  function applyZoom() {
    if (zoomFit) {
      var style = getComputedStyle(previewScroll);
      var availableWidth = previewScroll.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
      var availableHeight = previewScroll.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
      var scale = Math.min(1, availableWidth / previewImg.naturalWidth, availableHeight / previewImg.naturalHeight);
      previewImg.style.width = previewImg.naturalWidth ? Math.max(1, Math.floor(previewImg.naturalWidth * scale)) + "px" : "";
    } else {
      previewImg.style.width = Math.round(previewImg.naturalWidth * zoomLevel / 100) + "px";
    }
    zoomLabel.textContent = zoomFit ? t("zoom.fit") : zoomLevel + "%";
    previewScroll.classList.toggle("zoomed", !zoomFit);
  }

  function zoomStep(dir) {
    var i;
    if (zoomFit) {
      zoomFit = false;
      i = dir > 0 ? ZOOM_LEVELS.indexOf(100) : ZOOM_LEVELS.length - 1;
      zoomLevel = ZOOM_LEVELS[i < 0 ? 1 : i];
    } else {
      i = ZOOM_LEVELS.indexOf(zoomLevel);
      i = Math.min(ZOOM_LEVELS.length - 1, Math.max(0, i + dir));
      zoomLevel = ZOOM_LEVELS[i];
    }
    applyZoom();
  }

  zoomInBtn.addEventListener("click", function () { zoomStep(1); });
  zoomOutBtn.addEventListener("click", function () { zoomStep(-1); });
  zoomFitBtn.addEventListener("click", function () { zoomFit = true; applyZoom(); });
  previewImg.addEventListener("load", applyZoom);
  if (window.ResizeObserver) {
    new ResizeObserver(applyZoom).observe(previewScroll);
  } else {
    window.addEventListener("resize", applyZoom);
  }
  previewImg.addEventListener("dblclick", function () {
    if (zoomFit) { zoomFit = false; zoomLevel = 100; } else { zoomFit = true; }
    applyZoom();
  });

  /* ── Render (PNG) ──
     opts.auto: silent background re-render triggered by typing — keeps the
     last good preview, reports errors only in the status line. */
  function render(opts) {
    opts = opts || {};
    var auto = !!opts.auto;

    var zpl = input.value.trim();
    if (!zpl) { if (!auto) showError(t("err.empty")); return; }

    var params = getParams();
    if (!params) {
      if (auto) setStatus(t("err.size"), "status-err");
      else showError(t("err.size"));
      return;
    }
    if (!auto) {
      clearPreview();
      loadingText.textContent = t("loading.render");
      loading.classList.add("active");
      btn.disabled = true;
      setStatus(t("status.rendering"), "");
    }
    var mySeq = ++renderSeq;
    statusSize.textContent = "";
    statusTime.textContent = "";

    var t0 = performance.now();

    fetch(buildUrl(params, null), {
      method: "POST",
      headers: { "Content-Type": ctFor(params.fmt) },
      body: zpl
    })
    .then(function (res) {
      if (mySeq !== renderSeq) return;
      var elapsed = Math.round(performance.now() - t0);
      if (!auto) {
        loading.classList.remove("active");
        btn.disabled = false;
      }
      if (!res.ok) {
        return res.text().then(function (txt) {
          var msg = fmt(t("err.server"), { status: res.status, body: txt });
          if (auto) setStatus("\u26a0 " + firstLine(msg), "status-err");
          else showError(msg);
        });
      }
      return res.blob().then(function (blob) {
        lastPngBlob = blob;
        if (pngBlobUrl) URL.revokeObjectURL(pngBlobUrl);
        pngBlobUrl     = URL.createObjectURL(blob);
        dlPng.href     = pngBlobUrl;
        previewImg.src = pngBlobUrl;
        emptyState.style.display = "none";
        errBanner.classList.remove("visible");
        previewImg.classList.add("visible");
        dlBar.classList.add("visible");
        zoomCtrl.classList.add("visible");
        statusSize.textContent = "PNG  " + (blob.size / 1024).toFixed(1) + " KB";
        statusTime.textContent = elapsed + " ms";
        setStatus(t("status.ok"), "status-ok");
      });
    })
    .catch(function (err) {
      if (mySeq !== renderSeq) return;
      if (!auto) {
        loading.classList.remove("active");
        btn.disabled = false;
      }
      var msg = fmt(t("err.network"), { msg: err.message });
      if (auto) setStatus("\u26a0 " + firstLine(msg), "status-err");
      else showError(msg);
    });
  }

  /* ── Auto-render on typing ── */
  var AUTO_DELAY = 600;
  var autoTimer = null;

  function scheduleAutoRender() {
    clearTimeout(autoTimer);
    if (autoChk.checked) {
      autoTimer = setTimeout(function () { render({ auto: true }); }, AUTO_DELAY);
    }
  }

  autoChk.checked = store.get("labelize-auto") !== "0";
  autoChk.addEventListener("change", function () {
    clearTimeout(autoTimer);
    store.set("labelize-auto", this.checked ? "1" : "0");
    if (this.checked && input.value.trim()) render({ auto: true });
  });

  aaChk.checked = store.get("labelize-aa") === "1";
  aaChk.addEventListener("change", function () {
    store.set("labelize-aa", this.checked ? "1" : "0");
    if (input.value.trim()) render({ auto: true });
  });

  /* ── Compare with Labelary ── */
  // Verdict scale mirrors docs/DIFF_THRESHOLDS.md; the pixel rule (any channel
  // differs by more than 32) mirrors tests/common/image_compare.rs so the
  // playground number is on the same scale as the CI golden diffs.
  var VERDICTS = [
    { name: "PERFECT",  cls: "v-perfect",  max: 0,        note: "v.perfect" },
    { name: "GOOD",     cls: "v-good",     max: 1,        note: "v.good" },
    { name: "MINOR",    cls: "v-minor",    max: 5,        note: "v.minor" },
    { name: "MODERATE", cls: "v-moderate", max: 15,       note: "v.moderate" },
    { name: "HIGH",     cls: "v-high",     max: Infinity, note: "v.high" }
  ];

  function verdictFor(pct) {
    for (var i = 0; i < VERDICTS.length; i++) {
      if (pct <= VERDICTS[i].max) return VERDICTS[i];
    }
    return VERDICTS[VERDICTS.length - 1];
  }

  function decodeToCanvas(blob) {
    return createImageBitmap(blob).then(function (bmp) {
      var cv = document.createElement("canvas");
      cv.width = bmp.width;
      cv.height = bmp.height;
      cv.getContext("2d").drawImage(bmp, 0, 0);
      bmp.close();
      return cv;
    });
  }

  // Normalizes both images onto a common white canvas (Labelary is routinely
  // 1-2 px smaller than Labelize at the same nominal size), then counts pixels
  // where any channel differs by more than 32.
  function pixelCompare(aCv, bCv) {
    var W = Math.max(aCv.width, bCv.width);
    var H = Math.max(aCv.height, bCv.height);

    function onWhite(cv) {
      var c = document.createElement("canvas");
      c.width = W; c.height = H;
      var ctx = c.getContext("2d");
      ctx.fillStyle = "#fff";
      ctx.fillRect(0, 0, W, H);
      ctx.drawImage(cv, 0, 0);
      return ctx.getImageData(0, 0, W, H).data;
    }
    var da = onWhite(aCv);
    var db = onWhite(bCv);

    var diff = document.createElement("canvas");
    diff.width = W; diff.height = H;
    var dCtx = diff.getContext("2d");
    dCtx.fillStyle = "#fff";
    dCtx.fillRect(0, 0, W, H);
    var dd = dCtx.getImageData(0, 0, W, H).data;

    var diffCount = 0;
    for (var i = 0; i < da.length; i += 4) {
      var differs = Math.abs(da[i] - db[i]) > 32 ||
                    Math.abs(da[i + 1] - db[i + 1]) > 32 ||
                    Math.abs(da[i + 2] - db[i + 2]) > 32 ||
                    Math.abs(da[i + 3] - db[i + 3]) > 32;
      if (differs) {
        diffCount++;
        dd[i] = 255; dd[i + 1] = 0; dd[i + 2] = 0; dd[i + 3] = 255;
      }
    }
    dCtx.putImageData(new ImageData(dd, W, H), 0, 0);

    return {
      pct: (diffCount / (W * H)) * 100,
      diffCanvas: diff,
      aw: aCv.width, ah: aCv.height,
      bw: bCv.width, bh: bCv.height
    };
  }

  function compareWithLabelary() {
    // Disabled via LABELIZE_PLAYGROUND_LABELARY_COMPARE=false — the guard
    // holds even if the button is re-shown with devtools.
    if (!labelaryCompare) return;
    // NB: keep the name `fmtVal` — a local `var fmt` would shadow the i18n
    // `fmt()` helper inside every closure below.
    var fmtVal = fmtSel.value;
    if (fmtVal === "epl") { showError(t("err.epl")); return; }
    if (fmtVal === "tspl") { showError(t("err.tspl")); return; }
    if (fmtVal === "cpcl") { showError(t("err.cpcl")); return; }
    if (fmtVal === "dpl") { showError(t("err.dpl")); return; }
    var zpl = input.value.trim();
    if (!zpl) { showError(t("err.empty")); return; }

    var params = getParams();
    if (!params) { showError(t("err.size")); return; }
    var pxW = Math.ceil(params.w_mm * params.dpmm);
    var pxH = Math.ceil(params.h_mm * params.dpmm);
    if (pxW > 4096 || pxH > 4096) {
      showError(fmt(t("err.canvas"), { w: pxW, h: pxH }));
      return;
    }

    clearPreview();
    loadingText.textContent = t("loading.compare");
    loading.classList.add("active");
    compareBtn.disabled = true;
    setStatus(t("status.comparing"), "");
    statusSize.textContent = "";
    statusTime.textContent = "";

    var t0 = performance.now();

    // Labelary: HTTPS + CORS * (verified 2026-08-25); form-urlencoded is a
    // CORS-safelisted content type, so no preflight is triggered. Labelary does
    // not support EPL and rejects `application/epl` preflights, hence ZPL-only.
    var labelaryUrl = "https://api.labelary.com/v1/printers/" + params.dpmm +
                      "dpmm/labels/" + (params.w_mm / 25.4) + "x" + (params.h_mm / 25.4) + "/0/";

    var ours = fetch(buildUrl(params, null), {
      method: "POST",
      headers: { "Content-Type": ctFor(fmtVal) },
      body: zpl
    }).then(function (res) {
      if (!res.ok) throw new Error(fmt(t("err.renderFail"), { status: res.status }));
      return res.blob();
    });

    var theirs = fetch(labelaryUrl, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded", "Accept": "image/png" },
      body: zpl
    }).then(function (res) {
      if (!res.ok) {
        if (res.status === 429) throw new Error(t("err.rateLimit"));
        if (res.status === 404) throw new Error(t("err.l404"));
        throw new Error(fmt(t("err.lHttp"), { status: res.status }));
      }
      return res.blob();
    });

    Promise.all([ours, theirs])
      .then(function (results) {
        return Promise.all([decodeToCanvas(results[0]), decodeToCanvas(results[1])]);
      })
      .then(function (cvs) {
        var r = pixelCompare(cvs[0], cvs[1]);
        var v = verdictFor(r.pct);
        var elapsed = Math.round(performance.now() - t0);
        loading.classList.remove("active");
        compareBtn.disabled = false;

        compareImgZ.src = cvs[0].toDataURL("image/png");
        compareImgL.src = cvs[1].toDataURL("image/png");
        compareImgD.src = r.diffCanvas.toDataURL("image/png");

        verdictBadge.textContent = v.name;
        verdictBadge.className = "verdict-badge " + v.cls;
        verdictBadge.title = t(v.note);
        verdictMeta.textContent = r.pct.toFixed(2) + "%  \u00b7  Labelary " + r.bw + "\u00d7" + r.bh +
          "  \u00b7  Labelize " + r.aw + "\u00d7" + r.ah + "  \u00b7  " + elapsed + " ms  \u00b7  " + t(v.note);

        compareSection.classList.add("visible");
        statusSize.textContent = fmt(t("status.diff"), { pct: r.pct.toFixed(2) });
        statusTime.textContent = elapsed + " ms";
        setStatus(t("status.ok"), "status-ok");
      })
      .catch(function (err) {
        loading.classList.remove("active");
        compareBtn.disabled = false;
        showError(fmt(t("err.compare"), { msg: err.message }));
      });
  }

  /* ── PDF download (lazy) ── */
  dlPdf.addEventListener("click", function () {
    var zpl = input.value.trim();
    if (!zpl) return;
    var params = getParams();
    if (!params) { showError(t("err.size")); return; }
    dlPdf.classList.add("loading");
    dlPdf.disabled = true;

    fetch(buildUrl(params, "pdf"), {
      method: "POST",
      headers: { "Content-Type": ctFor(params.fmt) },
      body: zpl
    })
    .then(function (res) {
      dlPdf.classList.remove("loading");
      dlPdf.disabled = false;
      if (!res.ok) {
        return res.text().then(function (txt) {
          showError(fmt(t("err.pdf"), { status: res.status, body: txt }));
        });
      }
      return res.blob().then(function (blob) {
        var url = URL.createObjectURL(blob);
        var a   = document.createElement("a");
        a.href     = url;
        a.download = "label.pdf";
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        setTimeout(function () { URL.revokeObjectURL(url); }, 10000);
      });
    })
    .catch(function (err) {
      dlPdf.classList.remove("loading");
      dlPdf.disabled = false;
      showError(fmt(t("err.pdfNet"), { msg: err.message }));
    });
  });

  /* ── Copy PNG to clipboard ── */
  copyImgBtn.addEventListener("click", function () {
    if (!lastPngBlob) return;
    if (!(navigator.clipboard && window.ClipboardItem)) {
      showToast(fmt(t("toast.copyFail"), { msg: "Clipboard API unavailable" }));
      return;
    }
    navigator.clipboard.write([new ClipboardItem({ "image/png": lastPngBlob })])
      .then(function () { showToast(t("toast.imgCopied")); })
      .catch(function (e) { showToast(fmt(t("toast.copyFail"), { msg: e.message })); });
  });

  /* ── Share permalink (URL hash) ── */
  // Format: #f=zpl&w=4&h=6&u=in&d=8&c=b<urlsafe-base64> (or
  // c=u<uri-component> when TextEncoder is unavailable). Width/height stay
  // in inches for compatibility; `u` selects the displayed unit (default in).
  function b64encode(str) {
    var bytes = new TextEncoder().encode(str);
    var bin = "";
    for (var i = 0; i < bytes.length; i += 0x8000) {
      bin += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
    }
    return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  }

  function b64decode(b64) {
    b64 = b64.replace(/-/g, "+").replace(/_/g, "/");
    while (b64.length % 4) b64 += "=";
    var bin = atob(b64);
    var bytes = new Uint8Array(bin.length);
    for (var i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return new TextDecoder().decode(bytes);
  }

  function copyText(s) {
    if (navigator.clipboard && navigator.clipboard.writeText) return navigator.clipboard.writeText(s);
    return Promise.reject(new Error("Clipboard API unavailable"));
  }

  function loadFromHash() {
    if (!location.hash || location.hash.length < 3) return false;
    var q = {};
    location.hash.slice(1).split("&").forEach(function (kv) {
      var i = kv.indexOf("=");
      if (i > 0) q[kv.slice(0, i)] = decodeURIComponent(kv.slice(i + 1));
    });
    if (!q.c) return false;
    var code;
    try {
      if (q.c.charAt(0) === "b") code = b64decode(q.c.slice(1));
      else if (q.c.charAt(0) === "u") code = q.c.slice(1);
      else return false;
    } catch (e) { return false; }
    if (!code) return false;

    input.value = code;
    autoLoaded = null; // shared-link content belongs to the user
    if (["zpl", "epl", "tspl", "cpcl", "dpl"].indexOf(q.f) >= 0) fmtSel.value = q.f;
    if (q.d && [6, 8, 12, 24].indexOf(parseInt(q.d, 10)) >= 0) dpmmSel.value = String(parseInt(q.d, 10));
    // Permalinks without `a` predate the antialias option (default off), so
    // fall back to false rather than the viewer's localStorage: shared links
    // must fully define render settings.
    aaChk.checked = q.a === "1";
    var w = parseFloat(q.w), h = parseFloat(q.h);
    if (Number.isFinite(w) && w > 0 && Number.isFinite(h) && h > 0) {
      currentSizeUnit = q.u === "mm" ? "mm" : "in";
      sizeUnit.value = currentSizeUnit;
      sizeMm.width = w * 25.4;
      sizeMm.height = h * 25.4;
      displaySize();
      var match = null;
      Object.keys(SIZE_PRESETS).forEach(function (k) {
        if (Math.abs(SIZE_PRESETS[k][0] * 25.4 - sizeMm.width) < 0.000001 &&
            Math.abs(SIZE_PRESETS[k][1] * 25.4 - sizeMm.height) < 0.000001) match = k;
      });
      sizePreset.value = match || "custom";
    }
    return true;
  }

  shareBtn.addEventListener("click", function () {
    var code = input.value;
    if (!code.trim()) { showError(t("err.empty")); return; }
    var params = getParams();
    if (!params) { showError(t("err.size")); return; }
    var enc;
    try { enc = "b" + b64encode(code); }
    catch (e) { enc = "u" + encodeURIComponent(code); }
    var hash = "f=" + fmtSel.value + "&w=" + (sizeMm.width / 25.4) + "&h=" + (sizeMm.height / 25.4) +
               "&u=" + currentSizeUnit +
               "&d=" + dpmmSel.value + "&a=" + (aaChk.checked ? 1 : 0) + "&c=" + encodeURIComponent(enc);
    history.replaceState(null, "", "#" + hash);
    copyText(location.href)
      .then(function () { showToast(t("toast.urlCopied")); })
      .catch(function (e) { showToast(fmt(t("toast.copyFail"), { msg: e.message })); });
  });

  /* ── Samples ── */
  samplesSel.addEventListener("change", function () {
    var v = this.value;
    this.selectedIndex = 0;
    if (!v || !SAMPLES[v]) return;
    input.value = SAMPLES[v];
    autoLoaded = SAMPLES[v];
    // A sample knows its language; keep the format selector in sync.
    fmtSel.value = SAMPLE_FMT[v] || "zpl";
    fmtUserTouched = false; // content and format are consistent again
    updateCompareState();
    updateCaret();
    render();
    scheduleCheck();
  });

  /* ── Open File ── */
  openFileBtn.addEventListener("click", function () { fileInput.click(); });

  fileInput.addEventListener("change", function () {
    var file = this.files && this.files[0];
    if (!file) return;
    var ext = file.name.split(".").pop().toLowerCase();
    if (ext === "epl") fmtSel.value = "epl";
    else if (ext === "tspl") fmtSel.value = "tspl";
    else if (ext === "cpcl") fmtSel.value = "cpcl";
    else if (ext === "dpl") fmtSel.value = "dpl";
    else                   fmtSel.value = "zpl";
    fmtUserTouched = false; // content and format are consistent again
    updateCompareState();
    var reader = new FileReader();
    reader.onload = function (e) {
      input.value = e.target.result;
      autoLoaded = null; // opened file content belongs to the user
      input.focus();
      updateCaret();
      scheduleCheck();
    };
    reader.readAsText(file);
    this.value = "";
  });

  /* ── Caret indicator ── */
  function updateCaret() {
    var pos = input.selectionStart || 0;
    var upto = input.value.slice(0, pos);
    var ln = upto.split("\n").length;
    var col = pos - (upto.lastIndexOf("\n") + 1) + 1;
    caretPos.textContent = fmt(t("caret"), { ln: ln, col: col });
    var count = input.value.split("\n").length;
    if (count !== editorLineCount) {
      var numbers = [];
      for (var i = 1; i <= count; i++) numbers.push(i);
      lineNumbers.textContent = numbers.join("\n");
      editorLineCount = count;
    }
    lineNumbers.scrollTop = input.scrollTop;
  }

  input.addEventListener("scroll", function () { lineNumbers.scrollTop = input.scrollTop; });
  input.addEventListener("keyup", updateCaret);
  input.addEventListener("click", updateCaret);
  input.addEventListener("select", updateCaret);

  /* ── Syntax check ── */
  function renderSyntaxBadge() {
    if (!lastReport) {
      syntaxEl.className = "syntax-status";
      syntaxEl.textContent = "";
      syntaxEl.title = "";
      return;
    }
    var name = LANG_NAMES[lastReport.language] || "ZPL";
    syntaxEl.className = "syntax-status " + (lastReport.ok ? "syn-ok" : "syn-err");
    syntaxEl.title = lastReport.error || "";
    if (lastReport.ok) {
      var count = "";
      if (lastReport.labels > 1) count = " · " + fmt(t("syn.nLabels"), { n: lastReport.labels });
      else if (lastReport.labels === 1) count = " · " + t("syn.oneLabel");
      syntaxEl.textContent = name + " · " + t("syn.ok") + count;
    } else {
      var msg = lastReport.error || t("syn.bad");
      syntaxEl.textContent = name + " · " + t("syn.bad") + " · " + msg;
    }
  }

  function runCheck() {
    var code = input.value;
    if (!code.trim()) {
      lastReport = null;
      renderSyntaxBadge();
      return;
    }
    fetch("/validate?dpmm=" + encodeURIComponent(dpmmSel.value), {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: new TextEncoder().encode(code)
    })
      .then(function (res) { return res.json(); })
      .then(function (report) {
        if (input.value !== code) return; // content changed while in flight
        lastReport = report;
        renderSyntaxBadge();
        maybeSyncFormat(report);
      })
      .catch(function () { /* best-effort; render errors still surface in the banner */ });
  }

  function scheduleCheck() {
    clearTimeout(checkTimer);
    checkTimer = setTimeout(runCheck, CHECK_DELAY);
  }

  // While the editor holds the user's own content and they have not pinned
  // the format selector by hand, follow the detected language so the preview
  // always parses with the right engine.
  function maybeSyncFormat(report) {
    if (!LANG_NAMES[report.language]) return;
    var userOwns = autoLoaded === null || input.value !== autoLoaded;
    if (userOwns && !fmtUserTouched && fmtSel.value !== report.language) {
      fmtSel.value = report.language;
      updateCompareState();
      render();
    }
  }

  /* Format button: reflow the editor content one command per line via
     POST /format. The result is still the user's own content (just
     reflowed), so it never re-enables the demo-swap path. */
  formatBtn.addEventListener("click", function () {
    var code = input.value;
    if (!code.trim()) { showError(t("err.empty")); return; }
    fetch("/format", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: new TextEncoder().encode(code)
    })
      .then(function (res) {
        if (!res.ok) throw new Error("HTTP " + res.status);
        return res.arrayBuffer();
      })
      .then(function (buf) {
        if (input.value !== code) return; // changed while in flight
        input.value = new TextDecoder().decode(buf);
        autoLoaded = null; // formatted content belongs to the user
        updateCaret();
        render();
        scheduleCheck();
      })
      .catch(function (e) { showError(fmt(t("err.renderFail"), { status: e.message })); });
  });

  /* ── Events ── */
  btn.addEventListener("click", render);

  compareBtn.addEventListener("click", compareWithLabelary);

  // Deployment toggle: LABELIZE_PLAYGROUND_LABELARY_COMPARE=false removes the
  // button (and compareWithLabelary no-ops above) so the external POST to
  // api.labelary.com is never offered.
  if (!labelaryCompare) {
    compareBtn.style.display = "none";
  }

  // Labelary does not support EPL (404), so the compare tool is ZPL-only.
  function updateCompareState() {
    var noCompare = ["epl", "tspl", "cpcl", "dpl"].indexOf(fmtSel.value) >= 0;
    compareBtn.disabled = noCompare;
    compareNote.hidden = !noCompare;
    compareBtn.title = noCompare ? t("tip.compareEpl") : t("tip.compare");
  }
  fmtSel.addEventListener("change", function () {
    fmtUserTouched = true;
    updateCompareState();
    var demo = DEMOS[fmtSel.value];
    // The user hasn't typed anything of their own yet: swap in the demo for
    // the newly selected format so the canvas isn't drawn from the previous
    // language's data.
    if (demo && autoLoaded !== null && input.value === autoLoaded && input.value !== demo) {
      input.value = demo;
      autoLoaded = demo;
      fmtUserTouched = false; // content and format are consistent again
      updateCaret();
      render();
      scheduleCheck();
    }
  });

  input.addEventListener("input", function () {
    updateCaret();
    scheduleCheck();
    scheduleAutoRender();
  });

  input.addEventListener("keydown", function (e) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      render();
    }
  });

  // Ctrl/Cmd+S downloads the current PNG (render first if none yet).
  document.addEventListener("keydown", function (e) {
    if ((e.ctrlKey || e.metaKey) && (e.key === "s" || e.key === "S")) {
      e.preventDefault();
      if (pngBlobUrl) dlPng.click();
    }
  });

  /* ── Init ── */
  langSel.value = lang;
  applyI18n();
  autoLoaded = input.value; // the baked-in demo counts as auto-loaded
  var loadedFromHash = loadFromHash();
  updateCompareState();
  updateCaret();
  scheduleCheck();
  if (loadedFromHash) render();
})();
</script>
</body>
</html>
"##;

/// Deployment configuration for the playground page.
///
/// Read from `LABELIZE_*` environment variables by the HTTP server so Docker
/// deployments can pass them with `-e`. Every flag defaults to `true` (the
/// historical behaviour); an unset variable falls back to the default, and an
/// unparseable value warns on stderr before falling back — a typo'd flag
/// should be visible, not silently ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaygroundConfig {
    /// `LABELIZE_PLAYGROUND_ENABLED=false` makes the server API-only: `/`
    /// answers with a small endpoint listing instead of the playground page.
    /// The page itself never sees this flag — when it is set, the page is
    /// simply not served.
    pub enabled: bool,
    /// `LABELIZE_PLAYGROUND_LABELARY_COMPARE=false` removes the
    /// "Compare with Labelary" button, which otherwise POSTs the current
    /// ZPL to the external api.labelary.com service.
    pub labelary_compare: bool,
    /// Canonical public origin of the hosted playground, without a trailing
    /// slash (`LABELIZE_PUBLIC_BASE_URL`). Baked into the page's
    /// canonical/OG/JSON-LD URLs and echoed by `robots_txt`/`sitemap_xml`.
    /// After attaching a custom domain, set this so search engines
    /// consolidate on it instead of the default workers.dev origin.
    pub public_base_url: String,
}

/// The origin the page's SEO metadata ships with, matching the current
/// public Cloudflare Worker deployment.
pub const DEFAULT_PUBLIC_BASE_URL: &str = "https://labelize.764629910.workers.dev";

impl Default for PlaygroundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            labelary_compare: true,
            public_base_url: DEFAULT_PUBLIC_BASE_URL.to_string(),
        }
    }
}

impl PlaygroundConfig {
    /// Reads `LABELIZE_PLAYGROUND_ENABLED` and
    /// `LABELIZE_PLAYGROUND_LABELARY_COMPARE` from the environment.
    pub fn from_env() -> Self {
        Self {
            enabled: env_flag("LABELIZE_PLAYGROUND_ENABLED", true),
            labelary_compare: env_flag("LABELIZE_PLAYGROUND_LABELARY_COMPARE", true),
            public_base_url: env_base_url("LABELIZE_PUBLIC_BASE_URL"),
        }
    }
}

/// `LABELIZE_PUBLIC_BASE_URL`: trimmed, trailing slashes dropped, empty or
/// unset falls back to [`DEFAULT_PUBLIC_BASE_URL`].
fn env_base_url(name: &str) -> String {
    match std::env::var(name) {
        Ok(raw) => {
            let trimmed = raw.trim().trim_end_matches('/').to_string();
            if trimmed.is_empty() {
                DEFAULT_PUBLIC_BASE_URL.to_string()
            } else {
                trimmed
            }
        }
        Err(_) => DEFAULT_PUBLIC_BASE_URL.to_string(),
    }
}

fn env_flag(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(raw) => match parse_bool(&raw) {
            Some(v) => v,
            None => {
                eprintln!(
                    "labelize: ignoring invalid {}={:?} (expected true/false/1/0)",
                    name, raw
                );
                default
            }
        },
        Err(_) => default,
    }
}

/// Liberal boolean parsing for operator-supplied env values.
fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Placeholder inside `window.__LABELIZE_CONFIG__ = {…}` that [`page_html`]
/// replaces. When the page is served verbatim (npm `lz_playground_html`,
/// workers) the placeholder stays and the braces parse as an empty object,
/// leaving every feature enabled.
const CONFIG_MARKER: &str = "/*__CONFIG__*/";

/// Renders the playground page with `config` applied.
///
/// Returns the static [`PLAYGROUND_HTML`] unchanged (borrowed) unless the
/// Labelary compare is disabled or `public_base_url` differs from
/// [`DEFAULT_PUBLIC_BASE_URL`], so default deployments pay nothing.
pub fn page_html(config: &PlaygroundConfig) -> std::borrow::Cow<'static, str> {
    // `enabled` never reaches the page: the server declines to serve it.
    // The injected payload is boolean-only; if a string-valued option is
    // ever added, escape `<` as \u003c so the value cannot close the
    // script element.
    let base_changed = config.public_base_url != DEFAULT_PUBLIC_BASE_URL;
    if !base_changed && config.labelary_compare {
        return std::borrow::Cow::Borrowed(PLAYGROUND_HTML);
    }
    // NB: the marker sits *inside* the braces (`= {/*__CONFIG__*/};`), so the
    // replacement is the object body only — unreplaced it parses as `{}`.
    let mut html = if base_changed {
        PLAYGROUND_HTML.replace(DEFAULT_PUBLIC_BASE_URL, &config.public_base_url)
    } else {
        PLAYGROUND_HTML.to_string()
    };
    if !config.labelary_compare {
        html = html.replacen(CONFIG_MARKER, r#""labelaryCompare":false"#, 1);
    }
    std::borrow::Cow::Owned(html)
}

/// robots.txt for the hosted playground; `base` is the canonical origin
/// (no trailing slash).
pub fn robots_txt(base: &str) -> String {
    format!("User-agent: *\nAllow: /\n\nSitemap: {base}/sitemap.xml\n")
}

/// Single-entry sitemap pointing at the playground (the only indexable URL).
pub fn sitemap_xml(base: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>{base}/</loc>
    <changefreq>monthly</changefreq>
    <priority>1.0</priority>
  </url>
</urlset>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn marker_parses_as_empty_config_when_unreplaced() {
        // Served verbatim (npm lz_playground_html, workers), the page must
        // remain valid JS with every feature enabled by default.
        assert!(PLAYGROUND_HTML.contains("window.__LABELIZE_CONFIG__ = {/*__CONFIG__*/};"));
        assert_eq!(PLAYGROUND_HTML.matches(CONFIG_MARKER).count(), 1);
    }

    #[test]
    fn default_and_api_only_serve_the_static_page() {
        assert!(matches!(
            page_html(&PlaygroundConfig::default()),
            Cow::Borrowed(_)
        ));
        // `enabled` is server-side only; the page HTML is untouched.
        let api_only = PlaygroundConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(matches!(page_html(&api_only), Cow::Borrowed(_)));
    }

    #[test]
    fn disabled_compare_injects_the_flag_and_consumes_the_marker() {
        let html = page_html(&PlaygroundConfig {
            labelary_compare: false,
            ..Default::default()
        });
        assert!(html.contains(r#"window.__LABELIZE_CONFIG__ = {"labelaryCompare":false};"#));
        assert!(!html.contains(CONFIG_MARKER));
    }

    #[test]
    fn seo_metadata_ships_with_the_default_base_url() {
        // canonical link + og:url + JSON-LD WebApplication url
        assert_eq!(
            PLAYGROUND_HTML.matches(DEFAULT_PUBLIC_BASE_URL).count(),
            3,
            "canonical/og:url/JSON-LD must all carry the default origin"
        );
        assert!(PLAYGROUND_HTML
            .contains(r#"<link rel="canonical" href="https://labelize.764629910.workers.dev/">"#));
        assert!(PLAYGROUND_HTML.contains(r#"<script type="application/ld+json">"#));
        assert!(PLAYGROUND_HTML.contains(r#""@type": "FAQPage""#));
    }

    #[test]
    fn custom_base_url_replaces_every_default_occurrence() {
        let html = page_html(&PlaygroundConfig {
            public_base_url: "https://labelize.dev".to_string(),
            ..Default::default()
        });
        assert_eq!(html.matches("https://labelize.dev").count(), 3);
        assert!(!html.contains(DEFAULT_PUBLIC_BASE_URL));
        // default config still leaves the compare feature untouched
        assert!(html.contains(CONFIG_MARKER));
    }

    #[test]
    fn custom_base_url_and_disabled_compare_both_apply() {
        let html = page_html(&PlaygroundConfig {
            public_base_url: "https://labelize.dev".to_string(),
            labelary_compare: false,
            ..Default::default()
        });
        assert!(!html.contains(DEFAULT_PUBLIC_BASE_URL));
        assert!(html.contains(r#"{"labelaryCompare":false}"#));
    }

    #[test]
    fn robots_and_sitemap_point_at_the_base_url() {
        let robots = robots_txt("https://labelize.dev");
        assert!(robots.starts_with("User-agent: *\nAllow: /\n"));
        assert!(robots.contains("Sitemap: https://labelize.dev/sitemap.xml"));
        let sitemap = sitemap_xml("https://labelize.dev");
        assert!(sitemap.contains("<loc>https://labelize.dev/</loc>"));
    }

    #[test]
    fn env_base_url_trims_and_falls_back() {
        // Directly exercise the normalization the env reader applies.
        let normalize = |raw: &str| -> String {
            let trimmed = raw.trim().trim_end_matches('/').to_string();
            if trimmed.is_empty() {
                DEFAULT_PUBLIC_BASE_URL.to_string()
            } else {
                trimmed
            }
        };
        assert_eq!(normalize("https://labelize.dev/"), "https://labelize.dev");
        assert_eq!(
            normalize("  https://labelize.dev  "),
            "https://labelize.dev"
        );
        assert_eq!(normalize(""), DEFAULT_PUBLIC_BASE_URL);
        assert_eq!(normalize("   "), DEFAULT_PUBLIC_BASE_URL);
    }

    #[test]
    fn parse_bool_is_liberal() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool(" YES "), Some(true));
        assert_eq!(parse_bool("on"), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("Off"), Some(false));
        assert_eq!(parse_bool("no"), Some(false));
        assert_eq!(parse_bool(""), None);
        assert_eq!(parse_bool("maybe"), None);
    }
}
