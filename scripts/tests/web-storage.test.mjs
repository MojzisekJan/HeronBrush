import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

// Exercise the production bridge without a browser download. IndexedDB transactions
// deliberately complete later than the Rust call and can abort without an error event.
const source = (await readFile(new URL('../../crates/heron-brush/src/web_files.js', import.meta.url), 'utf8'))
  .replace(/^const \{unpack, delta\}.*\n/, '');
function element() {
  return {children: [], hidden: false, style: {}, setAttribute() {},
    append(...items) { this.children.push(...items); },
    replaceChildren() { this.children = []; }, click() { this.onclick?.(); }};
}
console.warn = (...args) => { if (!String(args[0]).startsWith('Heron Brush: could not store')) throw new Error('Unexpected warning'); };
const body = element(), listeners = new Map(), transactions = [], saved = new Map();
globalThis.document = {body, createElement: element};
globalThis.window = {addEventListener(name, handler) {listeners.set(name, handler);}};
globalThis.indexedDB = {open() {
  const request = {};
  queueMicrotask(() => {
    request.result = {transaction() {
      const transaction = {objectStore() {return {put(bytes, path) {
        transactions.push({transaction, bytes, path});
      }, delete(path) { transactions.push({transaction, path}); }};}};
      return transaction;
    }};
    request.onsuccess();
  });
  return request;
}};
const bridge = await import('data:text/javascript;base64,' + Buffer.from(source).toString('base64'));
const tick = () => new Promise(resolve => setImmediate(resolve));
function finish() {
  const {transaction, bytes, path} = transactions.shift();
  if (bytes) saved.set(path, bytes); else saved.delete(path);
  transaction.oncomplete();
}
function prevented() {
  let prevented = false;
  listeners.get('beforeunload')({preventDefault() {prevented = true;}});
  return prevented;
}
test('pending saves guard tab closure, own their bytes, and preserve write order', async () => {
  const bytes = new Uint8Array([1]);
  const first = bridge.store_file('/map.map', bytes);
  bytes[0] = 99;
  const second = bridge.store_file('/map.map', new Uint8Array([2]));
  await tick();
  assert.equal(prevented(), true);
  assert.equal(transactions.length, 1);
  assert.equal(transactions[0].bytes[0], 1);
  finish(); await first; await tick();
  assert.equal(transactions.length, 1);
  finish(); await second;
  assert.equal(saved.get('/map.map')[0], 2);
  assert.equal(prevented(), false);
});
test('aborted saves expose recovery and retry succeeds with the retained bytes', async () => {
  const write = bridge.store_file('/rescue.map', new Uint8Array([42]));
  await tick();
  transactions.shift().transaction.onabort();
  await write;
  assert.equal(prevented(), true);
  const notice = body.children[0];
  assert.equal(notice.hidden, false);
  assert.ok(notice.children.some(child => child.textContent === 'Download rescue.map'));
  notice.children.find(child => child.textContent === 'Retry storage').click();
  await tick();
  assert.equal(transactions[0].bytes[0], 42);
  finish(); await tick();
  assert.equal(notice.hidden, true);
  assert.equal(prevented(), false);
});

