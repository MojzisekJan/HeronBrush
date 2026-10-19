# Browser storage recovery checks

The production bridge is `crates/heron-brush/src/web_files.js`. Its status panel
must prioritize recovery when unsaved bytes remain in memory. A failed startup
storage open previously kept the message "Reload to try again" even after a new
map write failed. That advice hid the more urgent recovery instruction. The
panel now begins with "Keep this tab open and download a recovery copy before
reloading" whenever failed writes exist, and removes the standalone reload advice.
Other storage and physical-file notices remain visible after the recovery warning.

Run the bridge regression suite with:

```sh
node --test scripts/tests/web-storage.test.mjs
```

All 13 tests passed locally on 16 October 2026. The new case covers a synchronous
SecurityError opening IndexedDB, visible recovery controls, failed retry retention
and the logical beforeunload guard. A temporary negative control restored only the
old message behavior; 12 existing tests passed and the new test failed. Restoring
the fix returned the complete suite to green. This is prepared for the existing
CI command; remote CI was not run.

## Natural browser denial and actual recovery download

Run `python3 scripts/tests/serve_web_storage_denied.py` and open its printed
localhost URL in an installed browser. The page loads the production bridge in
an iframe with `allow-scripts allow-downloads`, without `allow-same-origin`.
IndexedDB therefore throws an actual SecurityError. The test does not replace or
mock IndexedDB. It deliberately mutates the caller's buffer after scheduling a
save, then lets the user press the real panel's Retry and Download buttons.

Local in-app Chromium checks passed natural denial, visible recovery instructions,
continued protection after Retry and exact recovery bytes. The iframe's Blob URL
observer verified the payload, while actual files in Downloads independently
confirmed the browser completed both recovery and normal-origin control downloads:

| File | Bytes | SHA-256 |
| --- | --- | --- |
| recovery.map | 49 | a973106b99eda7ff7d33e3729c6e02d2a21bfd79bd522898eb65ad73e34b2438 |
| normal-origin-control.map | 22 | b9c693de7c257df730fe49ce0e4999f3bcfdba4edad1ea4c4a6f4510f877f3a2 |

The browser automation download event timed out for both origins. That event was
not used as proof of completion; on-disk content and modification times verified
the files instead. The two generated downloads were removed only after verifying
their exact bytes, recorded timestamps and absence of symlinks. Test tabs and the
localhost server were closed. A screenshot was inspected during the UI check.

Evidence: `/tmp/heron-storage-denied-browser.json`,
`/tmp/heron-storage-recovery-negative.log`,
`/tmp/heron-storage-recovery-priority-final.log`.

This verifies the bridge and real panel, not the complete WASM editor or native
file-picker UI. Actual disk/quota exhaustion, browser closure dialogs, Firefox
and Safari, and native picker/write flows remain separate checks. The opaque
frame tests naturally unavailable storage with downloads permitted; it does not
prove recovery in an embedding that forbids downloads. No user maps or storage
were modified, and no engine, browser or dependency was installed.


Picker fallback is covered separately by `scripts/tests/web-picker.test.mjs`.
The combined command now covers 30 cases in CI, including bounded folder reads and failure completion. The actual restricted-context
Save As and download checks, and the remaining OS picker limitation, are described
in [web picker checks](web.md#picker-fallback-checks).

## Multiple failed writes

The recovery panel displays at most ten individual files per page. Each button's
tooltip contains its original virtual path. With multiple failed writes,
Download recovery ZIP creates one uncompressed UTF-8 ZIP containing the failed
write snapshots and `recovery.json`. The manifest records original paths, archive
entry names and byte counts. Normal hierarchy is retained under `files/`, so maps
with identical basenames remain separate. Unsafe paths, Windows reserved names,
case collisions and overlong names use unique entries under `unmapped/`.

This archive contains failed writes only, not a complete project. Downloading
never discards recovery bytes or removes the tab closure guard. A successful
storage retry removes the corresponding failed write. ZIP construction yields
while calculating CRC and after batches of small files. Blob parts avoid a flat
JavaScript copy of the whole archive; browser Blob memory is not bounded here.
The current ZIP32 implementation rejects 65,534 or more failed files and archives
approaching 4 GiB, with individual paginated downloads still available. ZIP64 and
automatic project restoration from the manifest remain missing features.

Run `node --test scripts/tests/web-recovery.test.mjs` for independent Python
`zipfile` interoperability, exact payloads, Unicode and path handling, asynchronous
CRC processing, file-count rejection, pagination, retained recovery state and
failed download handling. Together with storage and picker tests, 35 tests pass.
The portable-core workflow includes all three files; remote execution is unverified.

The natural-denial harness also offers Create multi-file recovery. Local Chromium
verified all three pages of 27 failed writes and the real Download recovery ZIP
button. The actual downloaded 7,437-byte archive passed Python CRC and exact-byte
checks for every file, including 25 separate `map.map` names and `český.txt`.
SHA-256: `64ba305f609d63e0601114103ab12352336983f2ecb6a2facc7c0f3cb28d8368`.
Evidence is in `/tmp/heron-recovery-zip-browser.json`; test output is in
`/tmp/heron-recovery-final.log`. This checks the production JavaScript bridge in
Chromium, not a freshly rebuilt full WASM application, Firefox or Safari.
