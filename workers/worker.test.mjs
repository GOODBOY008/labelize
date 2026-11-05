// Run after wasm/build.sh: node --experimental-vm-modules --test workers/*.test.mjs
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createContext, SourceTextModule, SyntheticModule } from 'node:vm';
import test from 'node:test';
import { execFileSync } from 'node:child_process';
import * as glue from '../wasm/npm/labelize_wasm_bg.js';

// Use the actual generated wasm-bindgen glue and WASM. Only the bundler's
// binary import is adapted for Node; no engine/encode behavior is mocked.
const wasm = new WebAssembly.Module(readFileSync(new URL('../wasm/npm/labelize_wasm_bg.wasm', import.meta.url)));
const spec = JSON.parse(readFileSync(new URL('../src/playground/openapi.json', import.meta.url), 'utf8'));
async function loadWorker(context) {
  const module = new SourceTextModule(readFileSync(new URL('./worker.js', import.meta.url), 'utf8'), { context });
  await module.link((specifier) => {
    const values = specifier.endsWith('.json') ? { default: spec } : specifier.endsWith('.wasm') ? { default: wasm } : glue;
    return new SyntheticModule(Object.keys(values), function () {
      for (const [name, value] of Object.entries(values)) this.setExport(name, value);
    }, { context });
  });
  await module.evaluate();
  return module.namespace.default;
}
const worker = await loadWorker();
const png = readFileSync(new URL('../testdata/labels/amazon.png', import.meta.url));
const call = (query, body = png) => worker.fetch(new Request(`https://example.com/encode${query}`, { method: 'POST', body }), {});

test('public API supports preflight and schema without initializing WASM', async () => {
  let initializations = 0;
  const isolated = await loadWorker(createContext({
    Request, Response, Headers, URL, Uint8Array,
    WebAssembly: { Instance: WebAssembly.Instance, instantiate() { initializations++; throw new Error('engine unavailable'); } },
  }));
  for (const path of [...Object.keys(spec.paths), '/openapi.json']) {
    const response = await isolated.fetch(new Request(`https://example.com${path}`, {
      method: 'OPTIONS', headers: { origin: 'https://client.example', 'access-control-request-method': path === '/health' || path === '/openapi.json' ? 'GET' : 'POST', 'access-control-request-headers': 'content-type' },
    }), {});
    assert.equal(response.status, 204);
    assert.equal(await response.text(), '');
    assert.equal(response.headers.get('access-control-allow-origin'), '*');
    assert.match(response.headers.get('access-control-allow-headers'), /content-type/i);
    assert.equal(response.headers.has('access-control-allow-credentials'), false);
  }
  const schema = await isolated.fetch(new Request('https://example.com/openapi.json'), {});
  assert.equal(schema.headers.get('content-type'), 'application/json');
  assert.deepEqual(await schema.json(), spec);
  assert.equal((await isolated.fetch(new Request('https://example.com/health'), {})).status, 200);
  assert.equal(initializations, 0);
  const failure = await isolated.fetch(new Request('https://example.com/convert', { method: 'POST', body: '^XA^XZ' }), {});
  assert.equal(failure.status, 500);
  assert.equal(failure.headers.get('access-control-allow-origin'), '*');
  assert.equal(initializations, 1);
});

test('CORS survives actual WASM success, bad requests, and validation failures', async () => {
  for (const [path, body, status] of [
    ['/convert?width=100&height=60', '^XA^FO10,10^FDAPI^FS^XZ', 200],
    ['/convert?dpmm=0', '^XA^XZ', 400],
    ['/validate?language=bogus', '^XA^XZ', 400],
    ['/validate?language=tspl', 'AZTEC', 200],
    ['/format?language=zpl', '^XA^FDAPI^FS^XZ', 200],
    ['/encode?language=tspl', png, 200],
  ]) {
    const response = await worker.fetch(new Request(`https://example.com${path}`, {
      method: 'POST', headers: { origin: 'https://client.example', 'content-type': 'application/zpl' }, body,
    }), {});
    assert.equal(response.status, status, path);
    assert.equal(response.headers.get('access-control-allow-origin'), '*', path);
    if (path.startsWith('/validate?language=tspl')) assert.equal((await response.json()).ok, false);
  }
  const html = await worker.fetch(new Request('https://example.com/'), {});
  assert.equal(html.headers.has('access-control-allow-origin'), false);
  assert.deepEqual(JSON.parse(/<script id="api-data" type="application\/json">([\s\S]*?)<\/script>/.exec(await html.text())[1]), spec);
});

test('worker uses real WASM and preserves both round trips', async () => {
  const { lz_render_lang } = glue;
  const outputs = [];
  for (const lang of ['zpl', 'tspl']) {
    const response = await call(`?language=${lang}`);
    assert.equal(response.status, 200);
    assert.equal(response.headers.get('content-type'), lang === 'zpl' ? 'text/plain; charset=utf-8' : 'application/octet-stream');
    const code = new Uint8Array(await response.arrayBuffer());
    outputs.push(lz_render_lang(code, 101.625, 203.25, 8, false, false, lang));
  }
  assert.deepEqual(outputs[0], outputs[1]);
});

