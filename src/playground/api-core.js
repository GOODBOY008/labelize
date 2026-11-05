  // Request and example generation share a snapshot, so edited controls cannot
  // change an in-flight request or the filename of its response.
  function apiResolve(spec, value) {
    return value.$ref ? value.$ref.slice(2).split("/").reduce(function (node, key) { return node[key]; }, spec) : value;
  }
  function apiOperation(spec, path) {
    var item = spec.paths[path];
    if (!item) throw new Error("Unknown endpoint");
    return { method: item.post ? "POST" : "GET", definition: item.post || item.get };
  }
  function apiPlaygroundParameters(params, language) {
    if (!params) return { language: language };
    return { width: params.w_mm, height: params.h_mm, dpmm: params.dpmm, mode: params.mode,
      antialias: String(params.aa), scale: params.scale, pages: params.allPages ? "all" : "first", language: params.fmt };
  }
  function apiBuildRequest(spec, state, origin) {
    var operation = apiOperation(spec, state.path);
    var url = new URL(state.path, origin);
    var values = Object.assign({}, state.parameters);
    var mode = values.mode || "raster", output = values.output || "png";
    (operation.definition.parameters || []).forEach(function (item) {
      var parameter = apiResolve(spec, item), schema = parameter.schema;
      var raw = values[parameter.name];
      if (raw == null || raw === "") return;
      if (state.path === "/convert" && ((parameter.name === "scale" && (mode !== "vector" || output !== "png")) ||
          (parameter.name === "pages" && (mode !== "vector" || output !== "pdf")))) return;
      raw = String(raw);
      var number = Number(raw);
      if ((schema.type === "number" || schema.type === "integer") &&
          (raw.trim() !== raw || !Number.isFinite(number) || !/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(raw) ||
           (schema.type === "integer" && (!Number.isSafeInteger(number) || !/^[+-]?\d+$/.test(raw))) ||
           (schema.minimum != null && (schema.exclusiveMinimum ? number <= schema.minimum : number < schema.minimum)) ||
           (schema.maximum != null && number > schema.maximum))) throw new Error("Invalid " + parameter.name);
      if (schema.enum && !schema.enum.includes(raw)) throw new Error("Invalid " + parameter.name);
      if (schema.type === "boolean" && !["true", "false"].includes(raw)) throw new Error("Invalid " + parameter.name);
      url.searchParams.set(parameter.name, raw);
    });
    var request = { path: state.path, method: operation.method, url: url.href, parameters: values, bodyMode: "none" };
    if (operation.definition.requestBody) {
      var body = apiResolve(spec, operation.definition.requestBody);
      request.contentType = state.contentType || Object.keys(body.content)[0];
      if (!body.content[request.contentType]) throw new Error("Invalid Content-Type");
      request.bodyMode = state.path === "/encode" ? "file" : state.bodyMode || "text";
      request.file = state.file || null;
      request.filename = request.file ? request.file.name : (state.path === "/encode" ? "input.png" : "input." + (values.language || request.contentType.split("/")[1]).replace("octet-stream", "bin"));
      request.body = request.bodyMode === "file" ? request.file : String(state.text || "");
    }
    return request;
  }
  function apiOutputName(request) {
    if (request.path === "/convert") return "label." + (request.parameters.output || "png");
    if (request.path === "/encode") return "image." + (request.parameters.language || "zpl");
    if (request.path === "/format") {
      var language = request.parameters.language || (request.filename || "").split(".").pop();
      return "formatted." + (["zpl", "epl", "tspl", "cpcl", "dpl"].includes(language) ? language : "txt");
    }
    return "response.json";
  }
  function apiShellQuote(value) { return "'" + String(value).replace(/'/g, "'\"'\"'") + "'"; }
  function apiBase64(value) {
    var bytes = new TextEncoder().encode(value), binary = "";
    bytes.forEach(function (byte) { binary += String.fromCharCode(byte); });
    return btoa(binary);
  }
  function apiExamples(request) {
    var hasBody = request.bodyMode !== "none", fromFile = request.bodyMode === "file";
    var filename = "./" + request.filename, output = apiOutputName(request);
    var json = request.path === "/validate" || request.path === "/health";
    var headers = hasBody ? { "Content-Type": request.contentType } : {};
    var continuation = " \\" + "\n  ";
    var curl = "curl --fail-with-body --max-time 30 " + apiShellQuote(request.url);
    if (hasBody) {
      curl += continuation + "-H " + apiShellQuote("Content-Type: " + request.contentType);
      if (fromFile) curl += continuation + "--data-binary " + apiShellQuote("@" + filename);
      else {
        // A pipe preserves a leading @, CR/LF and shell metacharacters. NUL
        // cannot be an argv byte, so only that case needs base64 transport.
        var nul = request.body.includes("\0");
        curl = "printf '%s' " + apiShellQuote(nul ? apiBase64(request.body) : request.body) +
          (nul ? " | base64 -d" : "") + " |" + continuation + curl + continuation + "--data-binary @-";
      }
      if (!json) curl += continuation + "-o " + apiShellQuote(output);
    }
    var js = 'import { readFile, writeFile } from "node:fs/promises";\n\n' +
      "const response = await fetch(" + JSON.stringify(request.url) + ", {\n  method: " + JSON.stringify(request.method) + ",\n";
    if (hasBody) js += "  headers: " + JSON.stringify(headers) + ",\n  body: " +
      (fromFile ? "await readFile(" + JSON.stringify(filename) + ")" : JSON.stringify(request.body)) + ",\n";
    js += "  signal: AbortSignal.timeout(30000),\n});\n" +
      'if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`);\n';
    if (json) {
      js += "const report = await response.json();\n";
      if (request.path === "/validate") js += 'if (!report.ok) throw new Error(report.error || "Validation failed");\n';
      js += "console.log(report);";
    } else js += "await writeFile(" + JSON.stringify(output) + ", new Uint8Array(await response.arrayBuffer()));";
    var python = "import json\nfrom pathlib import Path\nfrom urllib.error import HTTPError\nfrom urllib.request import Request, urlopen\n\n" +
      "request = Request(\n    " + JSON.stringify(request.url) + ",\n    method=" + JSON.stringify(request.method) + ",\n";
    if (hasBody) python += "    headers=" + JSON.stringify(headers) + ",\n    data=" +
      (fromFile ? "Path(" + JSON.stringify(filename) + ").read_bytes()" : JSON.stringify(new TextDecoder().decode(new TextEncoder().encode(request.body))) + '.encode("utf-8")') + ",\n";
    python += ")\ntry:\n    with urlopen(request, timeout=30) as response:\n        body = response.read()\nexcept HTTPError as error:\n" +
      '    raise SystemExit(f"HTTP {error.code}: {error.read().decode(\'utf-8\', \'replace\')}")\n';
    if (json) {
      python += "report = json.loads(body)\n";
      if (request.path === "/validate") python += 'if not report["ok"]:\n    raise SystemExit(report.get("error", "Validation failed"))\n';
      python += "print(json.dumps(report, indent=2))";
    } else python += "Path(" + JSON.stringify(output) + ").write_bytes(body)";
    return { curl: curl, javascript: js, python: python };
  }
  function apiPreview(bytes, contentType) {
    if (contentType.startsWith("image/png")) return { kind: "image" };
    if (contentType.startsWith("application/pdf")) return { kind: "pdf" };
    var limit = 65536, text;
    if (/^(text\/|application\/json)/.test(contentType)) {
      try {
        text = new TextDecoder("utf-8", { fatal: true }).decode(bytes.subarray(0, limit), { stream: bytes.length > limit });
        if (/[\x00-\x08\x0b\x0c\x0e-\x1f]/.test(text)) throw new Error("binary payload");
        if (contentType.startsWith("application/json") && bytes.length <= limit) {
          try { text = JSON.stringify(JSON.parse(text), null, 2); } catch (e) { /* Show malformed JSON verbatim. */ }
        }
        return { kind: "text", text: text, truncated: bytes.length > limit };
      } catch (e) { /* Binary printer payloads can use text/plain. */ }
    }
    var lines = [], end = Math.min(bytes.length, 512);
    for (var offset = 0; offset < end; offset += 16) {
      lines.push(offset.toString(16).padStart(8, "0") + "  " + Array.from(bytes.subarray(offset, Math.min(offset + 16, end)), function (b) { return b.toString(16).padStart(2, "0"); }).join(" "));
    }
    return { kind: "binary", text: lines.join("\n"), truncated: bytes.length > end };
  }
