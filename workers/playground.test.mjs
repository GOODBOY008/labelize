import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext, Script } from 'node:vm';
import test from 'node:test';

// Execute the shipped upload handler and parameter builder with browser APIs
// supplied by a small DOM harness. No copy of the conversion logic lives here.
const source = readFileSync(new URL('../src/playground.rs', import.meta.url), 'utf8');
const upload = source.slice(source.indexOf('  /* ── Image → Code'), source.indexOf('  fileInput.addEventListener("change"'));
const params = source.slice(source.indexOf('  var SIZE_PRESETS ='), source.indexOf('  function buildUrl('));
function element(value = '') {
  return { value, style: {}, disabled: false, listeners: {}, addEventListener(name, fn) { this.listeners[name] = fn; }, click() {}, focus() {}, remove() {} };
}
function harness({ width = 813, height = 1626, lang = 'zpl', dpmm = 8, fetch, readError = false } = {}) {
  const button = element(), fileInput = element();
  const revoked = [], downloads = [], calls = [];
  const context = {
    fmtSel: element(lang), dpmmSel: element(String(dpmm)), widthInput: element('101.6'), heightInput: element('152.4'), sizeUnit: element('mm'),
    sizePreset: element('4x6'), input: element('existing code'), aaChk: { checked: false },
    fmtUserTouched: true, autoLoaded: null,
    document: { getElementById: id => id === 'img2code-btn' ? button : fileInput,
      body: { appendChild() {} }, createElement() { const a = element(); a.click = () => downloads.push(a); return a; } },
    URL: { createObjectURL: () => `blob:${calls.length}`, revokeObjectURL: url => revoked.push(url) },
    Image: class { naturalWidth = width; naturalHeight = height; set src(value) { queueMicrotask(() => readError ? this.onerror() : this.onload()); } },
    fetch: async (url, options) => { calls.push({ url, options }); return fetch ? fetch(url, options) : new Response('^XA^GFA^XZ'); },
    setTimeout: fn => fn(), setStatus() {}, showToast() {}, t: key => key, fmt: (text, values) => text + values.msg,
    updateCompareState() {}, updateCaret() {}, scheduleCheck() {}, scheduleAutoRender() {}, render() { context.rendered = true; },
  };
  runInNewContext(params + upload, context);
  fileInput.files = [Object.assign(new Blob(['image']), { name: 'image.png' })];
  return { context, button, calls, revoked, downloads, change: () => fileInput.listeners.change.call(fileInput) };
}

test('image uploads preserve exact dimensions, including small and large images', async () => {
  for (const [width, height, dpmm] of [[13, 7, 8], [813, 1626, 8], [3500, 800, 24]]) {
    const h = harness({ width, height, dpmm });
    await h.change();
    const p = h.context.getParams();
    assert.ok(Math.abs(p.w_mm * dpmm - width) < 1e-9);
    assert.ok(Math.abs(p.h_mm * dpmm - height) < 1e-9);
    assert.equal(h.context.sizePreset.value, 'custom');
    assert.equal(h.context.input.value, '^XA^GFA^XZ');
    assert.ok(h.context.rendered);
    assert.equal(h.button.disabled, false);
    assert.equal(h.revoked.length, 1);
  }
});

test('pending upload uses the captured resolution and prevents duplicate uploads', async () => {
  let finish;
  const h = harness({ fetch: () => new Promise(resolve => { finish = resolve; }) });
  const pending = h.change();
  while (!finish) await new Promise(resolve => queueMicrotask(resolve));
  h.context.dpmmSel.value = '24';
  await h.change();
  assert.equal(h.calls.length, 1);
  finish(new Response('^XA^GFA^XZ'));
  await pending;
  assert.equal(h.context.dpmmSel.value, '8');
  assert.equal(h.context.getParams().dpmm, 8);
});

test('failed image read or API response preserves the existing editor and unlocks upload', async () => {
  for (const options of [{ readError: true }, { fetch: () => new Response('bad image', { status: 400 }) }]) {
    const h = harness(options);
    await h.change();
    assert.equal(h.context.input.value, 'existing code');
    assert.equal(h.context.widthInput.value, '101.6');
    assert.equal(h.button.disabled, false);
    assert.equal(h.revoked.length, 1);
    assert.equal(h.context.rendered, undefined);
  }
});

test('TSPL downloads binary output without changing the editor', async () => {
  const h = harness({ lang: 'tspl', fetch: () => new Response(new Uint8Array([0, 255, 13, 10])) });
  await h.change();
  assert.equal(h.downloads.length, 1);
  assert.equal(h.downloads[0].download, 'image.tspl');
  assert.equal(h.context.input.value, 'existing code');
  assert.equal(h.button.disabled, false);
  assert.equal(h.revoked.length, 1);
});


test('switching display units after upload preserves exact pixel dimensions', async () => {
  const h = harness({ width: 205, height: 137, dpmm: 12 });
  await h.change();
  for (const unit of ['in', 'mm', 'in', 'mm']) {
    h.context.sizeUnit.value = unit;
    h.context.sizeUnit.listeners.change.call(h.context.sizeUnit);
    const p = h.context.getParams();
    assert.ok(Math.abs(p.w_mm * 12 - 205) < 1e-9);
    assert.ok(Math.abs(p.h_mm * 12 - 137) < 1e-9);
  }
});

test('merged playground markup has unique IDs and a visible upload entry point', () => {
  const html = source.slice(source.indexOf('<!DOCTYPE html>'), source.indexOf('</html>'));
  const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
  assert.equal(new Set(ids).size, ids.length, 'duplicate controls after merge');
  for (const id of ['img-input', 'img2code-btn', 'label-width', 'label-height', 'size-unit']) {
    assert.ok(ids.includes(id), `missing ${id}`);
  }
  for (const [, attributes, script] of html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/g)) {
    if (!attributes.includes('application/ld+json')) new Script(script);
  }
});
