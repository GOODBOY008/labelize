import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { mkdtemp, mkdir, readFile, writeFile, rm, access } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

const exec = promisify(execFile);
const root = new URL('../', import.meta.url);
const spec = JSON.parse(readFileSync(new URL('src/playground/openapi.json', root), 'utf8'));
const core = readFileSync(new URL('src/playground/api-core.js', root), 'utf8');
const ui = readFileSync(new URL('src/playground/api.js', root), 'utf8');
function functions() {
  const context = { URL, TextEncoder, TextDecoder, btoa };
  runInNewContext(core, context);
  return context;
}
const api = functions();
const state = overrides => ({ path: '/convert', parameters: { width: '100', height: '60', dpmm: '8', mode: 'raster', output: 'png' }, bodyMode: 'text', text: '^XA^FDAPI^FS^XZ', contentType: 'application/zpl', ...overrides });

test('OpenAPI describes the existing API with resolvable refs and safe embedded JSON', () => {
  assert.equal(spec.openapi, '3.0.3');
  assert.deepEqual(Object.keys(spec.paths).sort(), ['/convert', '/encode', '/format', '/health', '/validate']);
  assert.equal(spec.servers[0].url, '/');
  assert.ok(!readFileSync(new URL('src/playground/openapi.json', root), 'utf8').includes('<'));
  function visit(value) {
    if (!value || typeof value !== 'object') return;
    if (value.$ref) assert.ok(api.apiResolve(spec, value), value.$ref);
    Object.values(value).forEach(visit);
  }
  visit(spec);
});

test('request construction preserves bodies, checks values and separates PNG/PDF options', () => {
  const source = '\x02L\r\n\u0000你好';
  const request = api.apiBuildRequest(spec, state({ text: source }), 'https://custom.example');
  assert.equal(request.body, source);
  assert.equal(request.contentType, 'application/zpl');
  assert.equal(new URL(request.url).origin, 'https://custom.example');
  assert.equal(new URL(request.url).searchParams.get('width'), '100');
  for (const [output, expected, excluded] of [['png', 'scale', 'pages'], ['pdf', 'pages', 'scale']]) {
    const query = new URL(api.apiBuildRequest(spec, state({ parameters: { mode: 'vector', output, scale: '3', pages: 'all' } }), 'https://example.com').url).searchParams;
    assert.ok(query.has(expected));
    assert.ok(!query.has(excluded));
  }
  const query = new URL(api.apiBuildRequest(spec, state({ parameters: { mode: 'raster', scale: '3', pages: 'all', width: '' } }), 'https://example.com').url).searchParams;
  assert.ok(!query.has('scale') && !query.has('pages') && !query.has('width'));
  for (const [name, value] of [['width', '0'], ['height', 'NaN'], ['dpmm', '1.5'], ['dpmm', '2147483648'], ['width', '1junk'], ['output', 'svg'], ['antialias', 'yes']]) {
    assert.throws(() => api.apiBuildRequest(spec, state({ parameters: { [name]: value } }), 'https://example.com'), /Invalid/);
  }
  const file = new File([new Uint8Array([0, 255, 13, 10])], 'raw.tspl');
  const binary = api.apiBuildRequest(spec, state({ bodyMode: 'file', file, contentType: 'application/tspl' }), 'https://example.com');
  assert.equal(binary.body, file);
  assert.equal(api.apiBuildRequest(spec, state({ path: '/encode', contentType: 'image/png' }), 'https://example.com').bodyMode, 'file');
  const health = api.apiBuildRequest(spec, { path: '/health' }, 'https://example.com');
  assert.equal(health.method, 'GET');
  assert.equal(health.bodyMode, 'none');
  assert.ok(!health.body);
});

