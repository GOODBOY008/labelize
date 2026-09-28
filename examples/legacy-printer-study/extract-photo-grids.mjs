// Extract recorded photo modules, never generate expected matrices with an encoder.
import { readFileSync, writeFileSync } from 'node:fs';
import assert from 'node:assert/strict';
const payloads = {
  'L01-left': '415c2642', 'L01-middle': '410d0a42', 'L01-right': '415c2642',
  'L02-left': '415c5c42', 'L02-middle': '415c42', 'L02-right': '417c7c42',
};
const records = [];
for (const file of ['compact-photo-observation.json', 'ci13-photo-observation.json']) {
  const observation = JSON.parse(readFileSync(new URL(file, import.meta.url), 'utf8'));
  assert.equal(Object.keys(observation.samples).length, 6);
  for (const [id, sample] of Object.entries(observation.samples)) {
    const hex = payloads[sample.ci27Counterpart ?? id];
    assert(hex);
    assert.equal(sample.sampledRows.length, 23);
    for (const row of sample.sampledRows) assert.match(row, /^[01]{23}$/);
    records.push([`${id}|${hex}`, ...sample.sampledRows].join('\n'));
  }
}
assert.equal(records.length, 12);
writeFileSync(new URL('../../testdata/legacy/ci13-ci27-photo-grids.txt', import.meta.url), records.join('\n\n')+'\n');
console.log('Extracted 12 recorded grids / 6348 modules; no encoder invoked.');
