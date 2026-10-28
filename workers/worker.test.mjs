// Run after wasm/build.sh: node --experimental-vm-modules --test workers/*.test.mjs
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { SourceTextModule, SyntheticModule } from 'node:vm';
import test from 'node:test';
import { execFileSync } from 'node:child_process';
import * as glue from '../wasm/npm/labelize_wasm_bg.js';

// Use the actual generated wasm-bindgen glue and WASM. Only the bundler's
// binary import is adapted for Node; no engine/encode behavior is mocked.
const wasm = new WebAssembly.Module(readFileSync(new URL('../wasm/npm/labelize_wasm_bg.wasm', import.meta.url)));
const module = new SourceTextModule(readFileSync(new URL('./worker.js', import.meta.url), 'utf8'));
await module.link((specifier) => {
  const values = specifier.endsWith('.wasm') ? { default: wasm } : glue;
  return new SyntheticModule(Object.keys(values), function () {
    for (const [name, value] of Object.entries(values)) this.setExport(name, value);
  });
});
await module.evaluate();
const worker = module.namespace.default;
const png = readFileSync(new URL('../testdata/labels/amazon.png', import.meta.url));
const call = (query, body = png) => worker.fetch(new Request(`https://example.com/encode${query}`, { method: 'POST', body }), {});

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