test('playground defaults produce matching conversion, validation and formatting requests', () => {
  const source = 'SIZE 40 mm,30 mm\nCLS\nTEXT 10,10,"1",0,1,1,"Shared label"\nPRINT 1';
  const parameters = api.apiPlaygroundParameters({ fmt: 'tspl', w_mm: 40, h_mm: 30, dpmm: 12,
    mode: 'vector', scale: 3, allPages: true, aa: true }, 'zpl');
  for (const path of ['/convert', '/validate', '/format']) {
    const request = api.apiBuildRequest(spec, state({ path, parameters, text: source, contentType: 'application/tspl' }), 'https://example.com');
    const query = new URL(request.url).searchParams;
    assert.equal(request.body, source);
    assert.equal(request.contentType, 'application/tspl');
    if (path === '/convert') {
      assert.equal(query.get('width'), '40');
      assert.equal(query.get('height'), '30');
      assert.equal(query.get('dpmm'), '12');
      assert.equal(query.get('mode'), 'vector');
      assert.equal(query.get('scale'), '3');
      assert.equal(query.get('antialias'), 'true');
      assert.equal(query.has('pages'), false);
      const pdf = api.apiBuildRequest(spec, state({ parameters: { ...parameters, output: 'pdf' } }), 'https://example.com');
      assert.equal(new URL(pdf.url).searchParams.get('pages'), 'all');
    } else assert.equal(query.get('language'), 'tspl');
  }
});

test('text and binary previews preserve downloads and bound displayed data', () => {
  const bytes = new TextEncoder().encode('{"ok":false,"error":"<script>"}');
  assert.match(api.apiPreview(bytes, 'application/json').text, /"ok": false/);
  for (const type of ['text/plain', 'application/octet-stream']) {
    const preview = api.apiPreview(new Uint8Array([0, 255, 13, 10]), type);
    assert.equal(preview.kind, 'binary');
    assert.match(preview.text, /00 ff 0d 0a/);
  }
  assert.equal(api.apiPreview(new Uint8Array(), 'image/png').kind, 'image');
  assert.equal(api.apiPreview(new Uint8Array(), 'application/pdf').kind, 'pdf');
  assert.equal(api.apiPreview(new TextEncoder().encode('x'.repeat(70000)), 'text/plain').truncated, true);
  assert.equal(api.apiPreview(new Uint8Array(1024), 'application/octet-stream').truncated, true);
});

