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
/// - Dimensions follow Labelary convention (inches); converted to mm before
///   calling POST /convert.  Render always produces PNG; PDF is lazy.
pub const PLAYGROUND_HTML: &str = r##"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Labelize Playground</title>
<link rel="icon" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Crect width='32' height='32' rx='7' fill='%235b8dee'/%3E%3Ctext x='16' y='22.5' font-family='sans-serif' font-weight='800' font-size='17' fill='%23fff' text-anchor='middle'%3EL%3C/text%3E%3C/svg%3E">
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
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

  :root {
    color-scheme: dark;
    --bg: #0f0f11;
    --surface: #1a1a1f;
    --surface2: #24242b;
    --border: #2e2e38;
    --accent: #5b8dee;
    --accent-hover: #7aa3f5;
    --text: #e2e2e8;
    --text-dim: #888896;
    --error: #e05555;
    --success: #4dbd74;
    --overlay: rgba(15,15,17,0.72);
    --err-bg: rgba(224,85,85,0.10);
    --sel: rgba(91,141,238,0.28);
    --chip-bg: rgba(255,255,255,0.15);
    --shadow: 0 4px 24px rgba(0,0,0,0.4);
    --verdict-ok: #4dbd74;
    --verdict-warn: #e0b84f;
    --verdict-mod: #e0803c;
    --verdict-bad: #e05555;
    --badge-fg: #0f0f11;
    --radius: 8px;
    --font-mono: "JetBrains Mono","Fira Code","Cascadia Code",Consolas,monospace;
    --font-ui: system-ui,-apple-system,"Segoe UI",sans-serif;
  }

  :root[data-theme="light"] {
    color-scheme: light;
    --bg: #f6f8fa;
    --surface: #ffffff;
    --surface2: #eef1f5;
    --border: #d8dee5;
    --accent: #2f6fdd;
    --accent-hover: #255fc2;
    --text: #1f2430;
    --text-dim: #606a78;
    --error: #cf222e;
    --success: #1a7f37;
    --overlay: rgba(246,248,250,0.78);
    --err-bg: rgba(207,34,46,0.07);
    --sel: rgba(47,111,221,0.20);
    --chip-bg: rgba(31,36,48,0.08);
    --shadow: 0 4px 16px rgba(31,36,48,0.14);
    --verdict-ok: #1a7f37;
    --verdict-warn: #9a6700;
    --verdict-mod: #bc4c00;
    --verdict-bad: #cf222e;
    --badge-fg: #ffffff;
  }

  html, body { height: 100%; }

  body {
    background: var(--bg); color: var(--text);
    font-family: var(--font-ui); font-size: 14px;
    display: flex; flex-direction: column; overflow: hidden;
  }

  :focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }

  /* ── Header ── */
  header {
    flex-shrink: 0; background: var(--surface);
    border-bottom: 1px solid var(--border);
    padding: 10px 20px; display: flex; align-items: center; gap: 12px;
  }

  .logo { display: flex; align-items: center; gap: 8px; }

  .logo-icon {
    width: 28px; height: 28px; background: var(--accent);
    border-radius: 6px; display: flex; align-items: center;
    justify-content: center; font-size: 14px; font-weight: 800; color: #fff;
  }

  .logo-text { font-size: 16px; font-weight: 700; color: var(--text); }

  .tagline {
    color: var(--text-dim); font-size: 13px;
    border-left: 1px solid var(--border); padding-left: 12px;
  }

  .header-spacer { flex: 1; }

  .hdr-select {
    background: var(--surface2); border: 1px solid var(--border);
    border-radius: 4px; color: var(--text); font-size: 12px;
    font-family: var(--font-ui); padding: 3px 6px; outline: none;
    cursor: pointer; transition: border-color 0.15s; max-width: 150px;
  }
  .hdr-select:focus { border-color: var(--accent); }

  #theme-btn {
    width: 28px; height: 28px; display: inline-flex; align-items: center;
    justify-content: center; background: var(--surface2); color: var(--text);
    border: 1px solid var(--border); border-radius: 4px; cursor: pointer;
    transition: border-color 0.15s, color 0.15s; flex-shrink: 0;
  }
  #theme-btn:hover { border-color: var(--accent); color: var(--accent); }
  #theme-btn svg { display: block; }
  #theme-btn .icon-sun { display: none; }
  :root[data-theme="dark"] #theme-btn .icon-sun  { display: block; }
  :root[data-theme="dark"] #theme-btn .icon-moon { display: none; }

  .github-link {
    font-size: 12px; color: var(--text); background: var(--surface2);
    border: 1px solid var(--border); border-radius: 4px; padding: 2px 10px;
    text-decoration: none; display: inline-flex; align-items: center; gap: 5px;
    transition: border-color 0.15s, color 0.15s, background 0.15s; white-space: nowrap;
  }

  .github-link:hover {
    color: #fff; background: var(--accent); border-color: var(--accent);
  }

  .badge {
    font-size: 11px; color: var(--text-dim); background: var(--surface2);
    border: 1px solid var(--border); border-radius: 4px; padding: 2px 7px;
    white-space: nowrap;
  }

  main {
    flex: 1; display: grid; grid-template-columns: 1fr 1fr; overflow: hidden;
    min-height: 0;
  }

  /* ── Editor panel ── */
  .editor-panel {
    display: flex; flex-direction: column;
    border-right: 1px solid var(--border); overflow: hidden;
  }

  .panel-header {
    flex-shrink: 0; padding: 7px 14px; background: var(--surface);
    border-bottom: 1px solid var(--border); font-size: 11px; font-weight: 600;
    text-transform: uppercase; letter-spacing: 0.8px; color: var(--text-dim);
    display: flex; align-items: center; gap: 6px;
  }

  .panel-header .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--accent); }

  .panel-header .hdr-side {
    margin-left: auto; font-size: 11px; text-transform: none;
    letter-spacing: 0; font-weight: 400; color: var(--text-dim);
  }

  #zpl-input {
    flex: 1; background: var(--bg); color: var(--text);
    font-family: var(--font-mono); font-size: 13px; line-height: 1.6;
    padding: 16px; border: none; outline: none; resize: none;
    tab-size: 2; overflow: auto; caret-color: var(--accent);
  }

  #zpl-input::selection { background: var(--sel); }

  /* ── Settings bar ── */
  .settings-bar {
    flex-shrink: 0; background: var(--surface);
    border-top: 1px solid var(--border);
    padding: 9px 14px; display: flex; flex-wrap: wrap; gap: 10px; align-items: center;
  }

  .sg { display: flex; align-items: center; gap: 5px; }

  .sg label {
    font-size: 11px; font-weight: 600; text-transform: uppercase;
    letter-spacing: 0.5px; color: var(--text-dim); white-space: nowrap;
  }

  .sg input, .sg select {
    background: var(--surface2); border: 1px solid var(--border);
    border-radius: 5px; color: var(--text); font-size: 12px;
    font-family: var(--font-ui); padding: 4px 7px; outline: none;
    transition: border-color 0.15s;
  }

  .sg select { cursor: pointer; }
  .sg input:focus, .sg select:focus { border-color: var(--accent); }
  .sg input[type="number"] { width: 58px; }

  .sg input[type="checkbox"] {
    width: 14px; height: 14px; accent-color: var(--accent); cursor: pointer; padding: 0;
  }

  .size-sep { color: var(--text-dim); font-size: 13px; }
  .unit-label { font-size: 11px; color: var(--text-dim); }

  .ghost-btn {
    background: var(--surface2); color: var(--text-dim);
    border: 1px solid var(--border); border-radius: 6px;
    padding: 6px 13px; font-size: 12px; font-weight: 600;
    font-family: var(--font-ui); cursor: pointer;
    display: inline-flex; align-items: center; gap: 6px;
    transition: border-color 0.15s, color 0.15s; white-space: nowrap;
  }

  .ghost-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .ghost-btn:disabled { opacity: 0.55; cursor: not-allowed; }

  #file-input { display: none; }

  #render-btn {
    margin-left: auto; background: var(--accent); color: #fff;
    border: none; border-radius: 6px; padding: 7px 18px;
    font-size: 13px; font-weight: 600; font-family: var(--font-ui);
    cursor: pointer; display: flex; align-items: center; gap: 6px;
    transition: background 0.15s, transform 0.1s; white-space: nowrap;
  }

  #render-btn:hover:not(:disabled) { background: var(--accent-hover); }
  #render-btn:active:not(:disabled) { transform: scale(0.97); }
  #render-btn:disabled { opacity: 0.55; cursor: not-allowed; }

  .shortcut {
    font-size: 10px; opacity: 0.7;
    background: var(--chip-bg); border-radius: 3px; padding: 1px 4px;
  }

  /* ── Preview panel ── */
  .preview-panel {
    display: flex; flex-direction: column;
    background: var(--bg); overflow: hidden; position: relative;
  }

  #preview-scroll {
    flex: 1; overflow: auto;
    display: flex; flex-direction: column; align-items: center;
    padding: 20px 20px 0; gap: 16px;
  }

  #preview-scroll.zoomed { align-items: flex-start; }

  .empty-state { margin: auto; text-align: center; color: var(--text-dim); }
  .empty-state svg { opacity: 0.2; margin-bottom: 12px; display: block; margin-left: auto; margin-right: auto; }
  .empty-state p { font-size: 13px; }

  /* ── Zoom control (floating, preview panel) ── */
  #zoom-ctrl {
    display: none; position: absolute; top: 10px; right: 12px; z-index: 5;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 6px; overflow: hidden; box-shadow: var(--shadow);
  }
  #zoom-ctrl.visible { display: flex; }

  #zoom-ctrl button {
    background: none; border: none; color: var(--text-dim);
    font-family: var(--font-ui); font-size: 13px; padding: 5px 9px;
    cursor: pointer; transition: color 0.15s, background 0.15s;
  }
  #zoom-ctrl button:hover { color: var(--accent); background: var(--surface2); }
  #zoom-label {
    font-size: 11px; color: var(--text-dim); padding: 5px 4px;
    display: flex; align-items: center; min-width: 44px; justify-content: center;
  }

  #loading {
    display: none; position: absolute; inset: 0;
    background: var(--overlay); align-items: center;
    justify-content: center; flex-direction: column; gap: 12px; z-index: 10;
  }

  #loading.active { display: flex; }

  .spinner {
    width: 32px; height: 32px; border: 3px solid var(--border);
    border-top-color: var(--accent); border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .loading-text { color: var(--text-dim); font-size: 13px; }

  #error-banner {
    display: none; width: 100%;
    background: var(--err-bg); border: 1px solid var(--error);
    border-radius: var(--radius); padding: 12px 16px;
    color: var(--error); font-size: 12px; font-family: var(--font-mono);
    word-break: break-word; white-space: pre-wrap;
  }

  #error-banner.visible { display: block; }

  #preview-img {
    display: none; max-width: 100%;
    border: 1px solid var(--border); border-radius: 4px;
    box-shadow: var(--shadow); background: #fff;
  }

  #preview-img.visible { display: block; }

  /* ── Toast (preview panel bottom) ── */
  #toast {
    position: absolute; left: 50%; bottom: 46px;
    transform: translateX(-50%) translateY(6px);
    background: var(--surface); color: var(--text); border: 1px solid var(--border);
    border-radius: 6px; padding: 8px 16px; font-size: 12px; font-family: var(--font-ui);
    box-shadow: var(--shadow); opacity: 0; pointer-events: none;
    transition: opacity 0.2s, transform 0.2s; z-index: 20; white-space: nowrap;
  }
  #toast.show { opacity: 1; transform: translateX(-50%) translateY(0); }

  /* ── Download bar (sticky — stays visible under a tall label) ── */
  #download-bar {
    display: none; width: 100%;
    padding: 14px 0 16px; margin-top: 4px;
    border-top: 1px solid var(--border);
    gap: 10px; justify-content: center; align-items: center; flex-wrap: wrap;
    position: sticky; bottom: 0; z-index: 6; background: var(--bg);
  }

  #download-bar.visible { display: flex; }

  .dl-btn {
    display: inline-flex; align-items: center; gap: 7px;
    padding: 8px 20px; border-radius: 6px; font-size: 13px;
    font-weight: 600; font-family: var(--font-ui);
    cursor: pointer; border: none; transition: background 0.15s, opacity 0.15s;
    text-decoration: none;
  }

  .dl-btn-png {
    background: var(--surface2); color: var(--text); border: 1px solid var(--border);
  }

  .dl-btn-png:hover { border-color: var(--accent); color: var(--accent); }

  .dl-btn-pdf { background: var(--accent); color: #fff; }
  .dl-btn-pdf:hover:not(:disabled) { background: var(--accent-hover); }
  .dl-btn-pdf:disabled { opacity: 0.55; cursor: not-allowed; }

  .icon-only { padding: 8px 11px; }

  .mini-spinner {
    display: none; width: 13px; height: 13px;
    border: 2px solid rgba(255,255,255,0.3); border-top-color: #fff;
    border-radius: 50%; animation: spin 0.7s linear infinite;
  }

  .dl-btn-pdf.loading .mini-spinner { display: inline-block; }
  .dl-btn-pdf.loading .dl-icon-pdf  { display: none; }

  /* ── Compare with Labelary ── */
  .compare-btn {
    background: var(--surface2); color: var(--text-dim);
    border: 1px solid var(--border); border-radius: 6px;
    padding: 7px 14px; font-size: 12px; font-weight: 600;
    font-family: var(--font-ui); cursor: pointer;
    display: inline-flex; align-items: center; gap: 6px;
    transition: border-color 0.15s, color 0.15s; white-space: nowrap;
  }

  .compare-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
  .compare-btn:disabled { opacity: 0.55; cursor: not-allowed; }

  #compare-section { display: none; width: 100%; }
  #compare-section.visible { display: block; }

  .verdict-row {
    display: flex; align-items: center; gap: 12px; flex-wrap: wrap;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--radius); padding: 10px 14px;
  }

  .verdict-badge {
    font-size: 12px; font-weight: 800; letter-spacing: 0.6px;
    padding: 3px 10px; border-radius: 4px; color: var(--badge-fg);
  }

  .v-perfect, .v-good  { background: var(--verdict-ok); }
  .v-minor             { background: var(--verdict-warn); }
  .v-moderate          { background: var(--verdict-mod); }
  .v-high              { background: var(--verdict-bad); }

  .verdict-meta { font-size: 12px; color: var(--text-dim); }

  .compare-grid {
    display: flex; gap: 12px; flex-wrap: wrap; margin-top: 10px;
  }

  .compare-col {
    flex: 1 1 150px; min-width: 0;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--radius); overflow: hidden;
  }

  .compare-col .col-title {
    display: block; padding: 6px 12px; font-size: 11px; font-weight: 700;
    text-transform: uppercase; letter-spacing: 0.8px; color: var(--text-dim);
    background: var(--surface2); border-bottom: 1px solid var(--border);
  }

  .compare-col img {
    display: block; width: 100%; background: #fff;
  }

  /* ── Status bar ── */
  .status-bar {
    flex-shrink: 0; background: var(--surface);
    border-top: 1px solid var(--border);
    padding: 4px 14px; font-size: 11px; color: var(--text-dim);
    display: flex; gap: 16px; align-items: center;
  }

  .status-ok  { color: var(--success); }
  .status-err { color: var(--error); }

  /* ── Responsive: stack editor above preview on narrow screens ── */
  @media (max-width: 880px) {
    header { padding: 8px 12px; gap: 8px; flex-wrap: wrap; }
    .tagline { display: none; }
    .badge { display: none; }
    main { grid-template-columns: 1fr; grid-template-rows: minmax(0, 44%) minmax(0, 56%); }
    .editor-panel { border-right: none; border-bottom: 1px solid var(--border); }
    #render-btn { margin-left: 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    *, *::before, *::after {
      transition: none !important;
      animation-duration: 0.01ms !important;
      animation-iteration-count: 1 !important;
    }
  }
