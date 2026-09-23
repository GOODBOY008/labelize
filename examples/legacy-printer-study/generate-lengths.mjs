// Native printer probes only. This script writes files; it never sends a job.
import { writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

const lengths = [500, 501, 511, 512, 596, 597];
const sha256 = data => createHash('sha256').update(data).digest('hex');
const manifest = {
  status: 'prepared; physical outcomes not recorded',
  purpose: 'Separate automatic size selection from fixed-size field-length rejection; then inspect any printed long records.',
  settings: { dpi: 300, widthDots: 600, heightDots: 300, ecc: 0, format: 1, ci: 27, moduleDots: 3, copiesPerLabel: 1 },
  payload: 'ASCII digit 1 repeated exactly length times',
  hashEncoding: 'UTF-8 with LF line endings (normalize CRLF to LF before checking)',
  probes: [],
};
let combined = '';
for (const length of lengths) {
  const data = '1'.repeat(length);
  const id = `N${length}`;
  let zpl = `~SD15\n^XA\n^MD0^PR2^PW600^LL300^LH0,0^LS0^LT0^PON^PMN^LRN^FWN^CI27\n^FO20,30^A0N,20,18^FD${id} / ECC000 / Format 1^FS\n`;
  for (const [x, size, caption] of [[30, 0, 'AUTO'], [325, 49, 'FIXED 49x49']]) {
    assert(x + 49 * 3 < 600);
    assert(88 + 49 * 3 < 300);
    zpl += `^FO${x},60^A0N,16,14^FD${caption}^FS\n^FO${x},88^CVY^BXN,3,0,${size},${size},1^FD${data}^FS^CVN\n`;
  }
  zpl += '^FO20,258^A0N,14,12^FD300dpi / 50x25mm / exactly repeated digit 1^FS\n^PQ1\n^XZ\n';
  const fields = [...zpl.matchAll(/\^BXN,3,0,(\d+),(\d+),1\^FD(1+)\^FS/g)];
  assert.equal(fields.length, 2);
  assert.deepEqual(fields.map(m => [Number(m[1]), Number(m[2]), m[3].length]), [[0, 0, length], [49, 49, length]]);
  assert.equal((zpl.match(/\^XA/g) || []).length, 1);
  assert.equal((zpl.match(/\^XZ/g) || []).length, 1);
  const file = `${id.toLowerCase()}-300dpi.zpl`;
  writeFileSync(new URL(file, import.meta.url), zpl);
  manifest.probes.push({ id, length, file, sha256: sha256(zpl), payloadSha256: sha256(data), automatic: null, fixed49: null });
  combined += zpl;
}
writeFileSync(new URL('legacy-lengths-300dpi.zpl', import.meta.url), combined);
manifest.combined = { file: 'legacy-lengths-300dpi.zpl', labels: lengths.length, sha256: sha256(combined) };
writeFileSync(new URL('length-manifest.json', import.meta.url), JSON.stringify(manifest, null, 2) + '\n');
console.log('Prepared six labels, AUTO / fixed 49x49 each. No print job sent.');
