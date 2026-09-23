// Manual hardware probes, not Labelary golden fixtures. Node is only needed
// to regenerate these optional artifacts; it is not a Rust build dependency.
import { writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

const groups = [
  { id: 'L01', title: 'Legacy CRLF / FH order', fields: [
    { caption: 'A backslash-& B', data: 'A\\&B', fh: false },
    { caption: 'A hex 0D 0A B', data: 'A#0D#0AB', fh: true },
    { caption: 'A hex 5C 26 B', data: 'A#5C#26B', fh: true },
  ], hypothesis: 'All three matrices should agree if backslash-& becomes CR/LF and FH runs before Legacy escapes. Agreement alone is not raw-byte decoding.' },
  { id: 'L02', title: 'Legacy backslash / double pipe', fields: [
    { caption: 'A two backslash B', data: 'A\\\\B', fh: false },
    { caption: 'A hex 5C B', data: 'A#5CB', fh: true },
    { caption: 'A two pipes B', data: 'A||B', fh: false },
  ], hypothesis: 'First two matrices should agree if doubled backslash becomes one. Third is unresolved: record matrix equality/difference, blank output, or exact INVALID code without assuming a mapping.' },
];
const settings = '^MD0^PR2^PW600^LL300^LH0,0^LS0^LT0^PON^PMN^LRN^FWN^CI27^CVY';
const bx = '^BXN,5,0,23,23,6';
const manifest = { status: 'prepared; not sent', settings: { dpi: 300, mediaMm: [50,25], upperTextMarginDots: 36, barcodeTopDots: 78, darkness: 15, speedIps: 2, ecc: 0, format: 6, symbolModules: 23, moduleDots: 5, ci: 27, cv: 'Y; reset N after each label' }, groups: [] };
let combined = '';
for (const group of groups) {
  let zpl = `~SD15\n^XA\n${settings}\n^FO20,36^A0N,16,14^FD${group.id} ${group.title}^FS\n`;
  for (const [i, field] of group.fields.entries()) {
    const x = 25 + i*200;
    assert(x+23*5 <= 600);
    zpl += `^FO${x},56^A0N,12,11^FD${field.caption}^FS\n`;
    zpl += `^FO${x},78${bx}${field.fh ? '^FH#' : ''}^FD${field.data}^FS\n`;
  }
  zpl += '^CVN\n^FO20,218^A0N,14,12^FD300dpi / SD15 / PR2 / ECC000 F6 / 23x23^FS\n^PQ1\n^XZ\n';
  assert.equal((zpl.match(/\^BX/g)||[]).length,3);
  assert.equal((zpl.match(/\^XA/g)||[]).length,1);
  assert.equal((zpl.match(/\^XZ/g)||[]).length,1);
  const file = `${group.id.toLowerCase()}-300dpi.zpl`;
  writeFileSync(new URL(file,import.meta.url),zpl);
  manifest.groups.push({...group,file,sha256:createHash('sha256').update(zpl).digest('hex')});
  combined += zpl;
}
writeFileSync(new URL('legacy-escapes-300dpi.zpl',import.meta.url),combined);
writeFileSync(new URL('manifest.json',import.meta.url),JSON.stringify(manifest,null,2)+'\n');
console.log('Prepared two labels, three native Legacy fields each. No print job sent.');
