const {unpack, delta} = await import(new URL("playtest_protocol.js", document.baseURI));
// The web build's files (crates/heron-brush/src/web.rs): the editor's own files kept in
// IndexedDB between visits, and the browser's ways of opening and saving a user's file.
// Where there is the File System Access API (Chromium), a picked file is written back in
// place; elsewhere a file comes in through a file input and goes out as a download.
//
// Byte arrays from Rust are views into the module's memory, so every function copies
// them before its first await.

let database = null;

// The clipboard's text, for a paste from the menu, or null when the browser refuses.
export async function clipboard_text() {
  try {
    return await navigator.clipboard.readText();
  } catch (error) {
    console.warn("Heron Brush: the clipboard could not be read", error);
    return null;
  }
}

// Whether the editor should draw with WebGPU: it should where the browser hands out an
// adapter, unless the address says `?renderer=gl` (or `?renderer=wgpu`).
export async function use_webgpu() {
  const asked = new URLSearchParams(location.search).get("renderer");
  if (asked) return asked === "wgpu";
  try {
    return Boolean(navigator.gpu && (await navigator.gpu.requestAdapter()));
  } catch {
    return false;
  }
}

function open() {
  database ??= new Promise((resolve, reject) => {
    const request = indexedDB.open("heron-brush", 1);
    request.onupgradeneeded = () => {
      request.result.createObjectStore("files");
      request.result.createObjectStore("handles");
    };
    request.onsuccess = () => {
      const db = request.result;
      db.onversionchange = () => { db.close(); database = null; };
      resolve(db);
    };
    request.onerror = () => { database = null; reject(request.error); };
    request.onblocked = () => { database = null; reject(new Error("Browser storage is blocked by another tab")); };
  }).catch(error => { database = null; throw error; });
  return database;
}

async function entries(store) {
  const db = await open();
  return new Promise((resolve, reject) => {
    const found = [];
    const transaction = db.transaction(store);
    transaction.onabort = () => reject(transaction.error || new Error("Storage read aborted"));
    const cursor = transaction.objectStore(store).openCursor();
    cursor.onsuccess = () => {
      const at = cursor.result;
      if (at) {
        found.push([at.key, at.value]);
        at.continue();
      } else {
        resolve(found);
      }
    };
    cursor.onerror = () => reject(cursor.error);
  });
}

async function put(store, key, value) {
  const db = await open();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(store, "readwrite");
    transaction.objectStore(store).put(value, key);
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error);
    transaction.onabort = () => reject(transaction.error || new Error("Storage transaction aborted"));
  });
}

// The file handles of picked files, by the editor's path for them.
const handles = new Map();
// The handle of the file just picked, until the editor names its path.
let picked = null;

// Every stored file as [path, bytes], after the handles are back in `handles`.
export async function load_files() {
  try {
    for (const [path, handle] of await entries("handles")) handles.set(path, handle);
    return await entries("files");
  } catch (error) {
    storageProblem = "Browser storage could not be opened. Existing files may be unavailable. Reload to try again.";
    showStorageStatus();
    console.warn("Heron Brush: the browser's storage is not available", error);
    return [];
  }
}

