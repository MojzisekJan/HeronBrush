# Babylon.js export check

`verify_feature_lab.cjs` runs the feature lab's Babylon.js export in Chromium. The
first half loads it the way a game does: a right-handed Babylon.js scene with Havok,
then `loadLevel` from the exported `heron_babylon.js`. The second half opens the
generated `feature_lab.html` with `?verify` and drives its character controller.
`feature_lab_checks.js` holds the checks; the results are in
[feature runtime checks for Babylon.js](../../../docs/feature-runtime-babylon.md).

```sh
cargo build -p heron-brush
python3 examples/engine-checks/export_feature_lab.py --root /tmp/fl babylon
python3 examples/engine-checks/feature_lab_poses.py /tmp/fl/poses.json
HERON_CDN_CACHE=/tmp/fl/cdn node examples/engine-checks/babylon/verify_feature_lab.cjs /tmp/fl /tmp/fl/poses.json /tmp/fl/out
```

It writes `babylon-lab.png` (the three.js check's view), `babylon-stations.png` (closer,
without fog), `babylon-playtest.png` (the generated page) and `results.json`, and
exits with status 1 on any failure.

Babylon.js and Havok come from jsDelivr, at the versions the exported page names, so
the check needs internet access once. `HERON_CDN_CACHE` keeps the downloads in a
folder for later runs.

The check needs Node with `playwright`. Without Node, Python's Playwright package
ships one: run `uv run --with playwright python -c "import playwright"` once, then
use the `node` inside its `playwright/driver` folder, with `NODE_PATH` pointing at a
folder that holds a `playwright` link to `driver/package`.

It runs in headless Chromium. `HERON_BROWSER=msedge` uses the installed Microsoft Edge,
and `firefox` or `webkit` Playwright's own builds of those engines.

## Material and lighting regressions

The lighting fixture covers loose brush roughness/metallic/AO/emission maps, numeric
clear coat and anisotropy, authored HDR rotation/intensity/background visibility,
all four camera display curves, local reflection probes and baked diffuse transfer.
The browser test measures pixel changes when shadows, diffuse probes, reflection
probes and grading are toggled independently. It also opens the embedded HTML page.

```sh
python3 examples/engine-checks/babylon/prepare_lighting_checks.py /tmp/bl --binary target/release/heron-brush
HERON_CDN_CACHE=/tmp/bl/cdn node examples/engine-checks/babylon/verify_lighting.cjs /tmp/bl /tmp/bl/lighting-results
```

The extra assets are embedded in the exported JSON/HTML. Local probes use an RGBE
atlas and SH coefficients baked at export time. They blend per fragment inside the
same map-space box volumes as the editor, including box projection for reflections.
One atlas sampler supports up to eight probes without eight additional cube samplers.
Animated objects sample the static capture as they move; the bake is not recomputed
at runtime. Large interiors still need sensibly fitted probe volumes.

Camera exposure, white balance, tint, contrast, saturation and the selected ACES,
AgX, Khronos Neutral or TonyMcMapface transform are applied in a scene-linear
postprocess. TonyMcMapface embeds the same licensed LUT already used by Heron's
renderer. Babylon's BRDF, direct lights and shadow filtering remain its own, so an
export is not intended to match the editor pixel for pixel.
