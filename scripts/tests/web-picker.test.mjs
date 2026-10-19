import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

const source = (await readFile(new URL('../../crates/heron-brush/src/web_files.js', import.meta.url), 'utf8'))
  .replace(/^const \{unpack, delta\}.*\n/, '');
function element(tag) {
  return {tag, children: [], style: {}, attributes: {},
    setAttribute(name, value) {this.attributes[name] = value;},
    append(...nodes) {for (const node of nodes) {this.children.push(node); node.parent = this;}},
    replaceChildren() {this.children = [];},
    remove() {if (this.parent) this.parent.children.splice(this.parent.children.indexOf(this),1); this.removed=true;},
    close() {this.open=false;}, showModal() {this.open=true;}, focus() {},
    click() {this.onclick?.({preventDefault() {}, stopPropagation() {}});}};
}
const body = element('body');
globalThis.document = {body, createElement: element};
globalThis.window = {addEventListener() {}, prompt() {throw Error('A native prompt must not be used for fallback save');}};
globalThis.indexedDB = {open() {throw Error('Picker tests must not store files');}};
const bridge = await import('data:text/javascript;base64,' + Buffer.from(source).toString('base64'));
const tick = () => new Promise(resolve => setImmediate(resolve));
const dialog = () => body.children.find(e => e.tag === 'dialog');
function nodes(root=dialog()) {return [root,...root.children.flatMap(node=>nodes(node))];}
const byTag = tag => nodes().find(e=>e.tag===tag);
const button = name => nodes().find(e=>e.tag==='button' && e.textContent===name);
const failure = name => async () => {throw new DOMException('Picker unavailable',name);};

test('restricted native open offers a readable file copy for SecurityError and NotSupportedError', async () => {
  for (const name of ['SecurityError','NotSupportedError']) {
    window.showOpenFilePicker = failure(name);
    const opened = bridge.pick_open(['map','obj']); await tick();
    assert.equal(dialog().attributes.role,'dialog');
    const input = byTag('input'); assert.equal(input.accept,'.map,.obj');
    input.files = [{name:'level.map',arrayBuffer:async()=>new Uint8Array([1,2,3]).buffer}];
    await input.onchange();
    assert.deepEqual(await opened,['level.map',new Uint8Array([1,2,3])]);
    assert.equal(dialog(),undefined);
  }
});
test('restricted save validates a basename and returns a download name without native prompts', async () => {
  window.showSaveFilePicker = failure('SecurityError');
  let completed=false;
  const saved=bridge.pick_save('suggested.map',['map']); saved.then(()=>completed=true); await tick();
  const input=byTag('input'); assert.equal(input.value,'suggested.map');
  for (const invalid of ['', '.', '..', '../level.map', 'folder\\level.map', 'bad\0name', 'bad\nname']) {
    input.value=invalid;button('Save').click();await tick();
    assert.equal(completed,false,JSON.stringify(invalid));
    assert.match(nodes().find(e=>e.attributes.role==='alert').textContent,/file name without folder/);
  }
  input.value=' level.map ';button('Save').click();
  assert.equal(await saved,'level.map'); assert.equal(dialog(),undefined);
});
test('native cancellation and explicit permission refusal do not open another picker', async () => {
  for (const name of ['AbortError','NotAllowedError']) {
    window.showOpenFilePicker=failure(name); window.showSaveFilePicker=failure(name);
    assert.equal(await bridge.pick_open(['map']),null);
    assert.equal(await bridge.pick_save('level.map',['map']),null);
    assert.equal(dialog(),undefined);
  }
});
test('missing native APIs use in-page dialogs with Escape and Cancel', async () => {
  delete window.showOpenFilePicker; delete window.showSaveFilePicker;
  const opened=bridge.pick_open(['map']); await tick();
  dialog().oncancel({preventDefault() {}});
  assert.equal(await opened,null);
  const saved=bridge.pick_save('level.map',['map']); await tick();
  button('Cancel').click(); assert.equal(await saved,null);
});
test('fallback read errors stay visible and can be cancelled', async () => {
  delete window.showOpenFilePicker;
  const opened=bridge.pick_open(['map']); await tick();
  byTag('input').files=[{name:'gone.map',arrayBuffer:async()=>{throw Error('File gone');}}];
  await byTag('input').onchange();
  assert.match(nodes().find(e=>e.attributes.role==='alert').textContent,/Could not read/);
  button('Cancel').click(); assert.equal(await opened,null);
});
test('starting a new fallback cancels the previous operation without leaving dialogs', async () => {
  delete window.showOpenFilePicker; delete window.showSaveFilePicker;
  const opened=bridge.pick_open(['map']); await tick();
  const saved=bridge.pick_save('next.map',['map']); await tick();
  assert.equal(await opened,null); assert.equal(body.children.filter(e=>e.tag==='dialog').length,1);
  button('Cancel').click(); assert.equal(await saved,null); assert.equal(dialog(),undefined);
});
test('Enter submits the save name without an HTML network form submission', async () => {
  delete window.showSaveFilePicker;
  const saved=bridge.pick_save('keyboard.map',['map']); await tick();
  assert.equal(button('Save').type,'button');
  byTag('input').onkeydown({key:'Enter',preventDefault(){}});
  assert.equal(await saved,'keyboard.map');
});
test('unexpected native errors are visible instead of silently appearing as cancellation', async () => {
  window.showOpenFilePicker=async()=>{throw Error('Unreadable handle');};
  assert.equal(await bridge.pick_open(['map']),null);
  assert.equal(dialog(),undefined);
  const status=body.children.find(e=>e.tag==='aside');
  assert.match(status.children[0].textContent,/Could not open the selected file/);
});