// One archive avoids hundreds of HTTP requests before the editor can start.
// Validate the complete index before handing any bytes to the virtual filesystem.
export function unpack_bundle(buffer) {
  const bytes = new Uint8Array(buffer);
  if (bytes.length < 8 || String.fromCharCode(...bytes.subarray(0, 4)) !== "HBW1") throw new Error("Invalid startup archive");
  const length = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(4, true);
  if (length > 1048576 || length > bytes.length - 8) throw new Error("Invalid startup index length");
  const entries = JSON.parse(new TextDecoder("utf-8", {fatal: true}).decode(bytes.subarray(8, 8 + length)));
  if (!Array.isArray(entries)) throw new Error("Invalid startup index");
  const names = new Set(), start = 8 + length;
  for (const entry of entries) {
    if (!Array.isArray(entry) || entry.length !== 3) throw new Error("Invalid startup entry");
    const [path, offset, size] = entry;
    if (typeof path !== "string" || !/^(assets|games|fonts)\//.test(path) || path.includes("\\") || path.split("/").some(part => !part || part === "." || part === "..") || names.has(path)) throw new Error("Invalid startup path");
    if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(size) || offset < 0 || size < 0 || offset > bytes.length - start || size > bytes.length - start - offset) throw new Error("Invalid startup entry bounds");
    names.add(path);
  }
  return entries.map(([path, offset, size]) => [path, bytes.slice(start + offset, start + offset + size)]);
}
// The manifest that names the archives, fetched once per visit. Only it needs
// revalidation: content-addressed archives can remain cached indefinitely without using
// yesterday's game definitions.
let bundleIndex;
function bundle_index() {
  bundleIndex ??= fetch("bundle/index.json", {cache: "no-cache"}).then(async index => {
    if (!index.ok) throw new Error(`Startup index: HTTP ${index.status}`);
    return index.json();
  });
  return bundleIndex;
}

function valid_archive(entry, stem) {
  return typeof entry?.sha256 === "string" && /^[a-f0-9]{64}$/.test(entry.sha256)
    && entry.file === `${stem}-${entry.sha256}.bin` && Number.isSafeInteger(entry.bytes)
    && entry.bytes >= 8 && entry.bytes <= 134217728;
}

// Fetches one archive the index names and checks its length and checksum.
async function fetch_archive(entry, label) {
  for (const cache of ["force-cache", "reload"]) {
    try {
      const response = await fetch(`bundle/${entry.file}`, {cache});
      if (!response.ok) throw new Error(`${label}: HTTP ${response.status}`);
      const buffer = await response.arrayBuffer();
      if (buffer.byteLength !== entry.bytes) throw new Error(`Truncated ${label}`);
      const digest = Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", buffer)), b => b.toString(16).padStart(2, "0")).join("");
      if (digest !== entry.sha256) throw new Error(`${label} checksum mismatch`);
      return unpack_bundle(buffer);
    } catch (error) {
      if (cache === "reload") throw error;
    }
  }
}

export async function load_bundle() {
  try {
    const manifest = await bundle_index();
    if (manifest?.format !== 1 || !valid_archive(manifest, "startup")) throw new Error("Invalid startup manifest");
    const files = await fetch_archive(manifest, "Startup archive");
    window.heronLoading?.("Starting the editor", 0.97);
    return files;
  } catch (error) {
    storageProblem = "Bundled editor data could not be loaded. Reload when the connection is available.";
    showStorageStatus();
    console.warn("Heron Brush: the bundled data could not be loaded", error);
    return [];
  }
}

// The Chinese, Japanese and Korean fonts, which only those interface languages fetch.
// Without them the interface falls back to the picker faces, so a failure is not fatal.
export async function load_cjk_fonts() {
  try {
    const manifest = await bundle_index();
    if (!valid_archive(manifest?.cjk, "cjk")) throw new Error("Invalid CJK font manifest");
    return await fetch_archive(manifest.cjk, "CJK font archive");
  } catch (error) {
    console.warn("Heron Brush: the Chinese, Japanese and Korean fonts could not be loaded", error);
    return [];
  }
}

export function delete_file(path) {
  const previous = pendingWrites.get(path) || Promise.resolve();
  const deletion = previous.then(async () => {
    const db = await open();
    await new Promise((resolve, reject) => {
      const transaction = db.transaction("files", "readwrite");
      transaction.objectStore("files").delete(path);
      transaction.oncomplete = resolve;
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error || new Error("Storage deletion aborted"));
    });
    failedWrites.delete(path);
  }).catch(error => {
    storageProblem = "A file could not be removed from browser storage. It may reappear after reload.";
    console.warn("Heron Brush: could not delete", path, error);
  }).finally(() => {
    if (pendingWrites.get(path) === deletion) pendingWrites.delete(path);
    showStorageStatus();
  });
  pendingWrites.set(path, deletion);
  showStorageStatus();
  return deletion;
}

