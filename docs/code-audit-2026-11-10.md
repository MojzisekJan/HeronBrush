# Heron Brush code audit

Open findings for repairs on another machine. This branch contains audit documentation only; none of the findings below has been fixed here. Keep this file updated as findings are confirmed, repaired or ruled out.

**Status:** audit in progress, 15 confirmed findings. Last checkpoint: 10 November 2026, 00:41 UTC.

**Audited source:** `f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae`. Source references below are pinned to this revision.

**Priority:** P1 means potential data loss, application failure or publishing into the wrong project; P2 means incorrect persistent state or behavior with a narrower impact. Priorities are triage judgments, not measured frequencies.

## Findings

| ID | Priority | Finding | Status |
| --- | --- | --- | --- |
| A001 | P1 | [Concurrent layer removal loses a remote brush](#a001) | Open |
| A002 | P1 | [Tiny malformed MDL header panics during allocation](#a002) | Open |
| A003 | P2 | [Remote edits accept duplicate node identities](#a003) | Open |
| A004 | P2 | [Layer export omission is not shared with connected guests](#a004) | Open |
| A005 | P1 | [Failed CLI merge write truncates the original map](#a005) | Open |
| A006 | P1 | [Oversized material texture produces GPU validation failure](#a006) | Open |
| A007 | P1 | [Concurrent patch edits silently duplicate the same identity](#a007) | Open |
| A008 | P2 | [Source line numbers create false entity merge conflicts](#a008) | Open |
| A009 | P1 | [Clipboard model references lose their source directory](#a009) | Open |
| A010 | P1 | [Godot live link omits imported model geometry](#a010) | Open |
| A011 | P1 | [Live link uses another projects global export profile](#a011) | Open |
| A012 | P2 | [Saving provisional text can leave an unmarked disk mismatch](#a012) | Open |
| A013 | P1 | [Saving nested groups breaks their hierarchy](#a013) | Open |
| A014 | P1 | [Linked duplicates reuse nested persistent group IDs](#a014) | Open |
| A015 | P2 | [Collaboration buffers frames and queues without bounds](#a015) | Open |

## A001

**P1 — Concurrent layer removal loses a remote brush**

The host removes an earlier layer while a guest inserts into a surviving layer. Added routes by a numeric layer index; the host silently ignores the now-invalid index.

**Reproduction and result:** Host has zero brushes and guest one after convergence. Normal layer insertion control passes.

**Repair direction:** Route edits by stable layer identity and define handling for concurrent removal. Test host, sender and a second guest.

**Source:** [crates/editor_core/src/document/history.rs:670](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/document/history.rs#L670), [crates/editor_core/src/document/history.rs:564](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/document/history.rs#L564).

**Regression probes:** `concurrent_layer_removal_keeps_a_new_brush_on_its_surviving_layer`.

## A002

**P1 — Tiny malformed MDL header panics during allocation**

Skin width and height from the header reach Vec::with_capacity without a checked decoded-size bound.

**Reproduction and result:** A 112-byte MDL triggers capacity overflow; catch_unwind captures it without allocating a large buffer.

**Repair direction:** Validate header dimensions, products and decoded budgets before allocation; cover related model parsers.

**Source:** [crates/map_model/src/entity_model.rs:229](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/map_model/src/entity_model.rs#L229), [crates/map_model/src/entity_model.rs:147](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/map_model/src/entity_model.rs#L147).

**Regression probes:** `malformed_mdl_header_returns_an_error_instead_of_panicking`.

The exact small malformed input can be recreated without external files:

```rust
let mut bytes = vec![0_u8; 112];
bytes[..4].copy_from_slice(b"IDPO");
bytes[4..8].copy_from_slice(&6_i32.to_le_bytes());
bytes[52..56].copy_from_slice(&i32::MAX.to_le_bytes());
bytes[56..60].copy_from_slice(&i32::MAX.to_le_bytes());
bytes[68..72].copy_from_slice(&1_i32.to_le_bytes());
let result = std::panic::catch_unwind(|| map_model::entity_model::read(&bytes, None));
assert!(result.is_ok());
assert!(result.unwrap().is_err());
```

## A003

**P2 — Remote edits accept duplicate node identities**

WireEdit conversion lacks the uniqueness validation applied to a full WireWorld.

**Reproduction and result:** A decoded Added edit inserts two nodes with NodeId(42).

**Repair direction:** Validate uniqueness within edits and against retained document nodes, including nested batches and children.

**Source:** [crates/editor_core/src/collab/wire.rs:468](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/collab/wire.rs#L468).

**Regression probes:** `network_edit_rejects_duplicate_node_identities`.

## A004

**P2 — Layer export omission is not shared with connected guests**

The omit flag mutates the layer directly and only marks modified; no history edit or collaboration callback is emitted.

**Reproduction and result:** Host flag becomes true; an already-connected guest remains false.

**Repair direction:** Make export-affecting layer changes normal edits and apply their state remotely; check live-link revision invalidation too.

**Source:** [crates/editor_core/src/document/layers.rs:178](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/document/layers.rs#L178).

**Regression probes:** `layer_export_omission_is_shared_with_existing_guests`.

## A005

**P1 — Failed CLI merge write truncates the original map**

The merge command writes directly to the output, which defaults to the existing ours file.

**Reproduction and result:** A subprocess with a 512-byte file-size soft limit returns failure but leaves the original 4387-byte file truncated to 512 bytes.

**Repair direction:** Use atomic replacement. Keep failure injection isolated in a child process and preserve the previous file on all write errors.

**Source:** [crates/editor_core/src/cli.rs:261](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/cli.rs#L261).

**Regression probes:** `failed_merge_output_preserves_the_previous_map`.

## A006

**P1 — Oversized material texture produces GPU validation failure**

Material mip upload creates textures without checking the active device dimensions. Environment upload already has a size adjustment path.

**Reproduction and result:** On Apple M4 with the app-equivalent 8192 limit, a 64x1 image passes and an 8193x1 PNG fails texture validation. The default uncaptured-error path would panic.

**Repair direction:** Clamp or reject oversized decoded material images consistently before GPU creation and expose a usable diagnostic.

**Source:** [crates/render_wgpu/src/scene.rs:1082](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/render_wgpu/src/scene.rs#L1082).

**Regression probes:** `an_image_wider_than_the_gpu_limit_is_handled_without_validation_failure`.

## A007

**P1 — Concurrent patch edits silently duplicate the same identity**

Patches are merged as a content multiset instead of by persistent identity.

**Reproduction and result:** Two edits of patch ID 42 produce two patches with ID 42 and zero conflicts; a one-sided patch edit control passes.

**Repair direction:** Merge patches by identity and report divergent changes rather than retaining duplicate identities.

**Source:** [crates/map_model/src/merge.rs:343](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/map_model/src/merge.rs#L343).

**Regression probes:** `patch_edits_on_both_branches_report_a_conflict_without_duplicate_ids`.

## A008

**P2 — Source line numbers create false entity merge conflicts**

Derived entity equality includes diagnostic source line positions.

**Reproduction and result:** Adding only a leading comment on one side and deleting an entity on the other reports an edit/delete conflict and retains the entity.

**Repair direction:** Compare semantic map content independently of source positions, including nested brushes and patches.

**Source:** [crates/map_model/src/merge.rs:275](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/map_model/src/merge.rs#L275), [crates/map_model/src/io/map_reader.rs:38](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/map_model/src/io/map_reader.rs#L38).

**Regression probes:** `leading_comments_do_not_turn_a_clean_entity_deletion_into_a_conflict`.

## A009

**P1 — Clipboard model references lose their source directory**

Copy serializes relative _heron_model paths unchanged; paste resolves them relative to the destination map.

**Reproduction and result:** A copied 32-unit model silently becomes a 128-unit model with the same filename in another map directory. In an unsaved map it becomes unresolvable. Same-map copy passes.

**Repair direction:** Preserve source asset identity in clipboard/prefab transfer; rebase or bundle references before inserting and retain undo semantics.

**Source:** [crates/editor_core/src/document/clipboard.rs:16](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/document/clipboard.rs#L16), [crates/editor_core/src/document/clipboard.rs:151](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/document/clipboard.rs#L151), [crates/editor_core/src/document/imported_models.rs:210](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/document/imported_models.rs#L210).

**Regression probes:** `copying_an_imported_model_to_another_map_keeps_the_original_asset`, `copying_an_imported_model_to_an_unsaved_map_keeps_it_resolvable`.

## A010

**P1 — Godot live link omits imported model geometry**

Only the Flax live-link route uses an export snapshot. The Godot text-only route skips imported_export preparation.

**Reproduction and result:** Ordinary Godot export contains one triangle and MeshInstance3D; live publishing creates only a Marker3D with _heron_model metadata.

**Repair direction:** Share imported-asset preparation between ordinary and live export. Audit the parallel Unity and Defold routes separately.

**Source:** [crates/editor_core/src/godot_live.rs:490](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/godot_live.rs#L490), [crates/editor_core/src/godot_live.rs:573](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/godot_live.rs#L573), [crates/editor_core/src/export.rs:514](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/export.rs#L514).

**Regression probes:** `live_link_keeps_imported_model_geometry`.

## A011

**P1 — Live link uses another projects global export profile**

Live-link start and addon installation call Settings::load while the export UI stores a map-specific project profile.

**Reproduction and result:** With only a project profile, live link refuses to start. With another project in global settings, it publishes the current map into that other project. Both cases use isolated profiles and temporary projects.

**Repair direction:** Resolve options from the active document profile consistently for installation, startup and restart.

**Source:** [crates/editor_core/src/godot_live.rs:183](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/godot_live.rs#L183), [crates/editor_core/src/godot_live.rs:247](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/godot_live.rs#L247), [crates/editor_core/src/export_settings.rs:111](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/export_settings.rs#L111).

**Regression probes:** `live_link_uses_the_current_maps_project_profile`, `live_link_does_not_publish_to_another_projects_global_profile`.

## A012

**P2 — Saving provisional text can leave an unmarked disk mismatch**

Save serializes provisional text and clears modified; cancellation removes preview nodes without restoring dirty state.

**Reproduction and result:** Current-HEAD core probe ends with zero in-memory brushes, six saved brushes and modified=false. An isolated native UI run with embedded dialogs independently reproduces typing HI, Cmd+S, then Escape: six saved brushes, zero visible brushes, modified=false.

**Repair direction:** Define save semantics during provisional editing: defer/reject save, save committed state, or correctly track the restored state against the saved snapshot.

**Source:** [crates/editor_core/src/document/text.rs:196](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/document/text.rs#L196), [crates/editor_core/src/document.rs:3265](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/document.rs#L3265), [crates/heron-brush/src/lib.rs:647](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/heron-brush/src/lib.rs#L647).

**Regression probes:** `saving_then_cancelling_text_preview_does_not_leave_an_unmarked_disk_mismatch`.

**Validation scope:** UI used an existing 2026-11-08 binary; relevant UI source last changed 2026-11-06. Core regression was compiled against current HEAD source. Native separate-dialog focus handling was not exercised.

## A013

**P1 — Saving nested groups breaks their hierarchy**

The writer emits a nested group before its parent, while `build_tree` places groups by removing them from the end of the pending list. It therefore places the parent first and removes the parent lookup before processing the child. The child is then reported as having a missing parent and is moved to the default layer.

**Reproduction and result:** Create an outer group containing an inner group containing one point entity. Serialize with `Document::to_map_text`, parse, and rebuild the tree. One top-level group containing the entity becomes two top-level groups with content counts `[0, 1]`; the loader reports `MissingParent`. No linked duplication or concurrent editing is required. Flat groups pass the save/reopen control.

This minimal map starts with a valid nested hierarchy. Open it, save it, then reopen the saved output:

```text
{ "classname" "worldspawn" }
{ "classname" "func_group" "_tb_type" "_tb_group" "_tb_id" "1" "_tb_name" "Outer" }
{ "classname" "func_group" "_tb_type" "_tb_group" "_tb_id" "2" "_tb_name" "Inner" "_tb_group" "1" }
{ "classname" "light" "origin" "0 0 0" "_tb_group" "2" }
```

**Repair direction:** Resolve hierarchy independently of entity order, with explicit missing-parent and cycle handling, or ensure writer and reader ordering agree. Preserve arbitrary valid input ordering. Add nested groups with point entities, brush entities and patches to round-trip tests.

**Source:** [group placement](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/map_model/src/node.rs#L510), [recursive group serialization](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/map_model/src/node.rs#L963).

**Regression probe:** `nested_group_hierarchy_survives_ordinary_save_and_reopen`.

## A014

**P1 — Linked duplicates reuse nested persistent group IDs**

`create_linked_duplicate` allocates a fresh persistent ID for the outer group, but `deep_renumber` changes only `NodeId` values in the subtree. It leaves each nested `Group.id` unchanged. The source and copy then share `_tb_id` values used by saved parent references.

**Reproduction and result:** Make a linked duplicate of a group containing a nested group and point entity. The four groups have only three unique persistent IDs before any save. After save/reopen, both point entities resolve to the first nested group with that ID. The separate ordering defect in A013 additionally detaches the nested groups; fixing A013 alone does not remove the identity collision. A flat linked group has distinct IDs and preserves membership.

**Repair direction:** Allocate fresh persistent group IDs throughout duplicated subtrees and preserve the intended linked-set identities separately. Cover structural linked-group rebuilds as well as initial duplication.

**Source:** [deep renumbering](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/document.rs#L924), [linked duplicate construction](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/document/groups.rs#L693).

**Regression probes:** `linked_nested_duplicate_allocates_fresh_persistent_group_ids`, `linked_nested_group_keeps_entity_membership_after_reopen`.

## A015

**P2 — Collaboration buffers frames and queues without bounds**

The TCP reader uses unrestricted `BufRead::lines`, then sends complete strings to an unbounded channel. The UI drains all queued messages into a vector. Iroh uses the same unbounded line/channel pattern; WebSocket input and output queues are also unbounded. A peer can grow memory use before JSON validation, including while transaction handling defers UI polling.

**Bounded observation:** A loopback TCP peer sent one malformed frame of 8 MiB plus four bytes, followed by eight 1 MiB frames. Before any UI processing, all nine frames were queued: 16,777,220 bytes in total. No large-memory crash or out-of-memory condition was attempted. The absence of a bound is also explicit in the transport implementation.

**Repair direction:** Define limits for frame bytes, queued bytes/messages and per-frame UI work. Reject oversized frames while reading and close or backpressure peers that exceed the budget. Apply equivalent policy to TCP, Iroh and WebSocket transports, accounting for legitimate world snapshots.

**Source:** [TCP reader and channels](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/collab/net.rs#L296), [inbox drain](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/collab/net.rs#L373), [Iroh reader and output queue](https://github.com/MojzisekJan/HeronBrush/blob/f62aa28ea2f1be7b37e7b72c1e7b4c979f8116ae/crates/editor_core/src/collab/net.rs#L559).

**Observation probe:** `observe_collaboration_frame_and_inbox_growth_without_large_allocations`. This probe passes when it measures the current unbounded behavior; it must be replaced with limit-enforcement assertions when the policy is implemented. Only TCP was exercised at runtime.

## Reproduction package

The audit assertions describe the intended safe behavior and therefore fail on the audited revision. Subprocess helper tests are marked ignored and are invoked by their parent tests. Tests run with one compiler job and one test thread, using existing release artifacts and offline dependencies.

| Probe batch | Result | Coverage |
| --- | --- | --- |
| Round two | 1 passed, 4 failed | A001–A004; ordinary layer insertion is the positive control |
| Round three core | 1 passed, 3 failed, 1 helper ignored | A005, A007, A008; one-sided patch change is the positive control |
| Round three GPU | 1 failed | A006; 64x1 texture passes before the oversized case |
| Round four | 1 passed, 6 failed, 1 helper ignored | A009–A012; same-map model copy is the positive control |
| Round five | 3 passed, 3 failed | A013–A015; flat linked group and cancellation without save are controls; the TCP observation passes by measuring current behavior |

Each finding above describes its input, observed failure and expected repair behavior. The named regression probes identify the intended cases; their source files are not part of this documentation branch. Recreate them in the repository test harness on the repair machine, then run with one compiler job and one test thread:

```sh
cargo test --locked --release -p editor_core -j 1 -- --test-threads=1
cargo test --locked --release -p map_model -j 1 -- --test-threads=1
cargo test --locked --release -p render_wgpu -j 1 -- --test-threads=1
```

These commands run the repository tests; the new audit regressions must be added when implementing the repairs. A006 requires an available GPU adapter. A005 requires a Unix subprocess for the file-size-limit reproduction.

## Findings still under investigation

No additional hypothesis is counted as a confirmed finding at this checkpoint. The current review is examining parser robustness, structural merges and differences between ordinary and live export.

## Checks that ruled out suspected problems

- **Remote edits applied during local drag rollback:** Frontend session polling and MCP paths defer while a document transaction is active.
- **Shape preview regeneration fails to mark dirty after save:** Regeneration uses Document::apply, which calls note_modification.
- **Imported model reload retains stale cache:** reload_game_assets resets the asset key and sync_game_assets replaces model caches.

## Validation limits

The GPU probe used a device with an 8192 maximum texture dimension matching the desktop renderer setup. No disk-full stress test or large-memory crash was performed: A005 uses a subprocess file-size limit, and A002 triggers capacity overflow before a large allocation. Tests used one compiler job and one test thread.

This remains a focused audit, not a claim that every code path or export engine is correct. Repairs and broader validation belong on the repair machine.