test('deletion waits for an in-flight save so deleted maps cannot reappear', async () => {
  const save = bridge.store_file('/deleted.map', new Uint8Array([9]));
  const deletion = bridge.delete_file('/deleted.map');
  await tick();
  assert.equal(transactions.length, 1);
  finish(); await save; await tick();
  finish(); await deletion;
  assert.equal(saved.has('/deleted.map'), false);
  assert.equal(prevented(), false);
});
test('a bundle HTTP failure is visible instead of becoming fake map bytes', async () => {
  globalThis.fetch = async () => ({ok: false, status: 503});
  const warn = console.warn;
  console.warn = () => {};
  try {
    assert.deepEqual(await bridge.load_bundle(), []);
  } finally { console.warn = warn; }
  const notice = body.children[0];
  assert.equal(notice.hidden, false);
  assert.match(notice.children[0].textContent, /Bundled editor data could not be loaded/);
});
test('same-basename file and folder imports retain names but get independent paths', async () => {
  const first = await bridge.picked_file_path('/heron-brush/picked', 'level.map');
  const second = await bridge.picked_file_path('/heron-brush/picked', 'level.map');
  assert.notEqual(first, second);
  assert.ok(first.endsWith('/level.map'));
  const folder = await bridge.picked_file_path('/heron-brush/files', 'project');
  assert.ok(folder.startsWith('/heron-brush/files/'));
  assert.ok(folder.endsWith('/project'));
});
test('reselecting the same physical file retains its stored path and save handle', async () => {
  const file = {name: 'level.map', arrayBuffer: async () => new Uint8Array([3]).buffer};
  const firstHandle = {getFile: async () => file};
  window.showOpenFilePicker = async () => [firstHandle];
  await bridge.pick_open(['map']);
  const path = await bridge.picked_file_path('/heron-brush/picked', file.name);
  bridge.adopt_handle(path);
  await tick(); finish();
  const sameHandle = {getFile: async () => file, isSameEntry: async other => other === firstHandle};
  window.showOpenFilePicker = async () => [sameHandle];
  await bridge.pick_open(['map']);
  assert.equal(await bridge.picked_file_path('/heron-brush/picked', file.name), path);
  const differentHandle = {getFile: async () => file, isSameEntry: async () => false};
  window.showOpenFilePicker = async () => [differentHandle];
  await bridge.pick_open(['map']);
  assert.notEqual(await bridge.picked_file_path('/heron-brush/picked', file.name), path);
});
test('startup archive round trips all staged source files byte for byte', async () => {
  const {mkdtemp, rm} = await import('node:fs/promises');
  const {tmpdir} = await import('node:os');
  const {join} = await import('node:path');
  const {execFileSync} = await import('node:child_process');
  const directory = await mkdtemp(join(tmpdir(), 'heron-web-pack-'));
  try {
    execFileSync('python3', [new URL('../pack_web_bundle.py', import.meta.url).pathname, directory]);
    const manifest = JSON.parse(await readFile(join(directory, 'index.json'), 'utf8'));
    const archive = await readFile(join(directory, manifest.file));
    const files = bridge.unpack_bundle(archive);
    const index = (await readFile(join(directory, 'files.txt'), 'utf8')).trim().split('\n');
    assert.deepEqual(files.map(([path]) => path), index);
    assert.ok(files.length > 100);
    for (const [path, bytes] of files) {
      const original = new URL('../../' + path.replace(/^games\//, 'gamedata/'), import.meta.url);
      assert.deepEqual(Buffer.from(bytes), await readFile(original), path);
    }
    const calls = [];
    globalThis.fetch = async (path, options) => {calls.push([path, options.cache]); return {ok: true, json: async () => manifest, arrayBuffer: async () => archive};};
    assert.equal((await bridge.load_bundle()).length, files.length);
    assert.deepEqual(calls, [['bundle/index.json', 'no-cache'], [`bundle/${manifest.file}`, 'force-cache']]);
    // A damaged cache entry must be checked and repaired from the network once.
    calls.length = 0;
    const damaged = Buffer.from(archive); damaged[damaged.length - 1] ^= 1;
    globalThis.fetch = async (path, options) => {
      calls.push([path, options.cache]);
      return {ok: true, json: async () => manifest, arrayBuffer: async () => options.cache === 'force-cache' ? damaged : archive};
    };
    assert.equal((await bridge.load_bundle()).length, files.length);
    assert.deepEqual(calls.map(([, cache]) => cache), ['no-cache', 'force-cache', 'reload']);
    // New data must change the content address instead of reusing an old URL.
    const {createHash} = await import('node:crypto');
    assert.equal(createHash('sha256').update(archive).digest('hex'), manifest.sha256);
    assert.notEqual(createHash('sha256').update(damaged).digest('hex'), manifest.sha256);
    const updatedHash = createHash('sha256').update(damaged).digest('hex');
    const updated = {...manifest, sha256:updatedHash, file:`startup-${updatedHash}.bin`};
    calls.length = 0;
    globalThis.fetch = async (path, options) => {
      calls.push([path, options.cache]);
      return {ok:true, json:async()=>updated, arrayBuffer:async()=>damaged};
    };
    const newer = await bridge.load_bundle();
    assert.equal(calls[1][0], `bundle/${updated.file}`);
    const originalLast = files.at(-1)[1], newerLast = newer.at(-1)[1];
    assert.equal(newerLast.at(-1), originalLast.at(-1) ^ 1);
    // Neither cached nor network-corrupt data may be accepted as a previous build.
    const warn = console.warn; console.warn = () => {};
    try {
      globalThis.fetch = async () => ({ok:true, json:async()=>manifest, arrayBuffer:async()=>damaged});
      assert.deepEqual(await bridge.load_bundle(), []);
    } finally { console.warn = warn; }

  } finally { await rm(directory, {recursive: true, force: true}); }
});
test('startup archive rejects truncation, traversal, duplicates and bad bounds', () => {
  function archive(entries, payload = new Uint8Array([1, 2])) {
    const manifest = new TextEncoder().encode(JSON.stringify(entries));
    const bytes = new Uint8Array(8 + manifest.length + payload.length);
    bytes.set(new TextEncoder().encode('HBW1'));
    new DataView(bytes.buffer).setUint32(4, manifest.length, true);
    bytes.set(manifest, 8); bytes.set(payload, 8 + manifest.length);
    return bytes;
  }
  for (const entries of [
    [['games/../../bad', 0, 1]], [['/games/x', 0, 1]], [['games/x', -1, 1]],
    [['games/x', 0, 3]], [['games/x', 0, 1], ['games/x', 1, 1]], [['games/x', 1.5, 1]],
  ]) assert.throws(() => bridge.unpack_bundle(archive(entries)));
  assert.throws(() => bridge.unpack_bundle(new Uint8Array(7)));
  const truncated = archive([['games/x', 0, 2]]).slice(0, -1);
  assert.throws(() => bridge.unpack_bundle(truncated));
});
test('physical file writes wait for close, copy bytes and preserve save order', async () => {
  const writes = [], closes = [];
  const handle = {name: 'ordered.map', queryPermission: async () => 'granted',
    createWritable: async () => ({write: async bytes => writes.push([...bytes]),
      close: () => new Promise(resolve => closes.push(resolve))})};
  window.showSaveFilePicker = async () => handle;
  await bridge.pick_save('ordered.map', ['map']);
  bridge.adopt_handle('/ordered.map'); await tick(); finish();
  const bytes = new Uint8Array([1]);
  const first = bridge.save_user_file('/ordered.map', 'ordered.map', bytes);
  bytes[0] = 99;
  const second = bridge.save_user_file('/ordered.map', 'ordered.map', new Uint8Array([2]));
  await tick();
  assert.deepEqual(writes, [[1]]);
  assert.equal(prevented(), true);
  closes.shift()(); await first; await tick();
  assert.deepEqual(writes, [[1], [2]]);
  assert.equal(prevented(), true);
  closes.shift()(); await second;
  assert.equal(prevented(), false);
});
test('failed physical writes abort their stream and visibly request a recovery download', async () => {
  let aborted = false, downloads = 0;
  const handle = {name: 'denied.map', queryPermission: async () => 'granted',
    createWritable: async () => ({write: async () => {throw new Error('Disk full');},
      abort: async () => {aborted = true;}})};
  window.showSaveFilePicker = async () => handle;
  await bridge.pick_save('denied.map', ['map']);
  bridge.adopt_handle('/denied.map'); await tick(); finish();
  const create = document.createElement, warn = console.warn, timer = globalThis.setTimeout;
  document.createElement = () => ({...element(), click() {downloads++;}});
  console.warn = () => {};
  globalThis.setTimeout = callback => {callback();};
  try { await bridge.save_user_file('/denied.map', 'denied.map', new Uint8Array([8])); }
  finally {document.createElement = create; console.warn = warn; globalThis.setTimeout = timer;}
  assert.equal(aborted, true);
  assert.equal(downloads, 1);
  assert.match(body.children[0].children[0].textContent, /Could not update denied.map in place/);
  body.children[0].children.find(child => child.textContent === 'Dismiss file notice').click();
  assert.equal(prevented(), false);
});

test('startup manifest rejects arbitrary URLs and oversized archives before fetching them', async () => {
  const warn = console.warn; console.warn = () => {};
  try {
    for (const manifest of [
      {format:1, sha256:'a'.repeat(64), file:'https://example.com/payload', bytes:10},
      {format:1, sha256:'a'.repeat(64), file:'startup-'+ 'a'.repeat(64)+'.bin', bytes:134217729},
      {format:2, sha256:'a'.repeat(64), file:'startup-'+ 'a'.repeat(64)+'.bin', bytes:10},
    ]) {
      let count = 0;
      globalThis.fetch = async () => {count++; return {ok:true, json: async () => manifest};};
      assert.deepEqual(await bridge.load_bundle(), []);
      assert.equal(count, 1);
    }
  } finally { console.warn = warn; }
});
test('background exports reuse idle workers, cancel stale work and keep the last viewer baseline', () => {
  const workers = [];
  document.baseURI = 'https://editor.test/';
  globalThis.Worker = class {
    constructor(url) {this.url = url; this.jobs = []; workers.push(this);}
    postMessage(job, transfers) {this.jobs.push(job); this.transfers = transfers;}
    terminate() {this.terminated = true;}
  };
  const submit = () => bridge.submit_playtest_export('{}', '/files', '{}', [['/image', new Uint8Array([1,2])]], 'map-A', 2);
  assert.equal(submit(), true);
  const first = workers[0], job = first.jobs[0];
  assert.equal(bridge.playtest_export_busy(), true);
  assert.equal(first.transfers.length, 1);
  bridge.cancel_playtest_export(false);
  assert.equal(first.terminated, true);
  assert.equal(bridge.playtest_export_epoch(), 1);
  first.onmessage({data:{id:job.id, html:'stale', message:{type:'level',revision:1,mapKey:'map-A'}}});
  assert.equal(bridge.take_playtest_export(), undefined);
  submit();
  const second = workers[1], next = second.jobs[0];
  second.onmessage({data:{id:next.id, html:'current', warnings:[], exportMs:10, message:{type:'level',revision:1,mapKey:'map-A'}}});
  const answer = bridge.take_playtest_export();
  assert.equal(bridge.publish_playtest_export(answer), true);
  assert.equal(bridge.playtest_export_busy(), false);
  submit();
  assert.equal(workers.length, 2, 'idle worker was unnecessarily restarted');
  assert.equal(second.jobs[1].previousHtml, undefined);
  bridge.cancel_playtest_export(false);
  submit();
  assert.equal(workers[2].jobs[0].previousHtml, 'current');
  assert.equal(workers[2].jobs[0].previousMapKey, 'map-A');
  assert.equal(bridge.publish_playtest_export({html:'wrong',message:{type:'patch',base:0,revision:2}}), false);
  bridge.cancel_playtest_export(true);
});

test('unavailable startup storage keeps recovery instructions ahead of reload advice', async () => {
  const previousDatabase = globalThis.indexedDB;
  const previousWarning = console.warn;
  const previousListeners = new Map(listeners);
  const warnings = [];
  try {
    globalThis.indexedDB = {open() {throw new DOMException('Storage denied', 'SecurityError');}};
    console.warn = (...args) => warnings.push(String(args[0]));
    const denied = await import('data:text/javascript;base64,' + Buffer.from(source).toString('base64') + '#denied-storage');
    assert.deepEqual(await denied.load_files(), []);
    const status = body.children.at(-1);
    assert.match(status.children[0].textContent, /Reload to try again/);
    await denied.store_file('/denied/recovery.map', new Uint8Array([73]));
    assert.match(status.children[0].textContent, /^Browser storage failed for 1 file/);
    assert.match(status.children[0].textContent, /Keep this tab open and download a recovery copy before reloading/);
    assert.doesNotMatch(status.children[0].textContent, /Reload to try again/);
    assert.ok(status.children.some(child => child.textContent === 'Download recovery.map'));
    assert.equal(prevented(), true);
    status.children.find(child => child.textContent === 'Retry storage').click();
    await tick();
    assert.equal(prevented(), true);
    assert.match(status.children[0].textContent, /Keep this tab open/);
    assert.ok(warnings.every(message => message.startsWith('Heron Brush:')));
  } finally {
    globalThis.indexedDB = previousDatabase;
    console.warn = previousWarning;
    listeners.clear();
    for (const [name, handler] of previousListeners) listeners.set(name, handler);
  }
});