// While the open map has unsaved changes, closing or reloading the tab asks first.
let unsaved = false;
window.addEventListener("beforeunload", (event) => {
  if (unsaved || pendingWrites.size || failedWrites.size || pendingFileWrites.size) {
    event.preventDefault();
    event.returnValue = "";
  }
});
export function set_unsaved_changes(on) {
  unsaved = on;
}

// Keep writes to each path ordered: an older save must never overwrite a newer one.
const pendingWrites = new Map();
const failedWrites = new Map();
const pendingFileWrites = new Map();
let fileNotice = "";
let folderReadError = null;
let storageProblem = "";
let storageStatus;
let recoveryPage = 0;
let recoveryArchiveBusy = false;
const recoveryPageSize = 10;

// Stored ZIP records, UTF-8 names and CRC-32. Blob parts avoid a second flat
// JavaScript payload buffer. This is a backup of failed writes, not a full project.
const recoveryCrcTable = Uint32Array.from({length: 256}, (_, value) => {
  for (let bit = 0; bit < 8; bit++) value = (value >>> 1) ^ ((value & 1) ? 0xedb88320 : 0);
  return value >>> 0;
});
const recoveryYield = () => new Promise(resolve => setTimeout(resolve, 0));
export async function recovery_archive(files) {
  // Callers retain immutable byte snapshots until this asynchronous build ends.
  const entries = Array.from(files);
  if (entries.length >= 65534) throw new Error("Too many files for one recovery ZIP. Download individual files instead.");
  const encoder = new TextEncoder(), used = new Set(), manifest = [];
  const parts = [], central = [];
  let offset = 0, centralSize = 0, processedSinceYield = 0;
  async function add(name, bytes) {
    const encoded = encoder.encode(name);
    const nextOffset = offset + 30 + encoded.length + bytes.length;
    const nextCentralSize = centralSize + 46 + encoded.length;
    if (nextOffset + nextCentralSize + 22 >= 0xffffffff) throw new Error("Recovery ZIP exceeds 4 GiB. Download individual files instead.");
    let crc = 0xffffffff;
    for (let start = 0; start < bytes.length; start += 262144) {
      const end = Math.min(start + 262144, bytes.length);
      for (let i = start; i < end; i++) crc = recoveryCrcTable[(crc ^ bytes[i]) & 255] ^ (crc >>> 8);
      processedSinceYield += end - start;
      if (processedSinceYield >= 262144) {
        processedSinceYield = 0;
        await recoveryYield();
      }
    }
    crc = (crc ^ 0xffffffff) >>> 0;
    const local = new Uint8Array(30 + encoded.length), l = new DataView(local.buffer);
    l.setUint32(0, 0x04034b50, true); l.setUint16(4, 20, true);
    l.setUint16(6, 0x800, true); l.setUint16(12, 33, true);
    l.setUint32(14, crc, true); l.setUint32(18, bytes.length, true); l.setUint32(22, bytes.length, true);
    l.setUint16(26, encoded.length, true); local.set(encoded, 30);
    const directory = new Uint8Array(46 + encoded.length), d = new DataView(directory.buffer);
    d.setUint32(0, 0x02014b50, true); d.setUint16(4, 20, true); d.setUint16(6, 20, true);
    d.setUint16(8, 0x800, true); d.setUint16(14, 33, true);
    d.setUint32(16, crc, true); d.setUint32(20, bytes.length, true); d.setUint32(24, bytes.length, true);
    d.setUint16(28, encoded.length, true); d.setUint32(42, offset, true); directory.set(encoded, 46);
    parts.push(local, bytes); central.push(directory);
    offset = nextOffset; centralSize = nextCentralSize;
  }
  for (let index = 0; index < entries.length; index++) {
    const [path, bytes] = entries[index];
    if (typeof path !== "string" || !(bytes instanceof Uint8Array)) throw new Error("Invalid recovery file.");
    const components = path.replace(/^\//, "").split("/");
    let name = "files/" + components.join("/");
    const unsafe = components.some(part => !part || part === "." || part === ".."
      || /[\\\x00-\x1f<>:"|?*]/.test(part) || /[. ]$/.test(part)
      || /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\.|$)/i.test(part));
    if (unsafe || encoder.encode(name).length > 65535 || used.has(name.toLowerCase())) {
      name = `unmapped/${String(index + 1).padStart(6, "0")}.bin`;
    }
    used.add(name.toLowerCase());
    manifest.push({path, entry: name, bytes: bytes.length});
    await add(name, bytes);
    if (index % 128 === 127) await recoveryYield();
  }
  await add("recovery.json", encoder.encode(JSON.stringify({format: "heron-brush-recovery", version: 1, files: manifest}, null, 2)));
  const end = new Uint8Array(22), view = new DataView(end.buffer);
  view.setUint32(0, 0x06054b50, true);
  view.setUint16(8, entries.length + 1, true); view.setUint16(10, entries.length + 1, true);
  view.setUint32(12, centralSize, true); view.setUint32(16, offset, true);
  return new Blob([...parts, ...central, end], {type: "application/zip"});
}
function showStorageStatus() {
  if (!storageStatus) {
    storageStatus = document.createElement("aside");
    storageStatus.setAttribute("role", "status");
    storageStatus.setAttribute("aria-live", "polite");
    storageStatus.style.cssText = "position:fixed;bottom:8px;right:8px;z-index:10000;max-width:420px;padding:10px;background:#252a30;color:white;border:1px solid #aaa;font:14px sans-serif";
    document.body.append(storageStatus);
  }
  storageStatus.replaceChildren();
  const failed = failedWrites.size;
  storageStatus.hidden = !storageProblem && !fileNotice && !failed && !pendingWrites.size && !pendingFileWrites.size;
  const message = document.createElement("div");
  const recovery = failed
    ? `Browser storage failed for ${failed} file(s). Keep this tab open and download a recovery copy before reloading.`
    : "";
  // A startup failure must not recommend reload while unsaved recovery bytes
  // exist only in memory. Keep that warning visible alongside other notices.
  const problem = failed ? storageProblem.replace(" Reload to try again.", "") : storageProblem;
  message.textContent = [recovery, problem, fileNotice].filter(Boolean).join(" ")
    || (pendingFileWrites.size ? "Saving the selected file…" : "Saving to browser storage…");
  storageStatus.append(message);
  if (fileNotice) {
    const dismiss = document.createElement("button");
    dismiss.textContent = "Dismiss file notice";
    dismiss.onclick = () => { fileNotice = ""; showStorageStatus(); };
    storageStatus.append(dismiss);
  }
  if (failed) {
    const retry = document.createElement("button");
    retry.textContent = "Retry storage";
    retry.onclick = () => {
      for (const [path, bytes] of failedWrites) store_file(path, bytes);
    };
    storageStatus.append(retry);
    if (failed > 1) {
      const archive = document.createElement("button");
      archive.textContent = recoveryArchiveBusy ? "Preparing recovery ZIP…" : "Download recovery ZIP";
      archive.disabled = recoveryArchiveBusy;
      archive.onclick = async () => {
        if (recoveryArchiveBusy) return;
        recoveryArchiveBusy = true;
        const snapshot = Array.from(failedWrites);
        showStorageStatus();
        try {
          const blob = await recovery_archive(snapshot);
          const name = `heron-recovery-${new Date().toISOString().replace(/[:.]/g, "-")}.zip`;
          download(name, blob);
          fileNotice = `Requested ${name} containing ${snapshot.length} failed file(s). Check your downloads before closing this tab.`;
        } catch (error) {
          fileNotice = `Could not create recovery ZIP: ${error?.message || error}. Individual downloads remain available.`;
        } finally {
          recoveryArchiveBusy = false;
          showStorageStatus();
        }
      };
      storageStatus.append(archive);
    }
    const pages = Math.ceil(failed / recoveryPageSize);
    recoveryPage = Math.min(recoveryPage, pages - 1);
    if (pages > 1) {
      const page = document.createElement("div");
      page.textContent = `Recovery files: page ${recoveryPage + 1} of ${pages}`;
      for (const [label, step] of [["Previous recovery files", -1], ["Next recovery files", 1]]) {
        const button = document.createElement("button"); button.textContent = label;
        button.disabled = recoveryPage + step < 0 || recoveryPage + step >= pages;
        button.onclick = () => { recoveryPage += step; showStorageStatus(); };
        page.append(button);
      }
      storageStatus.append(page);
    }
    let index = 0;
    for (const [path, bytes] of failedWrites) {
      if (index++ < recoveryPage * recoveryPageSize) continue;
      if (index > (recoveryPage + 1) * recoveryPageSize) break;
      const rescue = document.createElement("button");
      const name = path.split("/").pop() || "recovery.bin";
      rescue.textContent = `Download ${name}`;
      rescue.title = path;
      rescue.onclick = () => download(name, bytes);
      storageStatus.append(rescue);
    }
  }
}
export function store_file(path, bytes) {
  const copy = bytes.slice();
  const previous = pendingWrites.get(path) || Promise.resolve();
  const write = previous.then(() => put("files", path, copy)).then(() => {
    failedWrites.delete(path);
  }, error => {
    failedWrites.set(path, copy);
    console.warn("Heron Brush: could not store", path, error);
  }).finally(() => {
    if (pendingWrites.get(path) === write) pendingWrites.delete(path);
    showStorageStatus();
  });
  pendingWrites.set(path, write);
  showStorageStatus();
  return write;
}

function accept(extensions) {
  return extensions.length ? [{ description: extensions.map((e) => "." + e).join(", "), accept: { "application/octet-stream": extensions.map((e) => "." + e) } }] : undefined;
}

// Browser APIs can exist but be unavailable in an embedding or privacy context.
const unavailablePicker = error => ["SecurityError", "NotSupportedError"].includes(error?.name);
let cancelFallbackPicker = null;
function fallbackPicker(title, build) {
  cancelFallbackPicker?.();
  return new Promise(resolve => {
    const dialog = document.createElement("dialog");
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-label", title);
    dialog.style.cssText = "position:fixed;top:20%;left:50%;transform:translateX(-50%);margin:0;max-width:calc(100vw - 40px);width:400px;padding:20px;background:#252a30;color:white;border:1px solid #aaa;border-radius:6px;font:14px sans-serif;z-index:10001";
    const form = document.createElement("form");
    const heading = document.createElement("h2"); heading.textContent = title;
    form.append(heading); dialog.append(form);
    let finished = false;
    const finish = value => {
      if (finished) return;
      finished = true;
      cancelFallbackPicker = null;
      dialog.close?.(); dialog.remove(); resolve(value);
    };
    cancelFallbackPicker = () => finish(null);
    const focus = build(form, finish);
    const cancel = document.createElement("button");
    cancel.type = "button"; cancel.textContent = "Cancel"; cancel.onclick = () => finish(null);
    form.append(cancel);
    dialog.oncancel = event => { event.preventDefault(); finish(null); };
    dialog.onkeydown = event => event.stopPropagation();
    dialog.onkeyup = event => event.stopPropagation();
    document.body.append(dialog);
    if (typeof dialog.showModal === "function") dialog.showModal();
    else { dialog.setAttribute("open", ""); dialog.style.display = "block"; }
    focus?.focus();
  });
}
function openCopy(extensions) {
  return fallbackPicker("Open a file copy", (form, finish) => {
    const note = document.createElement("p"); note.textContent = "Choose a file to open a copy. Saving will download a new file.";
    const label = document.createElement("label"); label.textContent = "File to open ";
    const input = document.createElement("input"); input.type = "file";
    input.accept = extensions.map(e => "." + e).join(","); label.append(input);
    const error = document.createElement("p"); error.setAttribute("role", "alert");
    form.append(note, label, error);
    form.onsubmit = event => event.preventDefault();
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return finish(null);
      try { finish([file.name, new Uint8Array(await file.arrayBuffer())]); }
      catch { error.textContent = "Could not read this file. Select another file or cancel."; }
    };
    input.oncancel = () => finish(null);
    return input;
  });
}
function downloadName(suggested) {
  return fallbackPicker("Save a file copy", (form, finish) => {
    const note = document.createElement("p"); note.textContent = "The file will be downloaded. Choose a file name.";
    const label = document.createElement("label"); label.textContent = "File name ";
    const input = document.createElement("input"); input.type = "text";
    input.required = true; input.value = suggested; input.autocomplete = "off"; label.append(input);
    const error = document.createElement("p"); error.setAttribute("role", "alert");
    const save = document.createElement("button"); save.type = "button"; save.textContent = "Save";
    form.append(note, label, error, save);
    const submit = event => {
      event.preventDefault();
      const name = input.value.trim();
      if (!name || name === "." || name === ".." || /[\/\\\0\r\n]/.test(name)) {
        error.textContent = "Enter a file name without folder separators."; input.focus(); return;
      }
      finish(name);
    };
    // No network form submission: this also works in sandboxed frames that
    // permit scripts/downloads but do not permit HTML form submissions.
    save.onclick = submit;
    form.onsubmit = submit;
    input.onkeydown = event => { if (event.key === "Enter") submit(event); };
    return input;
  });
}

