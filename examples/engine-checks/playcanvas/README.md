# PlayCanvas export check

`verify_feature_lab.cjs` runs the feature lab's PlayCanvas export in Chromium. The
first half loads it the way a game does: a PlayCanvas app with ammo.js, then
`loadLevel` from the exported `heron_playcanvas.js`. The second half opens the
generated `feature_lab.html` with `?verify` and drives its player.
`feature_lab_checks.js` holds the checks; the results are in
[feature runtime checks for PlayCanvas](../../../docs/feature-runtime-playcanvas.md).

```sh
cargo build -p heron-brush
python3 examples/engine-checks/export_feature_lab.py --root /tmp/fl playcanvas
python3 examples/engine-checks/feature_lab_poses.py /tmp/fl/poses.json
HERON_CDN_CACHE=/tmp/fl/cdn node examples/engine-checks/playcanvas/verify_feature_lab.cjs /tmp/fl /tmp/fl/poses.json /tmp/fl/out
```

It writes `playcanvas-lab.png` (the three.js check's view), `playcanvas-stations.png`
(closer, without fog), `playcanvas-playtest.png` (the generated page) and
`results.json`, and exits with status 1 on any failure.

PlayCanvas comes from jsDelivr's npm mirror and ammo.js from the engine's repository
at the same tag, at the version the exported page names, so the check needs internet
access once. `HERON_CDN_CACHE` keeps the downloads in a folder for later runs.

The check needs Node with `playwright`. Without Node, Python's Playwright package
ships one: run `uv run --with playwright python -c "import playwright"` once, then
use the `node` inside its `playwright/driver` folder, with `NODE_PATH` pointing at a
folder that holds a `playwright` link to `driver/package`.

It runs in headless Chromium. `HERON_BROWSER=msedge` uses the installed Microsoft Edge,
and `firefox` or `webkit` Playwright's own builds of those engines.
