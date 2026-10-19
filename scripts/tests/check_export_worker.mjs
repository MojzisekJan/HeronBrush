// Run the built WASM exporter against a separately generated native golden level.
// This checks the headless API. The real browser viewer test remains separate.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';

const [releaseArgument, fixtureArgument] = process.argv.slice(2);
assert(releaseArgument && fixtureArgument, 'Usage: node check_export_worker.mjs RELEASE FIXTURE');
const release = path.resolve(releaseArgument), fixture = path.resolve(fixtureArgument);
const manifest = JSON.parse(fs.readFileSync(path.join(release, 'playtest-worker.json')));
assert.equal(manifest.format, 1);
for (const name of [manifest.js, manifest.wasm]) assert.equal(path.basename(name), name);
globalThis.self = globalThis;
const api = await import(pathToFileURL(path.join(release, manifest.js)).href);
await api.default({module_or_path: fs.readFileSync(path.join(release, manifest.wasm))});
const metadata = JSON.parse(fs.readFileSync(path.join(release, 'bundle/index.json')));
const archive = fs.readFileSync(path.join(release, 'bundle', metadata.file));
assert.equal(archive.subarray(0, 4).toString(), 'HBW1');
const start = 8 + archive.readUInt32LE(4);
const entries = JSON.parse(archive.subarray(8, start));
const files = entries.map(([name, offset, size]) => [
  '/heron-brush/install/' + name, new Uint8Array(archive.subarray(start + offset, start + offset + size)),
]);
const job = JSON.parse(fs.readFileSync(path.join(fixture, 'job.json')));
for (const file of job.files) files.push([file.path, new Uint8Array(fs.readFileSync(path.join(fixture, file.url)))]);
api.apply_files(files);
const result = api.export_playtest(JSON.stringify(job.snapshot), job.directory, JSON.stringify(job.options));
const {unpack} = await import(pathToFileURL(path.join(release, 'playtest_protocol.js')).href);
function same(expected, actual, location = 'level') {
  if (typeof expected === 'number') {
    assert(typeof actual === 'number' && Math.abs(expected - actual) < 1e-6, location + ' number differs');
  } else if (expected && typeof expected === 'object') {
    assert(actual && typeof actual === 'object', location + ' missing');
    assert.deepEqual(Object.keys(expected).sort(), Object.keys(actual).sort(), location + ' keys differ');
    for (const key of Object.keys(expected)) same(expected[key], actual[key], location + '.' + key);
  } else assert.equal(actual, expected, location);
}
same(job.expectedLevel, unpack(result.html).data.level);
console.log(JSON.stringify({result: 'PASS', triangles: job.triangles, files: files.length, exportMs: result.exportMs}));