// [name, bytes] of a file the user picks, or null when they cancel.
export async function pick_open(extensions) {
  cancelFallbackPicker?.();
  picked = null;
  if (typeof window.showOpenFilePicker === "function") {
    try {
      const [handle] = await window.showOpenFilePicker({ types: accept(extensions) });
      const file = await handle.getFile();
      picked = handle;
      return [file.name, new Uint8Array(await file.arrayBuffer())];
    } catch (error) {
      if (!unavailablePicker(error)) {
        if (!["AbortError", "NotAllowedError"].includes(error?.name)) {
          fileNotice = "Could not open the selected file. Try again or choose another file."; showStorageStatus();
        }
        return null;
      }
    }
  }
  return openCopy(extensions);
}

// A folder the user picks, as [name, [[path inside it, bytes], ...]], or null when they
// cancel. The browser hands over copies of the files; nothing is written back.
export async function pick_folder() {
  cancelFallbackPicker?.();
  picked = null;
  const previousFolderError = folderReadError;
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.webkitdirectory = true;
    input.onchange = async () => {
      const problem = detail => {
        folderReadError = {message: `Could not import the selected folder. ${detail} No files were imported. Try selecting the folder again.`};
        fileNotice = folderReadError.message;
        showStorageStatus();
      };
      try {
        const files = Array.from(input.files);
        if (!files.length) return resolve(null);
        // Validate all browser paths before reading bytes or returning an import.
        const paths = files.map(file => {
          if (typeof file.webkitRelativePath !== "string") return null;
          const parts = file.webkitRelativePath.split("/");
          return parts.length < 2 || parts.some(part => !part || part === "." || part === ".." || /[\\\0\r\n]/.test(part)) ? null : parts;
        });
        if (paths.some(parts => !parts)) {
          problem("The browser did not provide usable folder paths."); return resolve(null);
        }
        const name = paths[0][0];
        if (paths.some(parts => parts[0] !== name)) {
          problem("The selection contains multiple folder roots."); return resolve(null);
        }
        const entries = new Array(files.length);
        let next = 0;
        let failed = false;
        const read = async () => {
          while (!failed && next < files.length) {
            const index = next++;
            const relative = paths[index].slice(1).join("/");
            try {
              const bytes = new Uint8Array(await files[index].arrayBuffer());
              if (!failed) entries[index] = [relative, bytes];
            } catch {
              if (!failed) problem(`Could not read ${relative}.`);
              failed = true;
            }
          }
        };
        // Bound simultaneous browser IO and temporary promises. The completed
        // payload still contains the entire folder required by the Rust API.
        await Promise.all(Array.from({length: Math.min(4, files.length)}, read));
        if (failed) return resolve(null);
        // An older import must not dismiss a newer failure, even if both
        // failures have the same text. Compare the notice's identity.
        if (folderReadError === previousFolderError) {
          if (fileNotice === folderReadError?.message) fileNotice = "";
          folderReadError = null;
        }
        showStorageStatus();
        resolve([name, entries]);
      } catch {
        problem("Folder files could not be read.");
        resolve(null);
      }
    };
    input.oncancel = () => resolve(null);
    input.click();
  });
}

