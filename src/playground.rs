/// Self-contained HTML playground page served at `GET /`.
/// All HTML/CSS/JS is inlined — no external dependencies.
/// Dimensions follow Labelary convention (inches); converted to mm before
/// calling POST /convert.  Render always produces PNG; PDF is downloaded lazily.
pub const PLAYGROUND_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Labelize Playground</title>
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

  :root {
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
    --radius: 8px;
    --font-mono: "JetBrains Mono","Fira Code","Cascadia Code",Consolas,monospace;
    --font-ui: system-ui,-apple-system,"Segoe UI",sans-serif;
  }

  html, body { height: 100%; }

  body {
    background: var(--bg); color: var(--text);
    font-family: var(--font-ui); font-size: 14px;
    display: flex; flex-direction: column; overflow: hidden;
  }

  header {
    flex-shrink: 0; background: var(--surface);
    border-bottom: 1px solid var(--border);
    padding: 10px 20px; display: flex; align-items: center; gap: 14px;
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
    border-left: 1px solid var(--border); padding-left: 14px;
  }

  .header-spacer { flex: 1; }

  .badge {
    font-size: 11px; color: var(--text-dim); background: var(--surface2);
    border: 1px solid var(--border); border-radius: 4px; padding: 2px 7px;
  }

  main {
    flex: 1; display: grid; grid-template-columns: 1fr 1fr; overflow: hidden;
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

  #zpl-input {
    flex: 1; background: var(--bg); color: var(--text);
    font-family: var(--font-mono); font-size: 13px; line-height: 1.6;
    padding: 16px; border: none; outline: none; resize: none;
    tab-size: 2; overflow: auto; caret-color: var(--accent);
  }

  #zpl-input::selection { background: rgba(91,141,238,0.28); }

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

  .sg input:focus, .sg select:focus { border-color: var(--accent); }
  .sg input[type="number"] { width: 58px; }
  .size-sep { color: var(--text-dim); font-size: 13px; }
  .unit-label { font-size: 11px; color: var(--text-dim); }

  #open-file-btn {
    background: var(--surface2); color: var(--text-dim);
    border: 1px solid var(--border); border-radius: 6px;
    padding: 6px 13px; font-size: 12px; font-weight: 600;
    font-family: var(--font-ui); cursor: pointer;
    display: flex; align-items: center; gap: 6px;
    transition: border-color 0.15s, color 0.15s; white-space: nowrap;
  }

  #open-file-btn:hover { border-color: var(--accent); color: var(--accent); }

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
    background: rgba(255,255,255,0.15); border-radius: 3px; padding: 1px 4px;
  }

  /* ── Preview panel ── */
  .preview-panel {
    display: flex; flex-direction: column;
    background: var(--bg); overflow: hidden; position: relative;
  }

  #preview-scroll {
    flex: 1; overflow-y: auto;
    display: flex; flex-direction: column; align-items: center;
    padding: 20px 20px 0; gap: 16px;
  }

  .empty-state { margin: auto; text-align: center; color: var(--text-dim); }
  .empty-state svg { opacity: 0.2; margin-bottom: 12px; display: block; margin-left: auto; margin-right: auto; }
  .empty-state p { font-size: 13px; }

  #loading {
    display: none; position: absolute; inset: 0;
    background: rgba(15,15,17,0.72); align-items: center;
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
    background: rgba(224,85,85,0.10); border: 1px solid var(--error);
    border-radius: var(--radius); padding: 12px 16px;
    color: var(--error); font-size: 12px; font-family: var(--font-mono);
    word-break: break-word; white-space: pre-wrap;
  }

  #error-banner.visible { display: block; }

  #preview-img {
    display: none; max-width: 100%;
    border: 1px solid var(--border); border-radius: 4px;
    box-shadow: 0 4px 24px rgba(0,0,0,0.4); background: #fff;
  }

  #preview-img.visible { display: block; }

  /* ── Download bar ── */
  #download-bar {
    display: none; width: 100%;
    padding: 14px 0 20px; margin-top: 4px;
    border-top: 1px solid var(--border);
    gap: 10px; justify-content: center; align-items: center; flex-wrap: wrap;
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

  .mini-spinner {
    display: none; width: 13px; height: 13px;
    border: 2px solid rgba(255,255,255,0.3); border-top-color: #fff;
    border-radius: 50%; animation: spin 0.7s linear infinite;
  }

  .dl-btn-pdf.loading .mini-spinner { display: inline-block; }
  .dl-btn-pdf.loading .dl-icon-pdf  { display: none; }

  /* ── Status bar ── */
  .status-bar {
    flex-shrink: 0; background: var(--surface);
    border-top: 1px solid var(--border);
    padding: 4px 14px; font-size: 11px; color: var(--text-dim);
    display: flex; gap: 16px; align-items: center;
  }

  .status-ok  { color: var(--success); }
  .status-err { color: var(--error); }
