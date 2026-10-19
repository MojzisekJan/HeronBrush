// Usage: node bench_web_protocol.mjs /tmp/heron-web-bench/results.json
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {performance} from 'node:perf_hooks';
import {unpack,delta,apply} from '../../../crates/heron-brush/src/playtest_protocol.js';
const rows=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
for(const row of rows) {
 const before=unpack(fs.readFileSync(row.directory+'/before.html','utf8'));
 const after=unpack(fs.readFileSync(row.directory+'/after.html','utf8'));
 const times=[];let patch;
 for(let i=0;i<9;i++) {const start=performance.now();patch=delta(before.data,after.data);JSON.stringify(patch);times.push(performance.now()-start);}
 assert.deepEqual(apply(before.data,patch),after.data);
 const metadata=structuredClone(before.data);metadata.level.entities[0].properties.benchmark={type:'string',value:'changed'};
 const metadataPatch=delta(before.data,metadata);
 assert.deepEqual(apply(before.data,metadataPatch),metadata);
 assert.equal(Object.keys(metadataPatch.meshes).length,0);
 row.metadata_patch_bytes=Buffer.byteLength(JSON.stringify(metadataPatch));
 const identical=delta(before.data,before.data);assert.equal(Object.keys(identical.meshes).length,0);assert.equal(Object.keys(identical.textures).length,0);
 Object.assign(row,{delta_median_ms:times.sort((a,b)=>a-b)[4],mesh_count:Object.keys(before.data.meshes).length,changed_meshes:Object.keys(patch.meshes).length,patch_bytes:Buffer.byteLength(JSON.stringify(patch)),asset_bytes:Buffer.byteLength(JSON.stringify(before.data.meshes))+Buffer.byteLength(JSON.stringify(before.data.textures))});
 console.log(JSON.stringify(row));
}
fs.writeFileSync('/tmp/heron-web-bench/protocol-results.json',JSON.stringify(rows,null,2)+'\n');