// The name of the file the user chooses to save as, or null when they cancel. Without a
// usable save picker an in-page dialog asks for a name; each save is downloaded.
export async function pick_save(suggested, extensions) {
  cancelFallbackPicker?.();
  picked = null;
  if (typeof window.showSaveFilePicker === "function") {
    try {
      const handle = await window.showSaveFilePicker({ suggestedName: suggested, types: accept(extensions) });
      picked = handle;
      return handle.name;
    } catch (error) {
      if (!unavailablePicker(error)) {
        if (!["AbortError", "NotAllowedError"].includes(error?.name)) {
          fileNotice = "Could not choose a save location. Try again."; showStorageStatus();
        }
        return null;
      }
    }
  }
  return downloadName(suggested);
}

// Keep the original basename while isolating independently picked files. Chromium
// handles identify a physical file across visits; fallback pickers provide no identity.
export async function picked_file_path(root, name) {
  if (picked) {
    for (const [path, handle] of handles) {
      try {
        if (await picked.isSameEntry(handle)) return path;
      } catch { /* A stale handle must not prevent opening a different file. */ }
    }
  }
  return `${root}/${globalThis.crypto.randomUUID()}/${name}`;
}

// Ties the file just picked to the editor's path for it.
export function adopt_handle(path) {
  if (picked) {
    handles.set(path, picked);
    put("handles", path, picked).catch(() => {});
  } else {
    handles.delete(path);
  }
  picked = null;
}

