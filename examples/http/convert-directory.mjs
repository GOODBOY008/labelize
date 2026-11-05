#!/usr/bin/env node
// Node.js 20+, no packages required. Each request finishes before the next starts.
import { readFile, writeFile, mkdir, readdir } from 'node:fs/promises';
import { join, extname } from 'node:path';

async function main() {
  const [input, output, ...flags] = process.argv.slice(2);
  const usage = 'Usage: node convert-directory.mjs INPUT OUTPUT [--base-url URL] [--width 100] [--height 60] [--dpmm 8] [--format png|pdf] [--timeout 30]';
  if (input === '--help') { console.log(usage); return; }
  if (!input || !output) throw new Error(usage);
  const options = { 'base-url': process.env.LABELIZE_BASE_URL || 'https://labelize.764629910.workers.dev', width: '100', height: '60', dpmm: '8', format: 'png', timeout: '30' };
  for (let index = 0; index < flags.length; index += 2) {
    const name = flags[index].slice(2), value = flags[index + 1];
    if (!flags[index].startsWith('--') || !Object.hasOwn(options, name) || value === undefined) throw new Error(usage);
    options[name] = value;
  }
  const base = new URL(options['base-url']);
  if (!['http:', 'https:'].includes(base.protocol) || base.search || base.hash) throw new Error('--base-url must be an HTTP(S) URL without a query or fragment');
  for (const name of ['width', 'height', 'dpmm', 'timeout']) {
    if (!options[name].trim() || !Number.isFinite(Number(options[name])) || Number(options[name]) <= 0) throw new Error(`Invalid ${name}`);
  }
  if (!/^\d+$/.test(options.dpmm) || !Number.isSafeInteger(Number(options.dpmm)) || Number(options.dpmm) > 2147483647) throw new Error('dpmm must be a positive 32-bit integer');
  if (!['png', 'pdf'].includes(options.format)) throw new Error('--format must be png or pdf');
  const timeoutMs = Math.ceil(Number(options.timeout) * 1000);
  if (!Number.isSafeInteger(timeoutMs)) throw new Error('Invalid timeout');
  const languages = { '.zpl': 'zpl', '.epl': 'epl', '.tspl': 'tspl', '.cpcl': 'cpcl', '.dpl': 'dpl' };
  const files = (await readdir(input, { withFileTypes: true })).filter(file => file.isFile() && Object.hasOwn(languages, extname(file.name).toLowerCase())).sort((a, b) => a.name.localeCompare(b.name));
  if (!files.length) throw new Error('No .zpl, .epl, .tspl, .cpcl or .dpl files found');
  await mkdir(output, { recursive: true });
  const url = new URL(options['base-url'].replace(/\/+$/, '') + '/convert');
  url.search = new URLSearchParams({ width: options.width, height: options.height, dpmm: options.dpmm, output: options.format }).toString();
  let failures = 0;
  for (const file of files) {
    try {
      const response = await fetch(url, {
        method: 'POST', headers: { 'Content-Type': 'application/' + languages[extname(file.name).toLowerCase()] },
        body: await readFile(join(input, file.name)), signal: AbortSignal.timeout(timeoutMs),
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${await response.text()}`);
      const result = new Uint8Array(await response.arrayBuffer());
      const destination = join(output, file.name + '.' + options.format);
      await writeFile(destination, result);
      console.log(`${file.name} -> ${destination}`);
    } catch (error) {
      failures++;
      console.error(`${file.name}: ${error.message}`);
    }
  }
  console.log(`Converted ${files.length - failures}/${files.length} files; ${failures} failed.`);
  if (failures) process.exitCode = 1;
}

main().catch(error => { console.error(error.message); process.exitCode = 1; });
