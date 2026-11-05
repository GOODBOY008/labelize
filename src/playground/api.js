  Object.assign(I18N.en, {
    "nav.api": "API", "api.eyebrow": "Developer tools", "api.title": "API console",
    "api.intro": "Test a request, inspect the result, then use the same call in your script.",
    "api.guide": "API guide & automation", "api.spec": "OpenAPI JSON ↓", "api.request": "Request",
    "api.endpoint": "Endpoint", "api.input": "Input", "api.text": "Text", "api.file": "File (raw bytes)",
    "api.body": "Request body", "api.useSource": "Use playground source", "api.chooseFile": "Choose a file to send unchanged",
    "api.raw": "The body is sent as raw bytes, without JSON or a multipart wrapper.",
    "api.send": "Send request", "api.cancel": "Cancel", "api.access": "Public API · No signup", "api.url": "Request URL",
    "api.response": "Response", "api.download": "Download response ↓", "api.ready": "Ready when you are. Send a request to see the response.",
    "api.status": "HTTP status", "api.elapsed": "Elapsed", "api.bytes": "Size", "api.headers": "Response headers", "api.image": "API PNG response",
    "api.examples": "Take it to your script", "api.examplesHelp": "Examples follow the request above. JavaScript uses Node.js 20+; Python uses the standard library.",
    "api.exampleLanguage": "Example language", "api.copy": "Copy example", "api.default": "Detect automatically",
    "api.sending": "Sending request…", "api.complete": "Response received.", "api.cancelled": "Request cancelled.",
    "api.networkError": "Request failed: ", "api.validationFailed": "Validation failed: ", "api.fileRequired": "Choose a file before sending.",
    "api.pdfPreview": "PDF preview", "api.openPdf": "Open PDF in new tab ↗",
    "api.pdfFallback": "If the preview is unavailable, open the PDF in a new tab or download it.",
    "api.binaryHint": "Binary response · hexadecimal preview. Download to keep the original bytes.",
    "api.truncated": "Preview shortened; the download contains the full response.", "api.copied": "Example copied.",
    "api.copyFailed": "Copy unavailable. Select the example and copy it manually.", "api.sourceCopied": "Playground source and matching settings copied.",
    "api.param.width": "Width (mm)", "api.param.height": "Height (mm)", "api.param.dpmm": "Resolution (dpmm)",
    "api.param.output": "Output", "api.param.mode": "Rendering mode", "api.param.antialias": "Antialias",
    "api.param.scale": "PNG scale", "api.param.pages": "PDF pages", "api.param.language": "Language", "api.param.threshold": "Threshold"
  });
  Object.assign(I18N.zh, {
    "nav.api": "API", "api.eyebrow": "开发者工具", "api.title": "API 调试台",
    "api.intro": "发送请求、查看响应，然后把同样的调用用于自己的脚本。",
    "api.guide": "API 文档与自动转换", "api.spec": "OpenAPI JSON ↓", "api.request": "请求",
    "api.endpoint": "接口", "api.input": "输入方式", "api.text": "文本", "api.file": "文件（原始字节）",
    "api.body": "请求正文", "api.useSource": "使用 Playground 源码", "api.chooseFile": "选择文件，按原始字节发送",
    "api.raw": "正文按原始字节发送，不包装为 JSON 或表单。",
    "api.send": "发送请求", "api.cancel": "取消", "api.access": "公开 API · 无需注册", "api.url": "请求地址",
    "api.response": "响应", "api.download": "下载响应 ↓", "api.ready": "发送请求后，即可在此查看响应。",
    "api.status": "HTTP 状态", "api.elapsed": "耗时", "api.bytes": "大小", "api.headers": "响应头", "api.image": "API 返回的 PNG 图片",
    "api.examples": "在脚本中调用", "api.examplesHelp": "示例跟随上方请求更新。JavaScript 使用 Node.js 20+；Python 只需标准库。",
    "api.exampleLanguage": "示例语言", "api.copy": "复制示例", "api.default": "自动检测",
    "api.sending": "正在发送请求…", "api.complete": "已收到响应。", "api.cancelled": "请求已取消。",
    "api.networkError": "请求失败：", "api.validationFailed": "校验未通过：", "api.fileRequired": "请先选择文件。",
    "api.pdfPreview": "PDF 预览", "api.openPdf": "在新标签页打开 PDF ↗",
    "api.pdfFallback": "如果无法显示预览，请在新标签页打开或下载 PDF。",
    "api.binaryHint": "二进制响应 · 下方为十六进制预览，下载可保留原始字节。",
    "api.truncated": "预览已截短；下载文件包含完整响应。", "api.copied": "示例已复制。",
    "api.copyFailed": "无法自动复制，请选中示例手动复制。", "api.sourceCopied": "已复制 Playground 源码与对应设置。",
    "api.param.width": "宽度（mm）", "api.param.height": "高度（mm）", "api.param.dpmm": "分辨率（dpmm）",
    "api.param.output": "输出格式", "api.param.mode": "渲染模式", "api.param.antialias": "抗锯齿",
    "api.param.scale": "PNG 倍率", "api.param.pages": "PDF 页数", "api.param.language": "语言", "api.param.threshold": "阈值"
  });

  function initApiConsole() {
    var spec = JSON.parse(document.getElementById("api-data").textContent);
    var form = document.getElementById("api-form");
    var endpoint = document.getElementById("api-endpoint");
    var parameters = document.getElementById("api-parameters");
    var contentType = document.getElementById("api-content-type");
    var bodyMode = document.getElementById("api-body-mode");
    var body = document.getElementById("api-body");
    var file = document.getElementById("api-file");
    var send = document.getElementById("api-send"), cancel = document.getElementById("api-cancel");
    var message = document.getElementById("api-message");
    var download = document.getElementById("api-download"), image = document.getElementById("api-response-image");
    var snippetLanguage = document.getElementById("api-example-language"), snippet = document.getElementById("api-example-code");
    var currentPath = null, drafts = {}, controls = {}, pending = null, responseUrl = null, examples = null, requestEdited = false;
    Object.keys(spec.paths).forEach(function (path) {
      var option = document.createElement("option"), operation = apiOperation(spec, path);
      option.value = path;
      option.textContent = operation.method + " " + path;
      endpoint.appendChild(option);
    });
    function setMessage(text, error) {
      message.textContent = text;
      message.dataset.error = error ? "true" : "false";
      // A response message has runtime content; language changes must not
      // replace an error with the initial ready message.
      message.removeAttribute("data-i18n");
    }
    function parameterValues() {
      var values = {};
      Object.keys(controls).forEach(function (name) { values[name] = controls[name].value; });
      return values;
    }
    function snapshot() {
      return apiBuildRequest(spec, { path: endpoint.value, parameters: parameterValues(), contentType: contentType.value,
        bodyMode: bodyMode.value, text: body.value, file: file.files[0] }, location.origin);
    }
    function update() {
      var mode = controls.mode && controls.mode.value, output = controls.output && controls.output.value;
      if (controls.scale) controls.scale.disabled = mode !== "vector" || output !== "png";
      if (controls.pages) controls.pages.disabled = mode !== "vector" || output !== "pdf";
      var hasBody = !!apiOperation(spec, endpoint.value).definition.requestBody;
      document.getElementById("api-body-section").hidden = !hasBody;
      bodyMode.querySelector('option[value="text"]').disabled = endpoint.value === "/encode";
      if (endpoint.value === "/encode") bodyMode.value = "file";
      body.hidden = bodyMode.value !== "text";
      document.getElementById("api-file-section").hidden = bodyMode.value !== "file";
      document.getElementById("api-use-source").disabled = endpoint.value === "/encode";
      file.accept = endpoint.value === "/encode" ? "image/*" : ".zpl,.epl,.tspl,.cpcl,.dpl,.txt,.bin";
      try {
        var request = snapshot();
        document.getElementById("api-request-url").textContent = request.method + " " + request.url;
        examples = apiExamples(request);
        snippet.textContent = examples[snippetLanguage.value];
      } catch (error) {
        examples = null;
        document.getElementById("api-request-url").textContent = error.message;
        snippet.textContent = error.message;
      }
      document.getElementById("api-copy").disabled = !examples;
    }
    function selectEndpoint() {
      if (currentPath) drafts[currentPath] = { values: parameterValues(), contentType: contentType.value, bodyMode: bodyMode.value };
      currentPath = endpoint.value;
      var operation = apiOperation(spec, currentPath).definition, saved = requestEdited ? drafts[currentPath] : null;
      var defaults = requestEdited ? {} : apiPlaygroundParameters(getParams(), fmtSel.value);
      controls = {};
      parameters.replaceChildren();
      (operation.parameters || []).forEach(function (item) {
        var parameter = apiResolve(spec, item), schema = parameter.schema;
        var group = document.createElement("div"), label = document.createElement("label");
        group.className = "api-parameter";
        label.htmlFor = "api-param-" + parameter.name;
        label.textContent = t("api.param." + parameter.name);
        group.appendChild(label);
        var control;
        if (schema.enum || schema.type === "boolean") {
          control = document.createElement("select");
          var choices = schema.enum || ["false", "true"];
          if (schema.default == null) choices = [""].concat(choices);
          choices.forEach(function (value) {
            var option = document.createElement("option");
            option.value = value;
            option.textContent = value === "" ? t("api.default") : value;
            option.disabled = parameter.name === "mode" && value === "vector" && !vectorEnabled;
            control.appendChild(option);
          });
        } else {
          control = document.createElement("input");
          control.type = "number";
          control.step = schema.type === "integer" ? "1" : "any";
          if (schema.minimum != null) control.min = schema.minimum;
          if (schema.maximum != null) control.max = schema.maximum;
        }
        control.id = label.htmlFor;
        control.title = parameter.description || "";
        var initial = defaults[parameter.name];
        // Encoding only accepts ZPL/TSPL targets, even when the playground
        // currently contains another printer language.
        if (schema.enum && !schema.enum.includes(String(initial))) initial = null;
        control.value = saved ? saved.values[parameter.name] : initial != null ? String(initial) : parameter.example != null ? parameter.example : schema.default != null ? String(schema.default) : "";
        controls[parameter.name] = control;
        group.appendChild(control);
        parameters.appendChild(group);
      });
      document.getElementById("api-description").textContent = operation.description;
      contentType.replaceChildren();
      if (operation.requestBody) {
        Object.keys(apiResolve(spec, operation.requestBody).content).forEach(function (type) {
          var option = document.createElement("option");
          option.value = type;
          option.textContent = type;
          contentType.appendChild(option);
        });
        if (saved) contentType.value = saved.contentType;
        else if (!requestEdited && currentPath !== "/encode") contentType.value = ctFor(fmtSel.value);
      }
      if (!requestEdited && currentPath !== "/encode") body.value = input.value;
      bodyMode.value = saved ? saved.bodyMode : currentPath === "/encode" ? "file" : "text";
      update();
    }
    function clearResponse() {
      image.removeAttribute("src");
      document.getElementById("api-response-pdf").removeAttribute("src");
      document.getElementById("api-open-pdf").removeAttribute("href");
      download.removeAttribute("href");
      if (responseUrl) URL.revokeObjectURL(responseUrl);
      responseUrl = null;
      ["api-response-meta", "api-headers-section", "api-response-body", "api-image-frame", "api-pdf-frame", "api-download"].forEach(function (id) { document.getElementById(id).hidden = true; });
    }
    async function submit(event) {
      event.preventDefault();
      if (pending) return;
      var request;
      try {
        request = snapshot();
        if (request.bodyMode === "file" && !request.file) throw new Error(t("api.fileRequired"));
      } catch (error) { setMessage(error.message, true); return; }
      var controller = new AbortController();
      pending = controller;
      send.disabled = true;
      cancel.hidden = false;
      clearResponse();
      setMessage(t("api.sending"));
      document.getElementById("api-response-request").textContent = request.method + " " + request.url;
      var start = performance.now();
      try {
        var options = { method: request.method, credentials: "omit", signal: controller.signal };
        if (request.bodyMode !== "none") { options.headers = { "Content-Type": request.contentType }; options.body = request.body; }
        var response = await fetch(request.url, options);
        var bytes = new Uint8Array(await response.arrayBuffer());
        // Cancel can arrive after headers or while the body is being read.
        if (controller.signal.aborted) throw new DOMException("Aborted", "AbortError");
        document.getElementById("api-status").textContent = response.status + (response.statusText ? " " + response.statusText : "");
        document.getElementById("api-elapsed").textContent = Math.round(performance.now() - start) + " ms";
        document.getElementById("api-bytes").textContent = bytes.length.toLocaleString() + " B";
        document.getElementById("api-response-meta").hidden = false;
        document.getElementById("api-response-headers").textContent = Array.from(response.headers, function (entry) { return entry[0] + ": " + entry[1]; }).join("\n");
        document.getElementById("api-headers-section").hidden = false;
        var type = (response.headers.get("content-type") || "application/octet-stream").toLowerCase();
        responseUrl = URL.createObjectURL(new Blob([bytes], { type: type }));
        download.href = responseUrl;
        download.download = response.ok ? apiOutputName(request) : "response.txt";
        download.hidden = false;
        var preview = apiPreview(bytes, type);
        if (!response.ok && (preview.kind === "image" || preview.kind === "pdf")) preview = apiPreview(bytes, "application/octet-stream");
        if (preview.kind === "image" && response.ok) {
          image.src = responseUrl;
          document.getElementById("api-image-frame").hidden = false;
        } else if (preview.kind === "pdf" && response.ok) {
          document.getElementById("api-response-pdf").src = responseUrl;
          document.getElementById("api-open-pdf").href = responseUrl;
          document.getElementById("api-pdf-frame").hidden = false;
        } else {
          var output = document.getElementById("api-response-body");
          output.textContent = preview.text || "";
          if (preview.kind === "binary") output.textContent = t("api.binaryHint") + "\n\n" + output.textContent;
          if (preview.truncated) output.textContent += "\n\n" + t("api.truncated");
          output.hidden = false;
        }
        var failed = !response.ok, resultMessage = failed ? "HTTP " + response.status : t("api.complete");
        if (request.path === "/validate" && response.ok) {
          try {
            var report = JSON.parse(new TextDecoder().decode(bytes));
            if (report.ok === false) { failed = true; resultMessage = t("api.validationFailed") + report.error; }
          } catch (error) { /* The raw response remains inspectable. */ }
        }
        setMessage(resultMessage, failed);
      } catch (error) {
        setMessage(controller.signal.aborted ? t("api.cancelled") : t("api.networkError") + error.message, !controller.signal.aborted);
      } finally {
        pending = null;
        send.disabled = false;
        cancel.hidden = true;
      }
    }
    endpoint.addEventListener("change", selectEndpoint);
    function editRequest(event) {
      if (event.target !== endpoint) requestEdited = true;
      update();
    }
    form.addEventListener("input", editRequest);
    form.addEventListener("change", editRequest);
    form.addEventListener("submit", submit);
    cancel.addEventListener("click", function () { if (pending) pending.abort(); });
    snippetLanguage.addEventListener("change", update);
    document.getElementById("api-copy").addEventListener("click", function () {
      if (examples) copyText(examples[snippetLanguage.value]).then(function () { setMessage(t("api.copied")); }).catch(function () { setMessage(t("api.copyFailed"), true); });
    });
    document.getElementById("api-use-source").addEventListener("click", function () {
      if (endpoint.value === "/encode") return;
      requestEdited = true;
      body.value = input.value;
      bodyMode.value = "text";
      contentType.value = ctFor(fmtSel.value);
      var values = apiPlaygroundParameters(getParams(), fmtSel.value);
      Object.keys(values).forEach(function (name) { if (controls[name]) controls[name].value = values[name]; });
      update();
      setMessage(t("api.sourceCopied"));
    });
    langSel.addEventListener("change", selectEndpoint);
    window.addEventListener("beforeunload", function () { if (responseUrl) URL.revokeObjectURL(responseUrl); });
    selectEndpoint();
    return { show: function (focus) {
      // Routing runs after shared-link restoration. Refresh untouched defaults
      // here so opening API after editing the playground uses the same label.
      if (!requestEdited) selectEndpoint();
      document.title = t("api.title") + " | Labelize";
      if (focus) document.getElementById("api-title").focus({ preventScroll: true });
    } };
  }
  var apiConsole = initApiConsole();