// Hands the browser a file to save in its downloads.
export function download(name, bytes) {
  const url = URL.createObjectURL(new Blob([bytes.slice()], { type: "application/octet-stream" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
}

// Writes a user's file: into the picked file where there is one and the browser allows
// it, as a download otherwise.
export function save_user_file(path, name, bytes) {
  const copy = bytes.slice();
  const handle = handles.get(path);
  const previous = pendingFileWrites.get(path) || Promise.resolve();
  const write = previous.then(async () => {
    if (handle) {
      let writable;
      try {
        const mode = { mode: "readwrite" };
        if ((await handle.queryPermission(mode)) !== "granted" && (await handle.requestPermission(mode)) !== "granted") {
          throw new Error("permission refused");
        }
        writable = await handle.createWritable();
        await writable.write(copy);
        await writable.close();
        return;
      } catch (error) {
        try { await writable?.abort(); } catch { /* The stream may already be closed. */ }
        fileNotice = `Could not update ${name} in place. A recovery download was requested; check your downloads.`;
        console.warn("Heron Brush: could not write the file, downloading it instead", error);
      }
    }
    download(name, copy);
  }).catch(error => {
    // A failed download must leave a recoverable copy and must not reject the next save.
    failedWrites.set(path, copy);
    fileNotice = `Could not download ${name}. Use the recovery download button before closing this tab.`;
    console.warn("Heron Brush: could not store recovery download", error);
  }).finally(() => {
    if (pendingFileWrites.get(path) === write) pendingFileWrites.delete(path);
    showStorageStatus();
  });
  pendingFileWrites.set(path, write);
  showStorageStatus();
  return write;
}

// One isolated channel per editor tab. Retain the last good export for late/reloaded viewers.
const playtestId = globalThis.crypto.randomUUID();
let playtestChannel, playtestWindow, playtestLatest, playtestData, playtestTemplate;
let playtestRevision = 0;
export function open_playtest() {
  if (!playtestChannel) {
    playtestChannel = new BroadcastChannel("heron-playtest-" + playtestId);
    playtestChannel.onmessage = event => {
      if (event.data?.type === "ready" && playtestLatest) playtestChannel.postMessage(playtestLatest);
    };
  }
  playtestWindow = window.open(new URL("play.html#" + playtestId, document.baseURI), "heron-playtest-" + playtestId);
  return Boolean(playtestWindow);
}
export function playtest_active() { return Boolean(playtestWindow && !playtestWindow.closed); }
export function publish_playtest(html, mapKey) {
  const exported=unpack(html), previous=playtestRevision++;
  const revision=playtestRevision;
  const message=playtestLatest && playtestLatest.mapKey===mapKey && playtestTemplate===exported.template
    ? {type:"patch",patch:delta(playtestData,exported.data),mapKey,base:previous,revision}
    : {type:"level",html,mapKey,revision};
  playtestLatest={type:"level",html,mapKey,revision};
  playtestData=exported.data;playtestTemplate=exported.template;
  playtestChannel?.postMessage(message);
}

// Dedicated export worker. The editor polls answers without parsing large HTML here.
let exportWorker, exportBusy = false, exportAnswer, exportEpoch = 0, exportId = 0;
let workerRevision = 0, exportMetrics;
export function playtest_export_epoch() { return exportEpoch; }
export function playtest_export_busy() { return exportBusy; }
export function take_playtest_export() { const answer = exportAnswer; exportAnswer = undefined; return answer; }
export function playtest_export_metrics() { return exportMetrics; }
export function cancel_playtest_export(close = false) {
  exportId++; exportAnswer = undefined;
  if (exportWorker && (exportBusy || close)) {
    exportWorker.terminate(); exportWorker = undefined; exportEpoch++; workerRevision = 0;
  }
  exportBusy = false;
}
export function submit_playtest_export(snapshot, directory, options, files, mapKey, prepareMs) {
  if (exportBusy) return false;
  const id = ++exportId, started = performance.now();
  exportBusy = true;
  exportMetrics = {prepareMs, fileCount:files.length, fileBytes:files.reduce((sum, [, bytes])=>sum+(bytes?.byteLength || 0),0)};
  const fail = error => {
    exportWorker?.terminate(); exportWorker = undefined; exportEpoch++; workerRevision = 0;
    exportBusy = false; exportAnswer = {id, error:String(error)};
  };
  try {
    if (!exportWorker) {
      const url = new URL('playtest_worker.mjs', document.baseURI);
      // Refresh the tiny bootstrap when creating a worker; its manifest versions
      // the heavy module and WASM independently of the editor's own build.
      url.searchParams.set('session', crypto.randomUUID());
      exportWorker = new Worker(url, {type:'module'});

    }
    // The callback must use this job's timing even when reusing an idle worker.
    const worker = exportWorker;
    worker.onerror = event => {
      event.preventDefault();
      if (worker === exportWorker && id === exportId) fail(event.message || 'Export worker failed');
    };
    worker.onmessage = ({data}) => {
      if (worker !== exportWorker || data.id !== exportId) return;
      exportBusy = false;
      if (data.error) { fail(data.error); return; }
      workerRevision = data.message.revision;
      exportMetrics = {...exportMetrics, roundTripMs:performance.now()-started, exportMs:data.exportMs};
      exportAnswer = data;
    };
    worker.postMessage({id, snapshot, directory, options, files, mapKey,
      base:playtestRevision, revision:playtestRevision+1,
      previousHtml:workerRevision === playtestRevision ? undefined : playtestLatest?.html,
      previousMapKey:playtestLatest?.mapKey}, files.filter(([,bytes])=>bytes).map(([,bytes])=>bytes.buffer));
  } catch (error) { fail(error); }
  return true;
}
export function publish_playtest_export(answer) {
  const message = answer.message;
  if (!message || message.revision !== playtestRevision+1 || (message.type === 'patch' && message.base !== playtestRevision)) return false;
  playtestRevision = message.revision;
  playtestLatest = {type:'level', html:answer.html, mapKey:message.mapKey, revision:playtestRevision};
  // The synchronous publisher remains available for protocol fixtures.
  playtestData = playtestTemplate = undefined;
  playtestChannel?.postMessage(message);
  return true;
}