</style>
</head>
<body>

<header>
  <div class="logo">
    <span class="logo-icon">L</span>
    <span class="logo-text">Labelize</span>
  </div>
  <span class="tagline" data-i18n="app.tagline">ZPL &amp; EPL Label Playground</span>
  <span class="header-spacer"></span>
  <select id="samples" class="hdr-select" aria-label="Samples" data-i18n-aria="hdr.samples">
    <option value="" selected disabled data-i18n="hdr.samples">Samples</option>
    <option value="shipping" data-i18n="sample.shipping">Shipping label</option>
    <option value="barcodes" data-i18n="sample.barcodes">Barcodes &amp; 2D</option>
    <option value="graphics" data-i18n="sample.graphics">Shapes &amp; graphics</option>
  </select>
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
  <a class="github-link" href="https://github.com/GOODBOY008/labelize" target="_blank" rel="noopener">GitHub ↗</a>
  <span class="badge">v2.0</span>
</header>

<main>
  <!-- ── Left: Editor + Settings ── -->
  <div class="editor-panel">
    <div class="panel-header">
      <span class="dot"></span><span data-i18n="panel.code">ZPL / EPL Code</span>
      <span id="caret-pos" class="hdr-side" aria-live="off"></span>
    </div>

    <textarea id="zpl-input" spellcheck="false" autocomplete="off" autocorrect="off" autocapitalize="off">^XA
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

    <div class="settings-bar">
      <div class="sg">
        <label for="fmt" data-i18n="field.format">Format</label>
        <select id="fmt">
          <option value="zpl" selected>ZPL</option>
          <option value="epl">EPL</option>
        </select>
      </div>

      <div class="sg">
        <label for="size-preset" data-i18n="field.size">Size</label>
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

      <div class="sg" id="custom-size" style="display:none">
        <label for="width-in">W</label>
        <input id="width-in" type="number" value="4" min="0.5" max="15" step="0.1">
        <span class="size-sep">&times;</span>
        <label for="height-in">H</label>
        <input id="height-in" type="number" value="6" min="0.5" max="15" step="0.1">
        <span class="unit-label" data-i18n="unit.in">in</span>
      </div>

      <div class="sg">
        <label for="dpmm">dpmm</label>
        <select id="dpmm">
          <option value="6">6</option>
          <option value="8" selected>8</option>
          <option value="12">12</option>
          <option value="24">24</option>
        </select>
      </div>

      <input id="file-input" type="file" accept=".zpl,.epl">
      <button id="open-file-btn" class="ghost-btn">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
        </svg>
        <span data-i18n="btn.open">Open File</span>
      </button>

      <button id="compare-btn" class="compare-btn">
        &#8646; <span data-i18n="btn.compare">Compare with Labelary</span>
      </button>

      <label class="sg" for="auto-render" id="auto-wrap" title="Re-render automatically as you type" data-i18n-title="tip.auto">
        <input type="checkbox" id="auto-render" checked>
        <span data-i18n="btn.auto">Auto</span>
      </label>

      <label class="sg" for="antialias" id="aa-wrap" title="Emit 8-bit grayscale output instead of 1-bit black/white" data-i18n-title="tip.aa">
        <input type="checkbox" id="antialias">
        <span data-i18n="btn.aa">Antialias</span>
      </label>

      <button id="share-btn" class="ghost-btn" title="Copy a permalink to this label" data-i18n-title="tip.share">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
          <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>
          <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>
        </svg>
        <span data-i18n="btn.share">Share</span>
      </button>

      <button id="render-btn">
        &#9654; <span data-i18n="btn.render">Render</span> <span class="shortcut">Ctrl+&#9166;</span>
      </button>
    </div>
  </div>

  <!-- ── Right: Preview ── -->
  <div class="preview-panel">
    <div class="panel-header">
      <span class="dot" style="background:var(--success)"></span><span data-i18n="panel.preview">Preview</span>
      <span id="status-text" aria-live="polite" class="hdr-side"></span>
    </div>

    <div id="zoom-ctrl">
      <button id="zoom-out" type="button" title="Zoom out" data-i18n-title="tip.zoomOut">&minus;</button>
      <span id="zoom-label">Fit</span>
      <button id="zoom-in" type="button" title="Zoom in" data-i18n-title="tip.zoomIn">+</button>
      <button id="zoom-fit" type="button" title="Fit to panel width" data-i18n-title="tip.zoomFit">&#9974;</button>
    </div>

    <div id="loading">
      <div class="spinner"></div>
      <span class="loading-text" id="loading-text" data-i18n="loading.render">Rendering&hellip;</span>
    </div>

    <div id="preview-scroll">
      <div class="empty-state" id="empty-state">
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2">
          <rect x="3" y="3" width="18" height="18" rx="2"/>
          <line x1="3" y1="9" x2="21" y2="9"/>
          <line x1="9" y1="21" x2="9" y2="9"/>
        </svg>
        <p data-i18n-html="empty.title">Press <strong>Render</strong> to preview your label</p>
        <p style="font-size:11px;margin-top:6px;opacity:0.55">Ctrl+Enter</p>
      </div>

      <div id="error-banner" role="alert"></div>
      <img id="preview-img" alt="Rendered label" data-i18n-alt="alt.preview">

      <!-- Download bar — appears after a successful render -->
      <div id="download-bar">
        <a id="dl-png" class="dl-btn dl-btn-png" href="#" download="label.png">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          <span data-i18n="dl.png">Download PNG</span>
        </a>

        <button id="copy-img-btn" class="dl-btn dl-btn-png icon-only" title="Copy PNG to clipboard"
                data-i18n-title="tip.copyImg" aria-label="Copy PNG to clipboard" data-i18n-aria="tip.copyImg">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <rect x="9" y="9" width="13" height="13" rx="2"/>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
          </svg>
        </button>

        <button id="dl-pdf" class="dl-btn dl-btn-pdf">
          <svg class="dl-icon-pdf" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          <span class="mini-spinner"></span>
          <span data-i18n="dl.pdf">Download PDF</span>
        </button>
      </div>
    <!-- Compare with Labelary — appears after a successful comparison -->
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
    </div><!-- /preview-scroll -->

    <div id="toast" role="status"></div>

    <div class="status-bar">
      <span id="status-size"></span>
      <span id="status-time"></span>
    </div>
  </div>
