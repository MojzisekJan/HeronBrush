// This module owns all expensive authoring export and delta preparation.
import {unpack, delta} from './playtest_protocol.js';
let cached, api;
const ready = (async () => {
  const response = await fetch('playtest-worker.json', {cache:'no-cache'});
  if (!response.ok) throw Error(`Worker manifest: HTTP ${response.status}`);
  const manifest = await response.json();
  if (manifest.format !== 1 || !/^[a-f0-9]{64}$/.test(manifest.version)
      || !/^heron[-_]brush[-_]export[-_]web\.js$/.test(manifest.js)
      || !/^heron[-_]brush[-_]export[-_]web_bg\.wasm$/.test(manifest.wasm)) throw Error('Invalid export worker manifest');
  const moduleURL = new URL(manifest.js, import.meta.url); moduleURL.searchParams.set('v', manifest.version);
  const wasmURL = new URL(manifest.wasm, import.meta.url); wasmURL.searchParams.set('v', manifest.version);
  api = await import(moduleURL.href);
  const wasm = await fetch(wasmURL, {cache:'force-cache'});
  if (!wasm.ok) throw Error(`Worker WASM: HTTP ${wasm.status}`);
  await api.default({module_or_path:wasm});
})();
// Register rejection immediately; report it with the job's id below.
ready.catch(() => {});
self.onmessage = async ({data:job}) => {
  try {
    await ready;
    api.apply_files(job.files);
    const result = api.export_playtest(job.snapshot, job.directory, job.options);
    const next = unpack(result.html);
    if (!cached || cached.revision !== job.base || cached.mapKey !== job.mapKey) {
      cached = job.previousHtml ? {...unpack(job.previousHtml), revision:job.base, mapKey:job.previousMapKey} : null;
    }
    const message = cached && cached.mapKey === job.mapKey && cached.template === next.template
      ? {type:'patch', patch:delta(cached.data,next.data), mapKey:job.mapKey, base:job.base, revision:job.revision}
      : {type:'level', html:result.html, mapKey:job.mapKey, revision:job.revision};
    cached = {...next, revision:job.revision, mapKey:job.mapKey};
    self.postMessage({id:job.id, ...result, message});
  } catch (error) { self.postMessage({id:job.id, error:[self.heronExportPanic, String(error?.stack || error)].filter(Boolean).join('\n')}); }
};
