#!/usr/bin/env python3
"""Serve the production startup loader/cache check on a disposable localhost origin.

Open the URL in an installed browser. PASS after reload requires a full first
archive transfer, a zero-byte repeat transfer, a fresh index, and equal file counts.
"""
from pathlib import Path
import shutil
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from pack_web_bundle import pack
from serve_web_release import serve

PAGE = r"""<!doctype html><title>Heron startup cache check</title><pre id="result">Checking startup archive cache</pre><script type="module">
import {load_bundle} from './web_files.js';
try {
 const files=await load_bundle();
 if(files.length<100)throw Error('Missing startup data');
 const entries=performance.getEntriesByType('resource');
 const archive=entries.find(e=>/startup-[a-f0-9]{64}\.bin$/.test(e.name));
 const index=entries.find(e=>e.name.endsWith('/bundle/index.json'));
 if(!archive||!index)throw Error('Missing resource timing');
 const current={count:files.length,archiveTransfer:archive.transferSize,archiveEncoded:archive.encodedBodySize,indexTransfer:index.transferSize};
 const previous=sessionStorage.getItem('heron-cache-first');
 if(!previous){sessionStorage.setItem('heron-cache-first',JSON.stringify(current));location.reload();}
 else {
  const first=JSON.parse(previous);
  if(first.archiveTransfer<=0||current.archiveTransfer!==0||first.count!==current.count)throw Error('Archive was not reused from cache: '+JSON.stringify({first,current}));
  document.querySelector('#result').textContent='PASS: '+JSON.stringify({first,reload:current});
 }
}catch(error){document.querySelector('#result').textContent='FAIL: '+error.stack;}
</script>
"""

if __name__ == '__main__':
    root = Path(__file__).resolve().parents[2]
    with tempfile.TemporaryDirectory(prefix='heron-web-cache-') as temporary:
        directory = Path(temporary)
        pack(root, directory / 'bundle')
        for name in ['web_files.js', 'playtest_protocol.js']:
            shutil.copy2(root / 'crates/heron-brush/src' / name, directory / name)
        (directory / 'index.html').write_text(PAGE)
        serve(directory, 0)