async function serverFor(handler, callback) {
  const server = createServer(handler);
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  try { await callback(`http://127.0.0.1:${server.address().port}`); }
  finally { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
}
const outputBytes = Buffer.from([137, 80, 78, 71, 0, 255, 13, 10]);

test('generated cURL, Node and Python examples send exact text/file bytes with safe quoting', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'labelize-api-examples-'));
  const received = [];
  try {
    await serverFor(async (request, response) => {
      const chunks = [];
      for await (const chunk of request) chunks.push(chunk);
      received.push({ body: Buffer.concat(chunks), type: request.headers['content-type'] });
      response.writeHead(200, { 'content-type': 'image/png' });
      response.end(outputBytes);
    }, async origin => {
      const filename = "quo'te $(touch injected).tspl";
      const fileBytes = Buffer.from([0, 255, 13, 10, 128, 65]);
      await writeFile(join(directory, filename), fileBytes);
      const bodies = ["@start 'quote' `code` $(touch injected) $HOME\r\n你好", "NUL\0after\n", null];
      for (const text of bodies) {
        const request = api.apiBuildRequest(spec, state(text === null ? { bodyMode: 'file', file: new File([fileBytes], filename), contentType: 'application/tspl' } : { text }), origin);
        const examples = api.apiExamples(request);
        for (const language of ['curl', 'javascript', 'python']) {
          const command = language === 'curl' ? ['sh', ['-c', examples.curl]] : language === 'javascript' ? [process.execPath, ['--input-type=module', '-e', examples.javascript]] : ['python3', ['-c', examples.python]];
          await exec(...command, { cwd: directory });
          assert.deepEqual(received.at(-1).body, text === null ? fileBytes : Buffer.from(text));
          assert.equal(received.at(-1).type, request.contentType);
          assert.deepEqual(await readFile(join(directory, 'label.png')), outputBytes);
        }
      }
      await assert.rejects(access(join(directory, 'injected')));
    });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test('directory jobs process sequentially, report failures and preserve binary responses', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'labelize-api-jobs-'));
  const input = join(directory, 'input');
  await mkdir(input);
  await writeFile(join(input, 'label.zpl'), 'first');
  await writeFile(join(input, 'label.TSPL'), Buffer.from([0, 255, 13, 10]));
  await writeFile(join(input, 'bad.epl'), 'bad');
  let active = 0, maximum = 0;
  try {
    await serverFor(async (request, response) => {
      active++;
      maximum = Math.max(maximum, active);
      const chunks = [];
      for await (const chunk of request) chunks.push(chunk);
      assert.equal(new URL(request.url, 'http://localhost').searchParams.get('width'), '100');
      const bad = Buffer.concat(chunks).toString() === 'bad';
      response.writeHead(bad ? 400 : 200);
      response.end(bad ? 'bad source' : outputBytes);
      active--;
    }, async origin => {
      for (const language of ['python', 'javascript']) {
        const output = join(directory, language);
        const command = language === 'python' ? 'python3' : process.execPath;
        const script = fileURLToPath(new URL(`examples/http/convert-directory.${language === 'python' ? 'py' : 'mjs'}`, root));
        let failure;
        try { await exec(command, [script, input, output, '--base-url', origin]); } catch (error) { failure = error; }
        assert.equal(failure?.code, 1);
        assert.match(failure.stderr, /bad\.epl: HTTP 400: bad source/);
        assert.match(failure.stdout, /Converted 2\/3 files; 1 failed/);
        assert.deepEqual(await readFile(join(output, 'label.zpl.png')), outputBytes);
        assert.deepEqual(await readFile(join(output, 'label.TSPL.png')), outputBytes);
        await assert.rejects(access(join(output, 'bad.epl.png')));
      }
      assert.equal(maximum, 1);
    });
  } finally { await rm(directory, { recursive: true, force: true }); }
});

function responseHarness(fetch, request = api.apiBuildRequest(spec, state(), 'https://example.com')) {
  const elements = new Map(), blobs = new Map(), revoked = [], messages = [];
  const element = () => ({ hidden: true, disabled: false, textContent: '', removeAttribute(name) { delete this[name]; } });
  const context = {
    URL: { createObjectURL(blob) { const url = `blob:${blobs.size}`; blobs.set(url, blob); return url; }, revokeObjectURL(url) { revoked.push(url); } },
    TextEncoder, TextDecoder, Blob, AbortController, DOMException, performance,
    pending: null, responseUrl: null, image: element(), download: element(), send: element(),
    cancel: { ...element(), addEventListener(name, fn) { this[name] = fn; } },
    document: { getElementById(id) { if (!elements.has(id)) elements.set(id, element()); return elements.get(id); } },
    snapshot: () => request, t: key => key, setMessage: (text, error) => messages.push({ text, error }), fetch,
  };
  elements.set('api-download', context.download);
  const handler = ui.slice(ui.indexOf('    function clearResponse()'), ui.indexOf('    endpoint.addEventListener'));
  const cancellation = ui.slice(ui.indexOf('    cancel.addEventListener'), ui.indexOf('    snippetLanguage.addEventListener'));
  runInNewContext(core + handler + cancellation, context);
  return { context, elements, blobs, revoked, messages, submit: () => context.submit({ preventDefault() {} }) };
}

