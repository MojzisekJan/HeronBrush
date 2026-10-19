#!/usr/bin/env node
// Usage: node check_web_archive_sharing.cjs /absolute/path/to/generated_module.js
// Each measurement gets a fresh WASM allocator. Linear memory is a high-water
// allocation measure, not live heap size or the browser's total resident memory.
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const path = require('node:path');
const modulePath = path.resolve(process.argv[2] || '');
if (process.argv[3] === 'child') {
  const api = require(modulePath);
  const zip = process.argv[4] === 'zip';
  const size = Number(process.argv[5]);
  const control = process.argv[6] === 'copy';
  api.prepare(zip, size);
  const before = api.memory_bytes();
  api.mount(zip, control);
  const growth = api.memory_bytes() - before;
  process.stdout.write(JSON.stringify({ kind: zip ? 'zip' : 'pak', size, control, growth }));
} else {
  assert.ok(process.argv[2], 'Provide the wasm-bindgen Node module path');
  for (const kind of ['pak', 'zip']) {
    for (const size of [64, 128]) {
      for (const control of ['shared', 'copy']) {
        const child = spawnSync(process.execPath, [__filename, modulePath, 'child', kind, String(size), control], { encoding: 'utf8' });
        if (child.error) throw child.error;
        assert.equal(child.status, 0, child.stderr || child.stdout);
        const result = JSON.parse(child.stdout);
        const payload = size * 1024 * 1024;
        if (control === 'shared') assert.ok(result.growth < payload / 2, JSON.stringify(result));
        else assert.ok(result.growth >= payload, JSON.stringify(result));
        console.log(JSON.stringify(result));
      }
    }
  }
}