</style>
</head>
<body>

<header>
  <div class="logo">
    <span class="logo-icon">L</span>
    <span class="logo-text">Labelize</span>
  </div>
  <span class="tagline">ZPL &amp; EPL Label Playground</span>
  <span class="header-spacer"></span>
  <span class="badge">v0.5</span>
</header>

<main>
  <!-- ── Left: Editor + Settings ── -->
  <div class="editor-panel">
    <div class="panel-header">
      <span class="dot"></span>ZPL / EPL Code
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
        <label for="fmt">Format</label>
        <select id="fmt">
          <option value="zpl" selected>ZPL</option>
          <option value="epl">EPL</option>
          <option value="tspl">TSPL</option>
        </select>
      </div>

      <div class="sg">
        <label for="size-preset">Size</label>
        <select id="size-preset">
          <option value="4x6"     selected>4 &times; 6 in</option>
          <option value="4x4">4 &times; 4 in</option>
          <option value="4x3">4 &times; 3 in</option>
          <option value="2x4">2 &times; 4 in</option>
          <option value="2x2">2 &times; 2 in</option>
          <option value="3.5x1.5">3.5 &times; 1.5 in</option>
          <option value="custom">Custom&hellip;</option>
        </select>
      </div>

      <div class="sg" id="custom-size" style="display:none">
        <label for="width-in">W</label>
        <input id="width-in" type="number" value="4" min="0.5" max="15" step="0.1">
        <span class="size-sep">&times;</span>
        <label for="height-in">H</label>
        <input id="height-in" type="number" value="6" min="0.5" max="15" step="0.1">
        <span class="unit-label">in</span>
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

      <input id="file-input" type="file" accept=".zpl,.epl,.tspl">
      <button id="open-file-btn">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
        </svg>
        Open File
      </button>

      <button id="render-btn">
        &#9654; Render <span class="shortcut">Ctrl+&#9166;</span>
      </button>
    </div>
  </div>

  <!-- ── Right: Preview ── -->
  <div class="preview-panel">
    <div class="panel-header">
      <span class="dot" style="background:var(--success)"></span>Preview
      <span id="status-text" style="margin-left:auto;font-size:11px;text-transform:none;letter-spacing:0;font-weight:400"></span>
    </div>

    <div id="loading">
      <div class="spinner"></div>
      <span class="loading-text">Rendering&hellip;</span>
    </div>

    <div id="preview-scroll">
      <div class="empty-state" id="empty-state">
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2">
          <rect x="3" y="3" width="18" height="18" rx="2"/>
          <line x1="3" y1="9" x2="21" y2="9"/>
          <line x1="9" y1="21" x2="9" y2="9"/>
        </svg>
        <p>Press <strong>Render</strong> to preview your label</p>
        <p style="font-size:11px;margin-top:6px;opacity:0.55">Ctrl+Enter</p>
      </div>

      <div id="error-banner"></div>
      <img id="preview-img" alt="Rendered label">

      <!-- Download bar — appears after a successful render -->
      <div id="download-bar">
        <a id="dl-png" class="dl-btn dl-btn-png" href="#" download="label.png">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          Download PNG
        </a>

        <button id="dl-pdf" class="dl-btn dl-btn-pdf">
          <svg class="dl-icon-pdf" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
          <span class="mini-spinner"></span>
          Download PDF
        </button>
      </div>
    </div><!-- /preview-scroll -->

    <div class="status-bar">
      <span id="status-size"></span>
      <span id="status-time"></span>
    </div>
  </div>
</main>