test('worker rejects invalid query parameters instead of silently defaulting', async () => {
  for (const query of ['dpmm=0', 'dpmm=-1', 'dpmm=8junk', 'dpmm=1.5', 'dpmm=2147483648', 'dpmm=', 'threshold=-0', 'threshold=-1', 'threshold=256', 'threshold=2.5', 'threshold=abc', 'threshold=', 'language=', 'language=epl', 'dpmm=8&dpmm=12', 'language=zpl&language=tspl']) {
    assert.equal((await call(`?${query}`)).status, 400, query);
  }
  assert.equal((await call('', new Uint8Array())).status, 400);
  assert.equal((await call('', 'not an image')).status, 400);
  const oversized = await call('', 'P6\n4097 4096\n255\n');
  assert.equal(oversized.status, 400);
  assert.match(await oversized.text(), /16 Mi pixels/);
});

test('worker respects threshold extremes and uppercase language', async () => {
  for (const [threshold, byte] of [[0, '00'], [255, 'FF']]) {
    const response = await call(`?language=ZPL&threshold=${threshold}`);
    assert.equal(response.status, 200);
    const code = await response.text();
    if (threshold === 0) {
      for (const field of code.split('^GFA,').slice(1)) {
        const hex = field.slice(field.indexOf('\n') + 1).split('^FS')[0].replace(/\s/g, '');
        assert.match(hex, /^0+$/);
      }
    }
    else assert.ok(code.includes(byte));
  }
});

test('Node init subpath exports encoding and rejects unknown targets', () => {
  // init.mjs owns a separate engine instance; use a fresh process so its
  // wasm-bindgen memory caches cannot alias the worker's instance.
  execFileSync(process.execPath, ['--input-type=module', '-e', `
    import assert from 'node:assert/strict';
    import { readFileSync } from 'node:fs';
    import { lz_encode } from ${JSON.stringify(new URL('../wasm/npm/init.mjs', import.meta.url).href)};
    const png = readFileSync(new URL(${JSON.stringify(new URL('../testdata/labels/amazon.png', import.meta.url).href)}));
    assert.throws(() => lz_encode(png, 8, 128, 'epl'), /1:.*expected zpl or tspl/);
    assert.throws(() => lz_encode(png, 0, 128, 'zpl'), /1:.*dpmm must be positive/);
    assert.ok(lz_encode(png, 8, 128, 'ZPL').length > 0);
  `]);
});

test('vector output uses shared geometry without changing source resolution', async () => {
  const source = '^XA^FO20,20^A0N,30,20^FDPage One^FS^XZ^XA^FDPage Two^FS^XZ';
  const convert = query => worker.fetch(new Request(`https://example.com/convert?${query}`, {
    method: 'POST', headers: { 'content-type': 'application/zpl' }, body: source,
  }), {});
  const response = await convert('mode=vector&scale=3&width=40&height=30');
  assert.equal(response.status, 200);
  const png = new DataView(await response.arrayBuffer());
  assert.equal(png.getUint32(16), 960);
  assert.equal(png.getUint32(20), 720);
  const pdf = await convert('mode=vector&output=pdf&pages=all');
  assert.equal(pdf.status, 200);
  assert.equal(pdf.headers.get('content-type'), 'application/pdf');
  assert.equal((await pdf.text()).slice(0, 4), '%PDF');
  for (const query of ['mode=bogus', 'mode=vector&scale=0', 'mode=vector&scale=NaN', 'mode=vector&scale=10000',
    'mode=vector&output=pdf&scale=2', 'mode=raster&scale=2', 'pages=all', 'mode=vector&pages=all',
    'mode=vector&width=inf', 'mode=vector&dpmm=0', 'output=svg', 'pages=last', 'scale=2&scale=3', 'scale=3junk']) {
    assert.equal((await convert(query)).status, 400, query);
  }
  assert.throws(() => glue.lz_render_output(new TextEncoder().encode(source), 40, 30, 8, false, false, 'bogus', 'vector', 1, false), /1:unknown/);
});

test('vector Worker respects declared sizes and raster Worker keeps its default canvas', async () => {
  const source = 'SIZE 40 mm,30 mm\nDIRECTION 1\nCLS\nTEXT 10,10,"1",0,1,1,"Sized"\nPRINT 1\n';
  for (const [query, contentType, width, height] of [
    ['mode=vector', 'application/tspl', 320, 240],
    ['mode=VECTOR', 'application/octet-stream', 320, 240],
    ['mode=vector&width=50&height=60', 'application/tspl', 400, 480],
    ['mode=raster', 'application/tspl', 816, 1216],
  ]) {
    const response = await worker.fetch(new Request(`https://example.com/convert?${query}`, {
      method: 'POST', headers: { 'content-type': contentType }, body: source,
    }), {});
    assert.equal(response.status, 200, query);
    const png = new DataView(await response.arrayBuffer());
    assert.equal(png.getUint32(16), width, query);
    assert.equal(png.getUint32(20), height, query);
  }
});
