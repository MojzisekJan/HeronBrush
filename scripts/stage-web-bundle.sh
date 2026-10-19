#!/bin/sh
# Trunk stages one startup archive; the browser validates and preloads its entries.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
staging=${1:-${TRUNK_STAGING_DIR:?run from trunk, or name the staging folder}}
bundle="$staging/bundle"
rm -rf "$bundle"
python3 "$root/scripts/pack_web_bundle.py" "$bundle"

python3 "$root/scripts/stage-web-worker.py" "$staging"
