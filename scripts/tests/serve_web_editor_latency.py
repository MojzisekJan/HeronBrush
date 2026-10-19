#!/usr/bin/env python3
"""Serve the real editor with a disposable file picker and playtest timing controls.

Use Browse on the welcome screen to open the provided fixture through the production open
path. Then click Open playtest. This measures the editor's own prepareMs and the
main page's timer gaps, including rendering; it is not an isolated CPU benchmark.
No external files or services are read by the browser fixture.
"""
import argparse
from pathlib import Path
import shutil
import sys
import tempfile
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from serve_web_release import serve

PROBE = r'''<aside style="position:fixed;z-index:99999;right:8px;top:8px;background:#fff;color:#111;padding:8px;max-width:520px;font:12px monospace">
<button id="bench-play">Open playtest</button><button id="bench-arm">Measure next edit</button><button id="bench-edit">Measure edit only</button>
<pre id="bench-result">Loading fixture. Then use Browse on the welcome screen.</pre></aside>
<script type="module">
import * as bridge from './BRIDGE_PATH';
const output = document.querySelector('#bench-result');
const response = await fetch('./fixture.map');
if (!response.ok) throw Error('Fixture fetch failed');
const file = new File([await response.arrayBuffer()], 'large.map', {type:'text/plain'});
// Replaces only the chooser in this disposable diagnostic page. The editor still
// executes its ordinary file-open path, virtual storage, parsing and rendering.
window.showOpenFilePicker = async () => [{name:file.name, getFile:async()=>file, isSameEntry:async()=>false}];
output.textContent = 'Fixture ready. Use Browse on the welcome screen, then Open playtest.';
const nativeOpen=window.open.bind(window);let viewer;
window.open=(...args)=>{viewer=nativeOpen(...args);return viewer;};
const viewerFrame=()=>viewer?.document.querySelector('iframe[style*="visible"]');
const viewerTiming=()=>{
 try {const value=viewerFrame()?.contentDocument?.documentElement.dataset.heronPlaytestTiming;return value?JSON.parse(value):null;}
 catch {return null;}
};
let armed=false,last=performance.now(),gaps=[],baseline,baselineViewer,baselineFrame,started,editOnly=false,longTasks=[];
if(typeof PerformanceObserver==='function' && PerformanceObserver.supportedEntryTypes?.includes('longtask')) {
 new PerformanceObserver(list=>{
  if(armed) for(const entry of list.getEntries()) if(entry.startTime>=started) longTasks.push({startMs:entry.startTime-started,durationMs:entry.duration});
 }).observe({type:'longtask',buffered:false});
}
function arm(only=false) {
 armed=true;editOnly=only;gaps=[];longTasks=[];last=started=performance.now();
 baseline=bridge.playtest_export_metrics();baselineViewer=viewerTiming();baselineFrame=viewerFrame();
 output.textContent=only?'Measuring editor-only input for 1.5 seconds.':'Measuring export and viewer update. Make an edit in the canvas.';
}
document.querySelector('#bench-play').onclick=()=>{arm();if(!bridge.open_playtest()){armed=false;output.textContent='Popup blocked';}};
document.querySelector('#bench-arm').onclick=()=>arm();
document.querySelector('#bench-edit').onclick=()=>arm(true);
setInterval(()=>{
 const now=performance.now(),gap=now-last;last=now;if(!armed)return;
 gaps.push(gap);
 const metrics=bridge.playtest_export_metrics(),timing=viewerTiming();
 const exported=!bridge.playtest_export_busy() && metrics && metrics!==baseline;
 const sceneReady=timing && (!baselineViewer || viewerFrame()!==baselineFrame || timing.updates!==baselineViewer.updates);
 if((editOnly && now-started>=1500) || (exported && (sceneReady || now-started>30000))) {
  const result={status:editOnly?'EDITOR_ONLY':sceneReady?'COMPLETE':'VIEWER_TIMEOUT',title:document.title,
   ...(!editOnly?metrics:{}),viewer:timing,elapsedMs:now-started,
   maxTimerGapMs:gaps.reduce((maximum,value)=>Math.max(maximum,value),0),timerTicks:gaps.length,longTasks,
   visibility:document.visibilityState,heapUsedBytes:performance.memory?.usedJSHeapSize};
  document.documentElement.dataset.heronEditorBenchmark=JSON.stringify(result);
  output.textContent=JSON.stringify(result,null,2);armed=false;
 }
},16);
</script>'''

if __name__ == '__main__':
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('release',type=Path)
 parser.add_argument('map',type=Path)
 args=parser.parse_args()
 if not (args.release/'playtest-worker.json').is_file():parser.error('Supply a complete web release')
 with tempfile.TemporaryDirectory(prefix='heron-editor-latency-') as temporary:
  root=Path(temporary)/'web';shutil.copytree(args.release,root)
  bridges=list(root.glob('snippets/heron-brush-*/src/web_files.js'))
  if len(bridges)!=1:raise RuntimeError('Expected one production editor bridge')
  shutil.copy2(args.map,root/'fixture.map')
  page=(root/'index.html').read_text()
  (root/'index.html').write_text(page.replace('</body>',PROBE.replace('BRIDGE_PATH',bridges[0].relative_to(root).as_posix())+'</body>'))
  serve(root,0,metrics=True)
