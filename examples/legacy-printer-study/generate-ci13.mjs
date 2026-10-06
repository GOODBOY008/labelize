// Run generate.mjs first. This creates files only; it never sends a print job.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

const manifest = { status: 'prepared; physical result pending', ci: 13,
  resetCi: 27, source: 'manifest.json', groups: [] };
let combined = '';
for (const [original, id] of [['l01', 'L03'], ['l02', 'L04']]) {
  const source = readFileSync(new URL(`${original}-300dpi.zpl`, import.meta.url), 'utf8').replace(/\r\n/g, '\n');
  assert.equal((source.match(/\^CI27/g) || []).length, 1);
  const zpl = source.replace('^CI27', '^CI13')
    .replace(`^FD${original.toUpperCase()} `, `^FD${id} CI13 `)
    .replace('^CVN\n', '^CVN^CI27\n');
  // Compare complete barcode field commands, including FH and literal bytes.
  const fields = text => text.match(/\^BX[^\n]+/g);
  assert.deepEqual(fields(zpl), fields(source));
  assert.equal(fields(zpl).length, 3);
  assert.equal((zpl.match(/\^PQ1\n/g) || []).length, 1);
  const file = `${id.toLowerCase()}-ci13-300dpi.zpl`;
  writeFileSync(new URL(file, import.meta.url), zpl);
  manifest.groups.push({ id, source: `${original}-300dpi.zpl`, file,
    sha256: createHash('sha256').update(zpl).digest('hex') });
  combined += zpl;
}
writeFileSync(new URL('legacy-escapes-ci13-300dpi.zpl', import.meta.url), combined);
writeFileSync(new URL('ci13-manifest.json', import.meta.url), JSON.stringify(manifest, null, 2) + '\n');
console.log('Prepared L03/L04: unchanged barcode fields, CI13, restored CI27. No print job sent.');
