// Regenerate the five homepage PNGs after ./wasm/build.sh:
// node workers/demo.test.mjs --update-previews
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import test from 'node:test';
import { lz_render_output } from '../wasm/npm/init.mjs';

const site = new URL('./site/', import.meta.url);
const demos = JSON.parse(execFileSync('python3', [
  '-c', 'import json; from demo import load_demos; print(json.dumps(load_demos()))',
], { cwd: site, encoding: 'utf8' }));

test('all five homepage previews reproduce the shared Playground job', () => {
  assert.deepEqual(Object.keys(demos), ['zpl', 'epl', 'tspl', 'cpcl', 'dpl']);
  for (const [language, demo] of Object.entries(demos)) {
    const url = new URL(demo.url, 'https://example.org');
    assert.equal(url.pathname, '/playground/');
    const params = new URLSearchParams(url.hash.slice(1));
    const encoded = params.get('c');
    assert.equal(encoded[0], 'b', language);
    const source = Buffer.from(encoded.slice(1), 'base64url').toString('utf8');
    assert.equal(source, demo.source, language);
    assert.match(source, /000123/);
    assert.equal(params.get('f'), language);
    assert.equal(params.get('u'), 'mm');
    // Playground stores width and height in inches, including for displays in mm.
    const width = Number(params.get('w')) * 25.4;
    const height = Number(params.get('h')) * 25.4;
    const dpmm = Number(params.get('d'));
    assert.equal(width, 100);
    assert.equal(height, 50);
    assert.equal(dpmm, 8);

    const rendered = Buffer.from(lz_render_output(
      Buffer.from(source.trim()), width, height, dpmm, params.get('a') === '1',
      false, language, params.get('m') || 'raster', Number(params.get('s') || 1),
      params.get('p') === 'all',
    ));
    assert.equal(rendered.readUInt32BE(16), 800);
    assert.equal(rendered.readUInt32BE(20), 400);
    assert.equal(demo.image, `/assets/demo/${language}.png`);
    const snapshot = new URL('.' + demo.image, site);
    if (process.argv.includes('--update-previews')) writeFileSync(snapshot, rendered);
    assert.deepEqual(rendered, readFileSync(snapshot), language);
  }
});