test('shipped response handler preserves bytes, handles validation failures, and releases URLs', async () => {
  let options;
  const h = responseHarness(async (_, init) => { options = init; return new Response(new Uint8Array([0, 255, 13, 10]), { headers: { 'content-type': 'application/octet-stream' } }); });
  await h.submit();
  assert.equal(options.credentials, 'omit');
  assert.deepEqual(new Uint8Array(await h.blobs.get('blob:0').arrayBuffer()), new Uint8Array([0, 255, 13, 10]));
  assert.match(h.elements.get('api-response-body').textContent, /00 ff 0d 0a/);
  assert.equal(h.context.download.hidden, false);
  await h.submit();
  assert.deepEqual(h.revoked, ['blob:0']);
  assert.equal(h.context.send.disabled, false);
  const validation = responseHarness(async () => Response.json({ ok: false, error: '<script>bad</script>' }), api.apiBuildRequest(spec, state({ path: '/validate' }), 'https://example.com'));
  await validation.submit();
  assert.equal(validation.elements.get('api-status').textContent, '200');
  assert.equal(validation.messages.at(-1).error, true);
  assert.match(validation.elements.get('api-response-body').textContent, /<script>bad<\/script>/);
});

test('PDF responses share original bytes between preview, new tab and download, and clear stale viewers', async () => {
  const bytes = new TextEncoder().encode('%PDF-1.4\nopaque PDF response bytes\n%%EOF');
  const h = responseHarness(async () => new Response(bytes, { headers: { 'content-type': 'application/pdf' } }),
    api.apiBuildRequest(spec, state({ parameters: { output: 'pdf' } }), 'https://example.com'));
  await h.submit();
  const viewer = h.elements.get('api-response-pdf'), open = h.elements.get('api-open-pdf');
  assert.equal(h.elements.get('api-pdf-frame').hidden, false);
  assert.equal(viewer.src, h.context.download.href);
  assert.equal(open.href, viewer.src);
  assert.equal(h.context.download.download, 'label.pdf');
  assert.equal(h.blobs.get(viewer.src).type, 'application/pdf');
  assert.deepEqual(new Uint8Array(await h.blobs.get(viewer.src).arrayBuffer()), bytes);
  assert.equal(h.elements.get('api-response-body').hidden, true);
  h.context.fetch = async () => new Response(new Uint8Array([137, 80, 78, 71]), { headers: { 'content-type': 'image/png' } });
  await h.submit();
  assert.equal(h.elements.get('api-pdf-frame').hidden, true);
  assert.equal(viewer.src, undefined);
  assert.equal(open.href, undefined);
  assert.deepEqual(h.revoked, ['blob:0']);
  assert.equal(h.elements.get('api-image-frame').hidden, false);
  h.context.fetch = async () => new Response(bytes, { status: 400, headers: { 'content-type': 'application/pdf' } });
  await h.submit();
  assert.equal(h.elements.get('api-pdf-frame').hidden, true);
  assert.equal(viewer.src, undefined);
  assert.equal(h.elements.get('api-response-body').hidden, false);
});

test('shipped request handler cancels, prevents duplicates and recovers from network/API errors', async () => {
  let calls = 0;
  const h = responseHarness((_, options) => {
    calls++;
    return new Promise((_, reject) => options.signal.addEventListener('abort', () => reject(new DOMException('Aborted', 'AbortError'))));
  });
  const pending = h.submit();
  await h.submit();
  assert.equal(calls, 1);
  h.context.cancel.click();
  await pending;
  assert.equal(h.messages.at(-1).text, 'api.cancelled');
  assert.equal(h.context.send.disabled, false);
  assert.equal(h.context.cancel.hidden, true);
  h.context.fetch = async () => { throw new Error('offline'); };
  await h.submit();
  assert.match(h.messages.at(-1).text, /offline/);
  h.context.fetch = async () => new Response('invalid dpmm', { status: 400 });
  await h.submit();
  assert.equal(h.messages.at(-1).text, 'HTTP 400');
  assert.equal(h.context.send.disabled, false);
  const missing = responseHarness(async () => assert.fail('must not upload without a file'), api.apiBuildRequest(spec, state({ bodyMode: 'file' }), 'https://example.com'));
  await missing.submit();
  assert.equal(missing.messages.at(-1).text, 'api.fileRequired');
});