function folderSelection(files) {
  const create=document.createElement; let input;
  document.createElement=tag=>{const node=create(tag);if(tag==='input')input=node;return node;};
  let result;
  try {result=bridge.pick_folder();} finally {document.createElement=create;}
  input.files=files;
  return {input,result};
}
const folderFile=(path,read)=>({webkitRelativePath:path,arrayBuffer:read});
test('folder reads are bounded and retain browser order when IO completes out of order', async () => {
  const waiting=new Map();let active=0,peak=0;
  const files=Array.from({length:24},(_,i)=>folderFile(`Project/sub/file${i}.map`,()=>{
    active++;peak=Math.max(peak,active);
    return new Promise(resolve=>waiting.set(i,()=>{active--;resolve(new Uint8Array([i]).buffer);}));
  }));
  const {input,result}=folderSelection(files); const changed=input.onchange(); await tick();
  assert.equal(waiting.size,4);
  while(waiting.size){const i=Math.max(...waiting.keys());const finish=waiting.get(i);waiting.delete(i);finish();await tick();}
  await changed;const [root,entries]=await result;
  assert.equal(root,'Project');assert.equal(peak,4);assert.equal(active,0);
  assert.deepEqual(entries.map(([path])=>path),files.map((_,i)=>`sub/file${i}.map`));
  assert.deepEqual(entries.map(([,bytes])=>[...bytes]),files.map((_,i)=>[i]));
});
test('one folder read failure stops new IO and completes with no partial import', async () => {
  const waiting=new Map(),started=[];let completed=false;
  const files=Array.from({length:12},(_,i)=>folderFile(`Project/file${i}.map`,()=>{
    started.push(i);return new Promise((resolve,reject)=>waiting.set(i,{resolve,reject}));
  }));
  const {input,result}=folderSelection(files);result.then(()=>completed=true);
  const changed=input.onchange();await tick();
  waiting.get(0).reject(new DOMException('Cannot read file','NotReadableError'));await tick();
  const status=body.children.find(e=>e.tag==='aside');
  assert.match(status.children[0].textContent,/Could not read file0.map/);
  assert.match(status.children[0].textContent,/No files were imported/);
  assert.equal(completed,false,'in-flight reads settle before the import finishes');
  for(const [i,job] of waiting)if(i!==0)job.resolve(new Uint8Array([i]).buffer);
  await changed;assert.equal(await result,null);assert.equal(started.length,4);
});
test('a successful folder retry clears its own old error notice', async () => {
  const {input,result}=folderSelection([folderFile('Project/good.map',async()=>new Uint8Array([42]).buffer)]);
  await input.onchange();assert.deepEqual(await result,['Project',[['good.map',new Uint8Array([42])]]]);
  assert.equal(body.children.find(e=>e.tag==='aside').hidden,true);
});
test('unusable or mixed folder paths fail before any file bytes are read', async () => {
  for(const paths of [[''],['single.map'],['Project/../escape'],['Project/a\\b'],['Project/a','Other/b']]){
    let reads=0;
    const {input,result}=folderSelection(paths.map(path=>folderFile(path,async()=>{reads++;return new ArrayBuffer(0);})));
    await input.onchange();assert.equal(await result,null);assert.equal(reads,0);
    assert.match(body.children.find(e=>e.tag==='aside').children[0].textContent,/No files were imported/);
  }
});
test('folder cancellation and an empty selection complete without reading', async () => {
  let selection=folderSelection([]);selection.input.oncancel();assert.equal(await selection.result,null);
  selection=folderSelection([]);await selection.input.onchange();assert.equal(await selection.result,null);
});
test('starting folder selection cancels an earlier fallback dialog', async () => {
  delete window.showOpenFilePicker;
  const opened=bridge.pick_open(['map']);await tick();assert.ok(dialog());
  const {input,result}=folderSelection([]);assert.equal(await opened,null);assert.equal(dialog(),undefined);
  input.oncancel();assert.equal(await result,null);
});
test('a late folder read error discards earlier successful results', async () => {
  let reads=0;
  const files=Array.from({length:20},(_,i)=>folderFile(`Project/file${i}.map`,async()=>{
    reads++;await tick();if(i===9)throw Error('File disappeared');return new Uint8Array([i]).buffer;
  }));
  const {input,result}=folderSelection(files);await input.onchange();
  assert.equal(await result,null);assert.ok(reads>=10);
  assert.match(body.children.find(e=>e.tag==='aside').children[0].textContent,/Could not read file9.map/);
});
test('an unreadable browser file list resolves instead of leaving the dialog pending', async () => {
  const {input,result}=folderSelection([]);
  Object.defineProperty(input,'files',{get(){throw new DOMException('Selection unavailable','SecurityError');}});
  await input.onchange();assert.equal(await result,null);
  assert.match(body.children.find(e=>e.tag==='aside').children[0].textContent,/Folder files could not be read/);
});
test('an older folder import cannot dismiss a newer identical failure notice', async () => {
  const fail=()=>folderSelection([folderFile('Project/same.map',async()=>{throw Error('Unreadable');})]);
  let failed=fail();await failed.input.onchange();await failed.result;
  let finishSlow;
  const slow=folderSelection([folderFile('Slow/ok.map',()=>new Promise(resolve=>{finishSlow=resolve;}))]);
  const changed=slow.input.onchange();await tick();
  failed=fail();await failed.input.onchange();assert.equal(await failed.result,null);
  finishSlow(new Uint8Array([1]).buffer);await changed;await slow.result;
  const status=body.children.find(e=>e.tag==='aside');
  assert.equal(status.hidden,false);
  assert.match(status.children[0].textContent,/Could not read same.map/);
});
