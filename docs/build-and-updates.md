# Build, launch, shortcuts and updates

## Project build and launch profiles

On desktop, **File → Build and run** opens named profiles. Add, duplicate or reorder steps, then save the profile. Run also saves pending profile edits. Profiles live in `.heron-brush/build.json` beside the project export settings. An invalid or newer file produces an error and is never silently overwritten.

Each profile has a working directory and ordered steps:

- Export map writes a snapshot of the current document, including unsaved edits and editor metadata, to a different file. It does not save the source map.
- Export target uses the project's existing engine export options and class bindings. A step can override the destination directory.
- Copy file copies one file and preserves its permissions.
- Run starts an executable with an argument list and optional working directory. A final Run step can launch your game.

Arguments occupy one line each in the editor. Spaces belong to the argument; do not add shell quotes. Heron performs no shell expansion. If a tool requires a shell, choose that shell explicitly as the executable. Relative executable paths containing a directory resolve against the step's working directory; bare names use PATH.

Available substitutions are `${PROJECT_DIR}`, `${MAP_PATH}`, `${MAP_DIR}`, `${MAP_BASE_NAME}`, `${WORK_DIR_PATH}`, `${GAME_DIR_PATH}`, `${APP_DIR_PATH}` and `${CPU_COUNT}`. Unknown or unfinished substitutions fail the step. Export files and relative paths resolve against the profile working directory.

Example, after configuring the Godot export for your own project:

```json
{
  "version": 1,
  "profiles": [{
    "name": "Export and launch Godot",
    "workdir": "${PROJECT_DIR}",
    "steps": [
      {"type": "export_target", "engine": "godot", "directory": "godot/maps"},
      {"type": "run", "program": "godot", "args": ["--path", "${PROJECT_DIR}/godot"]}
    ]
  }]
}
```

Work runs in a background thread. The dialog shows stdout, stderr and completion status. A failing step prevents later steps from running. Stop terminates the owned process group on Unix; on Windows it terminates the child process. Closing the dialog leaves work running; exiting Heron cancels it. File copying and multi-file export check cancellation between chunks or files. A running engine export operation finishes its current computation before observing cancellation.

This first version does not support directory copies, glob patterns, expression language in profiles or compilation-profile import. The web editor cannot start local executables. A real compiler or game must already be installed; profiles never install tools.

## Custom shortcuts

**Preferences → Keyboard → Edit shortcuts** edits 30 common commands and tools over the Heron or Blender preset. Use names such as `Ctrl+Shift+B` or `F5`. Empty disables a binding; Reset restores its preset. Apply saves validated bindings with the other preferences.

Global commands also work while a text field has focus and require Ctrl, Alt or a function key. Map commands leave text entry alone. Conflict detection covers the editable command catalogue. Other preset gestures, camera controls, animation timeline controls and dynamic entity/tag commands remain outside that catalogue. Native text-field clipboard shortcuts remain available. Menu labels for editable commands follow the configured bindings.

## Diagnostic actions

An issue row can be hidden for the current document session; **Show hidden issues** restores hidden findings. An unchanged finding stays hidden after a map edit. Opening a different document resets hidden findings.

Two findings offer deterministic fixes: invalid brush UV values and an entity property with an empty name. Fix checks that the finding still exists, changes only its object and adds one Undo entry. It preserves selection and other properties. Missing assets, unknown classes and broken entity links still require a project-specific decision.

## Application updates

**Help → Application updates** checks the configured public GitHub repository. The default is `MojzisekJan/HeronBrush`, reserved for the user's planned fresh repository. Creating that repository and publishing a release are separate work. Until then a missing repository or an empty release list reports that no published release is available.

Checking on startup is off by default. Checking reads release metadata only. Download and installation require clicking their button; the editor never installs automatically. Prereleases are excluded unless enabled. Network errors appear in the dialog without blocking editing.

For direct Linux executable replacement, publish a raw ELF asset named `heron-brush-linux-x86_64` or `heron-brush-linux-aarch64`. GitHub release metadata must include the asset's SHA-256 digest and size. The updater verifies both and the ELF architecture before replacing the executable, retaining an `.update-backup` file. Save work and restart manually. AppImage packages and other platforms use the release download page. Release packaging must also provide the assets required by the chosen distribution; replacing an executable does not update external asset directories.

Local tests cover release selection, digest refusal, cancellation, wrong architecture, preservation of an existing staging file and staged replacement with a synthetic ELF fixture. End-to-end download, installation and restart against an actual HeronBrush release remain unperformed because no release exists. No engine or release artifact was downloaded during implementation.