</main>

<script>
(function () {
  "use strict";

  /* ── Persistence (private-mode safe) ── */
  var store = {
    get: function (k) { try { return localStorage.getItem(k); } catch (e) { return null; } },
    set: function (k, v) { try { localStorage.setItem(k, v); } catch (e) {} }
  };

  /* ── i18n ──
     Static text is tagged with data-i18n / data-i18n-html / data-i18n-title /
     data-i18n-aria / data-i18n-alt and swapped by applyI18n(); dynamic strings
     go through t(key) and fmt(t(key), {placeholder: value}). */
  var I18N = {
    en: {
      "app.title": "Labelize Playground",
      "app.tagline": "ZPL & EPL Label Playground",
      "hdr.samples": "Samples",
      "hdr.lang": "Language",
      "hdr.theme": "Toggle theme",
      "panel.code": "ZPL / EPL Code",
      "panel.preview": "Preview",
      "field.format": "Format",
      "field.size": "Size",
      "unit.in": "in",
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
      "btn.open": "Open File",
      "btn.compare": "Compare with Labelary",
      "btn.render": "Render",
      "btn.auto": "Auto",
      "btn.aa": "Antialias",
      "btn.share": "Share",
      "empty.title": "Press <strong>Render</strong> to preview your label",
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
      "tip.compareEpl": "Labelary does not support EPL — ZPL only",
      "tip.auto": "Re-render automatically as you type",
      "tip.aa": "Emit 8-bit grayscale output instead of 1-bit black/white",
      "tip.share": "Copy a permalink to this label",
      "tip.copyImg": "Copy PNG to clipboard",
      "tip.zoomIn": "Zoom in",
      "tip.zoomOut": "Zoom out",
      "tip.zoomFit": "Fit to panel width",
      "zoom.fit": "Fit",
      "caret": "Ln {ln}, Col {col}",
      "toast.urlCopied": "Share link copied to clipboard",
      "toast.imgCopied": "PNG copied to clipboard",
      "toast.copyFail": "Copy failed: {msg}",
      "err.empty": "Editor is empty — paste some ZPL or EPL first.",
      "err.server": "Server error {status}: {body}",
      "err.network": "Network error: {msg}",
      "err.epl": "Labelary does not support EPL — switch to ZPL to compare.",
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
      "app.title": "Labelize 在线预览",
      "app.tagline": "ZPL 与 EPL 标签在线预览",
      "hdr.samples": "示例",
      "hdr.lang": "语言",
      "hdr.theme": "切换主题",
      "panel.code": "ZPL / EPL 代码",
      "panel.preview": "预览",
      "field.format": "格式",
      "field.size": "尺寸",
      "unit.in": "英寸",
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
      "btn.open": "打开文件",
      "btn.compare": "与 Labelary 对比",
      "btn.render": "渲染",
      "btn.auto": "自动",
      "btn.aa": "抗锯齿",
      "btn.share": "分享",
      "empty.title": "点击<strong>渲染</strong>预览你的标签",
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
      "tip.compareEpl": "Labelary 不支持 EPL — 仅限 ZPL",
      "tip.auto": "输入时自动重新渲染",
      "tip.aa": "输出 8 位灰度（替代 1 位黑白）",
      "tip.share": "复制此标签的分享链接",
      "tip.copyImg": "复制 PNG 到剪贴板",
      "tip.zoomIn": "放大",
      "tip.zoomOut": "缩小",
      "tip.zoomFit": "适应面板宽度",
      "zoom.fit": "适应",
      "caret": "行 {ln}，列 {col}",
      "toast.urlCopied": "分享链接已复制到剪贴板",
      "toast.imgCopied": "PNG 已复制到剪贴板",
      "toast.copyFail": "复制失败：{msg}",
      "err.empty": "编辑器为空 — 请先粘贴 ZPL 或 EPL 代码。",
      "err.server": "服务器错误 {status}：{body}",
      "err.network": "网络错误：{msg}",
      "err.epl": "Labelary 不支持 EPL — 请切换到 ZPL 后再对比。",
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
      "^XZ"
  };

  /* ── DOM refs ── */
  var btn          = document.getElementById("render-btn");
  var openFileBtn  = document.getElementById("open-file-btn");
  var fileInput    = document.getElementById("file-input");
  var input        = document.getElementById("zpl-input");
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
  var customSize   = document.getElementById("custom-size");
  var widthIn      = document.getElementById("width-in");
  var heightIn     = document.getElementById("height-in");
  var dpmmSel      = document.getElementById("dpmm");
  var statusText   = document.getElementById("status-text");
  var statusSize   = document.getElementById("status-size");
  var statusTime   = document.getElementById("status-time");
  var compareBtn     = document.getElementById("compare-btn");
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

  sizePreset.addEventListener("change", function () {
    var isCustom = this.value === "custom";
    customSize.style.display = isCustom ? "flex" : "none";
    if (!isCustom) {
      var wh = SIZE_PRESETS[this.value];
      widthIn.value  = wh[0];
      heightIn.value = wh[1];
    }
  });

  function getParams() {
    var fmtVal = fmtSel.value;
    var dpmm  = parseInt(dpmmSel.value, 10) || 8;
    var w_in  = parseFloat(widthIn.value)  || 4;
    var h_in  = parseFloat(heightIn.value) || 6;
    var w_mm  = +(w_in * 25.4).toFixed(2);
    var h_mm  = +(h_in * 25.4).toFixed(2);
    return { fmt: fmtVal, dpmm: dpmm, w_mm: w_mm, h_mm: h_mm, aa: aaChk.checked };
  }

  function buildUrl(p, output) {
    var u = "/convert?width=" + p.w_mm + "&height=" + p.h_mm + "&dpmm=" + p.dpmm;
    if (p.aa) u += "&antialias=true";
    if (output) u += "&output=" + output;
    return u;
  }

  function ctFor(fmt) {
    return fmt === "epl" ? "application/epl" : "application/zpl";
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
      previewImg.style.width = "";
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

  autoChk.checked = store.get("labelize-auto") !== "0";
  autoChk.addEventListener("change", function () {
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
    // NB: keep the name `fmtVal` — a local `var fmt` would shadow the i18n
    // `fmt()` helper inside every closure below.
    var fmtVal = fmtSel.value;
    if (fmtVal === "epl") { showError(t("err.epl")); return; }
    var zpl = input.value.trim();
    if (!zpl) { showError(t("err.empty")); return; }

    var params = getParams();
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
                      "dpmm/labels/" + widthIn.value + "x" + heightIn.value + "/0/";

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
  // Format: #f=zpl&w=4&h=6&d=8&c=b<urlsafe-base64>  (or c=u<uri-component>
  // when TextEncoder is unavailable).
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
    if (q.f === "epl" || q.f === "zpl") fmtSel.value = q.f;
    if (q.d && [6, 8, 12, 24].indexOf(parseInt(q.d, 10)) >= 0) dpmmSel.value = String(parseInt(q.d, 10));
    // Permalinks without `a` predate the antialias option (default off), so
    // fall back to false rather than the viewer's localStorage: shared links
    // must fully define render settings.
    aaChk.checked = q.a === "1";
    var w = parseFloat(q.w), h = parseFloat(q.h);
    if (w > 0 && h > 0) {
      widthIn.value = w;
      heightIn.value = h;
      var match = null;
      Object.keys(SIZE_PRESETS).forEach(function (k) {
        if (SIZE_PRESETS[k][0] === w && SIZE_PRESETS[k][1] === h) match = k;
      });
      sizePreset.value = match || "custom";
      customSize.style.display = match ? "none" : "flex";
    }
    return true;
  }

  shareBtn.addEventListener("click", function () {
    var code = input.value;
    if (!code.trim()) { showError(t("err.empty")); return; }
    var enc;
    try { enc = "b" + b64encode(code); }
    catch (e) { enc = "u" + encodeURIComponent(code); }
    var hash = "f=" + fmtSel.value + "&w=" + widthIn.value + "&h=" + heightIn.value +
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
    updateCaret();
    render();
  });

  /* ── Open File ── */
  openFileBtn.addEventListener("click", function () { fileInput.click(); });

  fileInput.addEventListener("change", function () {
    var file = this.files && this.files[0];
    if (!file) return;
    var ext = file.name.split(".").pop().toLowerCase();
    if (ext === "epl") fmtSel.value = "epl";
    else               fmtSel.value = "zpl";
    var reader = new FileReader();
    reader.onload = function (e) {
      input.value = e.target.result;
      input.focus();
      updateCaret();
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
  }

  input.addEventListener("keyup", updateCaret);
  input.addEventListener("click", updateCaret);
  input.addEventListener("select", updateCaret);

  /* ── Events ── */
  btn.addEventListener("click", render);

  compareBtn.addEventListener("click", compareWithLabelary);

  // Labelary does not support EPL (404), so the compare tool is ZPL-only.
  function updateCompareState() {
    var isEpl = fmtSel.value === "epl";
    compareBtn.disabled = isEpl;
    compareBtn.title = isEpl ? t("tip.compareEpl") : t("tip.compare");
  }
  fmtSel.addEventListener("change", updateCompareState);

  input.addEventListener("input", function () {
    if (!autoChk.checked) return;
    clearTimeout(autoTimer);
    autoTimer = setTimeout(function () { render({ auto: true }); }, AUTO_DELAY);
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
  var loadedFromHash = loadFromHash();
  updateCaret();
  if (loadedFromHash) render();
})();
</script>
</body>
</html>
"##;
