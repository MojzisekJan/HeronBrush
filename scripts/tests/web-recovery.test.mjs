import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile, mkdtemp, writeFile, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {execFileSync} from 'node:child_process';
const source = (await readFile(new URL('../../crates/heron-brush/src/web_files.js', import.meta.url), 'utf8'))
  .replace(/^const \{unpack, delta\}.*\n/, '');
function element() {
  return {children: [], style: {}, setAttribute() {}, append(...items) {this.children.push(...items);},
    replaceChildren() {this.children = [];}, click() {return this.onclick?.();}};
}
const body = element(), listeners = new Map();
globalThis.document = {body, createElement: element};
globalThis.window = {addEventListener(name, handler) {listeners.set(name, handler);}};
globalThis.indexedDB = {open() {throw new DOMException('Storage denied', 'SecurityError');}};
const bridge = await import('data:text/javascript;base64,' + Buffer.from(source).toString('base64'));
const bytes = text => new TextEncoder().encode(text);
const children = item => [item, ...item.children.flatMap(children)];
const buttons = () => children(body).filter(item => item.textContent?.startsWith('Download '));
function guarded() {
  let result = false;
  listeners.get('beforeunload')({preventDefault() {result = true;}});
  return result;
}
test('recovery ZIP preserves bytes, Unicode, duplicate basenames and unsafe paths via manifest', async () => {
  const files = [
    ['/project/a/map.map', bytes('first')], ['/project/b/map.map', bytes('second')],
    ['/project/český.txt', bytes('žluťoučký')], ['../../escape', bytes('unsafe')],
    ['/project/A/map.map', bytes('case collision')], ['/nul.txt', new Uint8Array()],
    ['/project/back\\slash', bytes('backslash')],
  ];
  const archive = await bridge.recovery_archive(files);
  const dir = await mkdtemp(join(tmpdir(), 'heron-recovery-'));
  try {
    await writeFile(join(dir, 'backup.zip'), new Uint8Array(await archive.arrayBuffer()));
    await writeFile(join(dir, 'expected.json'), JSON.stringify(files.map(([path, data]) => ({path, bytes: [...data]}))));
    execFileSync('python3', ['-c', `import json,sys,zipfile
from pathlib import PurePosixPath
with zipfile.ZipFile(sys.argv[1]) as z:
 assert z.testzip() is None
 manifest=json.loads(z.read('recovery.json'))
 assert manifest['format']=='heron-brush-recovery' and manifest['version']==1
 expected=json.load(open(sys.argv[2]))
 assert len(z.namelist())==len(expected)+1
 assert len(set(n.lower() for n in z.namelist()))==len(z.namelist())
 for item,want in zip(manifest['files'],expected):
  assert item['path']==want['path']
  assert not PurePosixPath(item['entry']).is_absolute() and '..' not in PurePosixPath(item['entry']).parts
  assert list(z.read(item['entry']))==want['bytes']
  assert item['bytes']==len(want['bytes'])
 assert manifest['files'][0]['entry']=='files/project/a/map.map'
 assert manifest['files'][1]['entry']=='files/project/b/map.map'
`, join(dir, 'backup.zip'), join(dir, 'expected.json')]);
  } finally {await rm(dir, {recursive: true, force: true});}
});
test('archive yields while processing large payloads', async () => {
  let yielded = false;
  const timer = setTimeout(() => {yielded = true;}, 0);
  await bridge.recovery_archive([['/large.bin', new Uint8Array(1024 * 1024)]]);
  clearTimeout(timer);
  assert.equal(yielded, true);
});
test('unsupported archive counts and invalid bytes fail explicitly', async () => {
  await assert.rejects(bridge.recovery_archive(Array(65534).fill(['/empty', new Uint8Array()])), /Too many files/);
  await assert.rejects(bridge.recovery_archive([['/bad', []]]), /Invalid recovery file/);
});
test('many failed writes have bounded controls, every page is reachable and ZIP retains closure guard', async () => {
  const warn = console.warn;
  console.warn = () => {};
  try {
    for (let i = 0; i < 25; i++) await bridge.store_file(`/project/${i}.map`, bytes(`map ${i}`));
  } finally {console.warn = warn;}
  assert.equal(guarded(), true);
  const visited = [];
  for (let page = 0; page < 3; page++) {
    const individual = buttons().filter(item => item.textContent !== 'Download recovery ZIP');
    assert.equal(individual.length, page === 2 ? 5 : 10);
    visited.push(...individual.map(item => item.title));
    const next = children(body).find(item => item.textContent === 'Next recovery files');
    assert.equal(next.disabled, page === 2);
    if (page < 2) next.click();
  }
  assert.equal(new Set(visited).size, 25);
  const oldCreate = URL.createObjectURL, oldRevoke = URL.revokeObjectURL;
  const timer = globalThis.setTimeout;
  let downloaded;
  URL.createObjectURL = blob => {downloaded = blob; return 'blob:test';};
  URL.revokeObjectURL = () => {};
  globalThis.setTimeout = (callback, delay) => delay === 10000 ? 0 : timer(callback, delay);
  try {await buttons().find(item => item.textContent === 'Download recovery ZIP').click();}
  finally {URL.createObjectURL = oldCreate; URL.revokeObjectURL = oldRevoke; globalThis.setTimeout = timer;}
  assert.ok(downloaded.size > 25 * 30);
  assert.equal(guarded(), true);
  assert.equal(buttons().filter(item => item.title).length, 5);
});
test('failed ZIP download preserves individual recovery and clears busy state', async () => {
  const oldCreate = URL.createObjectURL;
  URL.createObjectURL = () => {throw new Error('Download unavailable');};
  try {await buttons().find(item => item.textContent === 'Download recovery ZIP').click();}
  finally {URL.createObjectURL = oldCreate;}
  assert.match(children(body).map(item => item.textContent || '').join(' '), /Individual downloads remain available/);
  assert.equal(buttons().find(item => item.textContent === 'Download recovery ZIP').disabled, false);
  assert.equal(guarded(), true);
});
