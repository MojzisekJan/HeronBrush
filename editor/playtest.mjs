import {unpack,pack,apply,unchanged} from "./playtest_protocol.js";
const id = location.hash.slice(1);
const status = document.getElementById('status');
if (!/^[a-zA-Z0-9-]{1,64}$/.test(id)) {
  status.textContent = 'Open Play in a new tab from the Heron Brush editor.';
} else {
  const channel = new BroadcastChannel('heron-playtest-' + id);
  let current, pending, state, mapKey, latestHtml, handshake, exported, revision;
  const frames = new WeakMap();
  channel.onmessage = async event => {
    let message = event.data, noChange = false;
    if (message?.type === 'patch') {
      if(!exported || message.mapKey!==mapKey || message.base!==revision) {channel.postMessage({type:'ready'});return;}
      noChange=unchanged(exported.data,message.patch);
      if(!noChange) exported={template:exported.template,data:apply(exported.data,message.patch)};
    } else if (message?.type === 'level' && typeof message.html === 'string') {
      exported=unpack(message.html);
    } else return;
    revision=message.revision;
    clearInterval(handshake);
    if(noChange) return;
    if (typeof message.html === 'string' && message.html === latestHtml && message.mapKey === mapKey) return;
    latestHtml = message.html;
    if (message.mapKey !== mapKey) { state = undefined; mapKey = message.mapKey; }
    const active=current, version=revision;
    if(active && frames.get(active.contentWindow)?.mapKey===mapKey && frames.get(active.contentWindow)?.template===exported.template && active.contentWindow.heronPlaytestUpdate) {
      status.hidden=false;status.textContent='Updating the level…';
      try {
        const updated=await active.contentWindow.heronPlaytestUpdate(exported.data);
        if(version!==revision || active!==current) return;
        if(updated) {status.hidden=true;return;}
      } catch(error) {
        if(version!==revision || active!==current) return;
        status.hidden=false;status.textContent='Could not update the level: '+error.message;return;
      }
    }
    // Compatible updates consume structured data directly. Build a full HTML
    // document only when a new frame is actually needed.
    const html = message.html ?? pack(exported.template,exported.data);
    latestHtml = html;
    pending?.remove();
    const frame = document.createElement('iframe');
    frame.title = 'Level playtest';
    frame.allow = 'fullscreen';
    frame.style.visibility = 'hidden';
    // Serve all module imports locally. The export data already contains meshes/textures.
    const imports = JSON.stringify({imports:{three:new URL('vendor/three/build/three.module.js',location.href).href,'three/addons/':new URL('vendor/three/examples/jsm/',location.href).href}});
    frame.srcdoc = html.replace(/<script type="importmap">.*?<\/script>/s, '<script type="importmap">'+imports+'</script>').replace('Loading the level. Internet access is needed to load Three.js.', 'Loading the level.');
    pending = frame;
    document.body.append(frame);
    frames.set(frame.contentWindow, {frame, mapKey, template:exported.template});
    status.hidden = false; status.textContent = 'Updating the level…';
  };
  window.addEventListener('message', event => {
    if (event.origin !== location.origin) return;
    const entry = frames.get(event.source);
    if (!entry || entry.mapKey !== mapKey) return;
    if (event.data?.type === 'heron-state' && entry.frame === current) state = event.data.state;
    if (event.data?.type === 'heron-error' && entry.frame === pending) {
      pending.remove(); pending = undefined;
      status.hidden = false; status.textContent = 'Could not update the level: ' + event.data.message;
    }
    if (event.data?.type === 'heron-ready' && entry.frame === pending) {
      if (current && frames.get(current.contentWindow)?.mapKey === mapKey) state = current.contentWindow.heronPlaytestSnapshot?.() || state;
      pending.contentWindow.postMessage({type:'heron-restore',state},location.origin);
      current?.remove(); current = pending; pending = undefined;
      current.style.visibility = 'visible'; status.hidden = true;
    }
  });
  const ready = () => channel.postMessage({type:'ready'});
  ready();
  // Also recover if the viewer opened before the editor channel became ready.
  handshake = setInterval(ready, 1000);
  window.addEventListener('pagehide', () => {clearInterval(handshake);channel.close();});
}