<script>
(function () {
  "use strict";

  var btn        = document.getElementById("render-btn");
  var openFileBtn = document.getElementById("open-file-btn");
  var fileInput   = document.getElementById("file-input");
  var input      = document.getElementById("zpl-input");
  var loading    = document.getElementById("loading");
  var emptyState = document.getElementById("empty-state");
  var errBanner  = document.getElementById("error-banner");
  var previewImg = document.getElementById("preview-img");
  var dlBar      = document.getElementById("download-bar");
  var dlPng      = document.getElementById("dl-png");
  var dlPdf      = document.getElementById("dl-pdf");
  var sizePreset = document.getElementById("size-preset");
  var customSize = document.getElementById("custom-size");
  var widthIn    = document.getElementById("width-in");
  var heightIn   = document.getElementById("height-in");
  var statusText = document.getElementById("status-text");
  var statusSize = document.getElementById("status-size");
  var statusTime = document.getElementById("status-time");

  var pngBlobUrl = null;

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
    var fmt   = document.getElementById("fmt").value;
    var dpmm  = parseInt(document.getElementById("dpmm").value, 10) || 8;
    var w_in  = parseFloat(widthIn.value)  || 4;
    var h_in  = parseFloat(heightIn.value) || 6;
    var w_mm  = +(w_in * 25.4).toFixed(2);
    var h_mm  = +(h_in * 25.4).toFixed(2);
    return { fmt: fmt, dpmm: dpmm, w_mm: w_mm, h_mm: h_mm };
  }

  function buildUrl(p, output) {
    var u = "/convert?dpmm=" + p.dpmm;
    if (p.fmt !== "tspl") u += "&width=" + p.w_mm + "&height=" + p.h_mm;
    if (output) u += "&output=" + output;
    return u;
  }

  function ctFor(fmt) {
    if (fmt === "epl") return "application/epl";
    if (fmt === "tspl") return "application/tspl";
    return "application/zpl";
  }

  function clearPreview() {
    errBanner.classList.remove("visible");
    previewImg.classList.remove("visible");
    dlBar.classList.remove("visible");
    emptyState.style.display = "none";
    if (pngBlobUrl) { URL.revokeObjectURL(pngBlobUrl); pngBlobUrl = null; }
  }

  function showError(msg) {
    clearPreview();
    errBanner.textContent = msg;
    errBanner.classList.add("visible");
    statusText.textContent = "Error";
    statusText.className = "status-err";
  }

  /* ── Render (PNG) ── */
  function render() {
    var zpl = input.value.trim();
    if (!zpl) { showError("Editor is empty — paste some ZPL, EPL, or TSPL first."); return; }

    var params = getParams();
    clearPreview();
    loading.classList.add("active");
    btn.disabled = true;
    statusText.textContent = "Rendering\u2026";
    statusText.className = "";
    statusSize.textContent = "";
    statusTime.textContent = "";

    var t0 = performance.now();

    fetch(buildUrl(params, null), {
      method: "POST",
      headers: { "Content-Type": ctFor(params.fmt) },
      body: zpl
    })
    .then(function (res) {
      var elapsed = Math.round(performance.now() - t0);
      loading.classList.remove("active");
      btn.disabled = false;
      if (!res.ok) {
        return res.text().then(function (txt) {
          showError("Server error " + res.status + ": " + txt);
        });
      }
      return res.blob().then(function (blob) {
        pngBlobUrl     = URL.createObjectURL(blob);
        dlPng.href     = pngBlobUrl;
        previewImg.src = pngBlobUrl;
        previewImg.classList.add("visible");
        dlBar.classList.add("visible");
        statusSize.textContent = "PNG  " + (blob.size / 1024).toFixed(1) + " KB";
        statusTime.textContent = elapsed + " ms";
        statusText.textContent = "OK";
        statusText.className   = "status-ok";
      });
    })
    .catch(function (err) {
      loading.classList.remove("active");
      btn.disabled = false;
      showError("Network error: " + err.message);
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
          showError("PDF error " + res.status + ": " + txt);
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
      showError("PDF download error: " + err.message);
    });
  });

  /* ── Open File ── */
  openFileBtn.addEventListener("click", function () { fileInput.click(); });

  fileInput.addEventListener("change", function () {
    var file = this.files && this.files[0];
    if (!file) return;
    var ext = file.name.split(".").pop().toLowerCase();
    var fmtSel = document.getElementById("fmt");
    if (ext === "epl") fmtSel.value = "epl";
    else if (ext === "tspl") fmtSel.value = "tspl";
    else fmtSel.value = "zpl";
    var reader = new FileReader();
    reader.onload = function (e) {
      input.value = e.target.result;
      input.focus();
    };
    reader.readAsText(file);
    this.value = "";
  });

  btn.addEventListener("click", render);

  input.addEventListener("keydown", function (e) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      render();
    }
  });
})();
</script>
</body>
</html>
"##;
