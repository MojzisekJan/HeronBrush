#!/usr/bin/env python3
"""Serve a built worker and native golden fixture for an installed browser.

The page's Start button opens the real viewer. It checks native/worker level
parity, patch delivery, cancellation/restart, preserved motion state and reports
main-window tick gaps during worker execution. Main editor snapshot preparation
is outside this harness's timing scope and still needs separate measurement.
"""
import argparse
from pathlib import Path
import shutil
import sys
import tempfile
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from serve_web_release import serve

PAGE = r'''<!doctype html><title>Heron export worker check</title>
<button id="start">Start worker check</button><pre id="result">Ready</pre>
<script type="module">
import * as bridge from './worker_bridge.js';
import {unpack} from './playtest_protocol.js';
const note=document.querySelector('#result'), button=document.querySelector('#start');
const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const require=(ok,message)=>{if(!ok)throw Error(message)};
let viewer, maxGap=0, ticks=0;
const actualOpen=window.open.bind(window);
window.open=(...args)=>viewer=actualOpen(...args);
async function bootstrap(job) {
 const bundle=await bridge.load_bundle();require(bundle.length>100,'Missing startup archive');
 const files=bundle.map(([path,bytes])=>['/heron-brush/install/'+path,bytes]);
 for(const item of job.files)files.push([item.path,new Uint8Array(await (await fetch('fixture/'+item.url)).arrayBuffer())]);
 return files;
}
function same(expected,actual,path='level') {
 if(typeof expected==='number'){require(typeof actual==='number'&&Math.abs(expected-actual)<1e-6,path+' number differs');return;}
 if(expected&&typeof expected==='object'){
  require(actual&&typeof actual==='object',path+' missing');
  require(JSON.stringify(Object.keys(expected).sort())===JSON.stringify(Object.keys(actual).sort()),path+' keys differ');
  for(const key of Object.keys(expected))same(expected[key],actual[key],path+'.'+key);
 }else require(expected===actual,path+' differs');
}
async function exportJob(job,files) {
 require(bridge.submit_playtest_export(JSON.stringify(job.snapshot),job.directory,JSON.stringify(job.options),files,'worker-map',0),'Worker busy');
 const start=performance.now();
 while(performance.now()-start<180000){
  const answer=bridge.take_playtest_export();
  if(answer){if(answer.error)throw Error(answer.error);return answer;}
  await wait(20);
 }
 throw Error('Worker timeout');
}
function publish(answer) {
 answer.html=answer.html.replace("new URLSearchParams(location.search).has('verify')",'true');
 if(answer.message.type==='level')answer.message.html=answer.html;
 require(bridge.publish_playtest_export(answer),'Invalid worker revision');
}
async function frame() {
 const start=performance.now();
 while(performance.now()-start<30000){
  const found=viewer?.document.querySelector('iframe');
  if(found?.contentWindow.heronPlaytest&&found.style.visibility==='visible')return found;
  await wait(25);
 }
 throw Error('Viewer did not render');
}
button.onclick=async()=>{
 button.disabled=true;
 let last=performance.now();
 const probe=setInterval(()=>{const now=performance.now();if(bridge.playtest_export_busy()){maxGap=Math.max(maxGap,now-last);ticks++;}last=now;},16);
 try {
  require(bridge.open_playtest(),'Viewer popup blocked');
  const job=await (await fetch('fixture/job.json')).json();
  note.textContent='Exporting native fixture in a Web Worker';
  const answer=await exportJob(job,await bootstrap(job));
  same(job.expectedLevel,unpack(answer.html).data.level);
  const firstMetrics=bridge.playtest_export_metrics();
  publish(answer);
  const original=await frame(), api=original.contentWindow.heronPlaytest;
  api.teleport(new api.THREE.Vector3(1,4,2));api.face(.7);
  const animation=api.animations.values().next().value;if(animation){animation.seek(1.5);animation.pause();animation.update(0);}
  const trigger=api.triggers[0];if(trigger){trigger.wait=3;trigger.fired=true;trigger.pending=true;trigger.inside.add('player');}
  const state=original.contentWindow.heronPlaytestSnapshot();
  job.snapshot.source=job.snapshot.source.replace('"classname" "worldspawn"','"classname" "worldspawn"\n"_worker_check" "changed"');
  const patch=await exportJob(job,[]);
  require(patch.message.type==='patch','Compatible update did not produce a patch');
  require(bridge.playtest_export_metrics().fileBytes===0,'Unchanged files were resent');
  publish(patch);await wait(150);
  require(await frame()===original,'Compatible patch replaced the iframe');
  same(state,original.contentWindow.heronPlaytestSnapshot(),'motion state after patch');
  // Cancel a real worker, then restore its last published baseline and all files.
  const epoch=bridge.playtest_export_epoch();
  bridge.submit_playtest_export(JSON.stringify(job.snapshot),job.directory,JSON.stringify(job.options),[],'worker-map',0);
  bridge.cancel_playtest_export(false);
  require(bridge.playtest_export_epoch()===epoch+1,'Busy worker was not terminated');
  require(bridge.take_playtest_export()===undefined,'Cancelled result leaked');
  const resumed=await exportJob(job,await bootstrap(job));
  publish(resumed);await wait(150);
  same(state,(await frame()).contentWindow.heronPlaytestSnapshot(),'motion state after restart');
  note.textContent='PASS: native parity, live patch, cancellation/restart and preserved player/animation/trigger state\n'+JSON.stringify({triangles:job.triangles,firstMetrics,maxBusyTickGapMs:maxGap,busyTicks:ticks,visibility:document.visibilityState});
 }catch(error){note.textContent='FAIL: '+error.stack;}
 finally{clearInterval(probe);bridge.cancel_playtest_export(true);}
};
</script>'''

if __name__=='__main__':
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('release',type=Path)
 parser.add_argument('fixture',type=Path)
 args=parser.parse_args()
 if not (args.release/'playtest-worker.json').is_file():parser.error('Complete the Trunk worker build first')
 with tempfile.TemporaryDirectory(prefix='heron-worker-check-') as temporary:
  root=Path(temporary)/'web';shutil.copytree(args.release,root)
  shutil.copytree(args.fixture,root/'fixture')
  sources=list(root.glob('snippets/heron-brush-*/src/web_files.js'))
  if len(sources)!=1:raise RuntimeError('Expected one built editor bridge')
  shutil.copy2(sources[0],root/'worker_bridge.js')
  (root/'index.html').write_text(PAGE)
  serve(root,0)
