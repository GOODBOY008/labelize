  /* Bundled documentation: hash routing never serializes or discards the draft. */
  var docsPages = JSON.parse(document.getElementById("docs-data").textContent);
  var docsView = document.getElementById("docs-view");
  var docsArticle = document.getElementById("docs-article");
  var docsSearch = document.getElementById("docs-search");
  var docsNav = document.getElementById("docs-nav");
  var docsOutline = document.getElementById("docs-outline");
  var activeDoc = null;
  function docsText(en, zh) { return lang === "zh" ? zh : en; }
  function docsNavigation() {
    var query = docsSearch.value.trim().toLowerCase();
    var terms = query.split(/\s+/).filter(Boolean);
    docsNav.replaceChildren();
    var groups = {}, count = 0;
    docsPages.forEach(function (page) {
      var haystack = (page.title + " " + page.search).toLowerCase();
      if (!terms.every(function (term) { return haystack.indexOf(term) !== -1; })) return;
      if (!groups[page.group]) {
        var section = document.createElement("section");
        var heading = document.createElement("h2");
        heading.className = "subheader";
        heading.textContent = page.group;
        section.appendChild(heading);
        docsNav.appendChild(section);
        groups[page.group] = section;
      }
      var link = document.createElement("a");
      link.className = activeDoc && activeDoc.id === page.id ? "nav-link active bg-primary text-white" : "nav-link";
      link.href = "#docs/" + page.id;
      link.textContent = page.title;
      if (activeDoc && activeDoc.id === page.id) link.setAttribute("aria-current", "page");
      groups[page.group].appendChild(link);
      count++;
    });
    document.getElementById("docs-results").textContent = query ?
      (count ? count + docsText(" matching guides", " 篇匹配指南") : docsText("No guides found. Try size, barcode or HTTP.", "未找到指南。请尝试 size、barcode 或 HTTP。")) :
      docsPages.length + docsText(" guides · documentation in English", " 篇指南 · 文档正文为英文");
  }
  function docsRoute(focus) {
    var parts = location.hash.slice(1).split("/");
    var visible = parts[0] === "docs";
    var apiVisible = parts[0] === "api";
    docsView.hidden = !visible;
    document.getElementById("api-view").hidden = !apiVisible;
    document.getElementById("playground-view").hidden = visible || apiVisible;
    document.querySelector(".about").hidden = visible || apiVisible;
    document.getElementById("nav-docs").setAttribute("aria-current", visible ? "page" : "false");
    document.getElementById("nav-api").setAttribute("aria-current", apiVisible ? "page" : "false");
    document.getElementById("nav-playground").setAttribute("aria-current", visible || apiVisible ? "false" : "page");
    ["nav-playground", "nav-api", "nav-docs"].forEach(function (id) {
      var link = document.getElementById(id);
      var selected = link.getAttribute("aria-current") === "page";
      ["active", "bg-primary", "text-white"].forEach(function (name) { link.classList.toggle(name, selected); });
    });
    if (apiVisible) { apiConsole.show(focus); return; }
    if (!visible) { document.title = t("app.title"); if (focus) input.focus(); return; }
    var page = docsPages.find(function (p) { return p.id === (parts[1] || "getting-started"); });
    if (!page) {
      activeDoc = null;
      docsArticle.innerHTML = '<h1 tabindex="-1">Guide not found</h1><p><a href="#docs/getting-started">Open the getting started guide</a> or use search to find a topic.</p>';
      docsOutline.replaceChildren();
      document.title = "Guide not found | Labelize";
    } else {
      document.title = page.title + " | Labelize Docs";
      if (activeDoc !== page) {
        activeDoc = page;
        // HTML is compiled from repository Markdown, never user input or network data.
        docsArticle.innerHTML = page.html;
        docsArticle.querySelectorAll("table").forEach(function (table) { table.classList.add("table", "table-vcenter"); });
        docsArticle.querySelectorAll(".doc-try").forEach(function (button) { button.classList.add("btn", "btn-primary", "btn-sm"); });
        docsArticle.querySelector("h1").tabIndex = -1;
        docsArticle.querySelectorAll("pre").forEach(function (pre) {
          var button = document.createElement("button");
          button.type = "button";
          button.className = "doc-copy btn btn-outline-secondary btn-sm";
          button.textContent = docsText("Copy code", "复制代码");
          button.addEventListener("click", function () {
            copyText(pre.textContent).then(function () {
              document.getElementById("docs-message").textContent = docsText("Code copied.", "代码已复制。");
            }).catch(function () {
              document.getElementById("docs-message").textContent = docsText("Copy unavailable. Select the code and copy it manually.", "无法自动复制，请选中代码手动复制。");
            });
          });
          pre.before(button);
        });
        var source = document.createElement("a");
        source.className = "docs-source";
        source.href = "https://github.com/GOODBOY008/labelize/blob/main/docs/" + page.file;
        source.textContent = docsText("View Markdown source on GitHub ↗", "在 GitHub 查看 Markdown 源文件 ↗");
        source.target = "_blank";
        source.rel = "noopener";
        docsArticle.appendChild(source);
        docsOutline.replaceChildren();
        var title = document.createElement("h2");
        title.className = "subheader";
        title.textContent = docsText("On this page", "本页内容");
        docsOutline.appendChild(title);
        page.headings.forEach(function (heading) {
          if (heading[0] !== "2") return;
          var link = document.createElement("a");
          link.className = "nav-link";
          link.href = "#docs/" + page.id + "/" + heading[1];
          link.textContent = heading[2];
          docsOutline.appendChild(link);
        });
        document.getElementById("docs-message").textContent = "";
      }
      docsArticle.querySelectorAll(".doc-try").forEach(function (b) { b.textContent = docsText("Try in playground", "在 Playground 中运行"); });
    }
    docsNavigation();
    var anchor;
    try { anchor = decodeURIComponent(parts.slice(2).join("/")); } catch (e) { anchor = ""; }
    var target = anchor ? Array.from(docsArticle.querySelectorAll("[id]")).find(function (el) { return el.id === anchor; }) : null;
    if (focus) {
      var heading = target || docsArticle.querySelector("h1");
      heading.tabIndex = -1;
      heading.focus({ preventScroll: true });
      if (target) target.scrollIntoView({ block: "start" });
      else { docsView.scrollTop = 0; window.scrollTo(0, 0); }
    }
  }
  docsSearch.addEventListener("input", docsNavigation);
  docsArticle.addEventListener("click", function (event) {
    var button = event.target.closest(".doc-try");
    if (!button) return;
    input.value = button.parentElement.querySelector("code").textContent;
    fmtSel.value = button.dataset.language;
    dpmmSel.value = button.dataset.dpmm;
    sizeMm.width = Number(button.dataset.width);
    sizeMm.height = Number(button.dataset.height);
    currentSizeUnit = "mm";
    sizeUnit.value = "mm";
    sizePreset.value = "custom";
    displaySize();
    outputMode.value = "raster";
    outputScale.value = "1";
    outputScale.disabled = true;
    allPdfPages.checked = false;
    allPdfPages.disabled = true;
    aaChk.checked = false;
    autoLoaded = null;
    fmtUserTouched = true;
    updateCompareState();
    updateCaret();
    scheduleCheck();
    location.hash = "playground";
    render();
  });
  window.addEventListener("hashchange", function () { docsRoute(true); });
  langSel.addEventListener("change", function () { activeDoc = null; docsRoute(false); });
  docsRoute(location.hash.indexOf("#docs") === 0 || location.hash === "#api");
