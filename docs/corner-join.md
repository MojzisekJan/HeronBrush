# Join corner

Joins two brushes meeting in an L, such as the top and side of a window frame,
two cornices or two walls, solving both at once so there is no gap or overlap.

## Basic use

1. Select exactly two box brushes whose ends overlap in an L.
2. In the **Inset & bevel** panel, click **Join corner**.
3. Check the **Before** / **After** projection. Blue and orange mark the two
   parts.
4. **Apply** makes both cuts as one Undo step. **Cancel** or Escape closes the
   preview without changes.

Axis-aligned boxes are supported in all three main planes and all corner
orientations. Both parts must have the same depth in the third axis and overlap
across their whole end rectangle; widths may differ. Equal widths get a 45°
mitre; otherwise the cut runs between the outer and inner corner of the overlap.

Without rounding, the result is two convex brushes with the same combined outer
shape; only the doubly occupied overlap is removed. Brush IDs, selection and
entity or layer placement are kept. The new cut face takes its material from the
most similarly oriented face, as with clipping. UVs are not unified across parts.

Ends that merely touch, T-joints, crossings, differing depths and already
bevelled or rotated parts are rejected. Brushes are not extended. If the map or
selection changes while the preview is open, reopen it.

## Rounding

Radius and segment fields have **−** / **+** buttons that update the preview
and disable at the allowed limit. Values can still be typed or dragged.

**Round outer corner** removes volume from both parts, which remain separate
brushes. In independent mode the radius is limited by the narrower part.

**Round inner corner** adds a curved fill on the inside of the L, one convex
brush per segment. The preview shows these parts and their count. Radial inner
contacts use `skip`; visible faces take material and UVs from the nearest
original orientation. The fill parts do not overlap, and the radius must keep
the fill within both arms.

Both options can be used alone or together, each with its own radius and 1 to
64 segments. One segment is a straight chamfer. The inner fill requires both
brushes to have the same direct parent (entity, group or layer); it is added to
that parent and selected with the originals. The whole join is one Undo/Redo
step.

### Concentric rounding

With both roundings on, **Concentric rounding** is available. It requires equal
arm width `t`. You set the inner radius `r`; the outer radius is `r + t` and is
shown in the dialog. The inner segment count applies to both arcs, so they share
a centre and angular divisions. The independent outer settings are ignored.

The outer arc may exceed the arm width; the fill brushes are trimmed by the same
outer cuts, so a large radius produces no overlap. Both radii must fit within the
arm lengths. With different widths, two concentric quarter circles cannot be
tangent to all four straight edges, so the tool reports an error.

## Full-thickness wedges

**Full-thickness wedges** rebuilds the whole bend. The two original brushes are
shortened at the tangent points and remain straight arms with their IDs. One
convex four-sided prism per segment is inserted between them, with inner and
outer arc faces, top and bottom, and two `skip` contacts.

You set the inner radius `r` and 1 to 64 segments; the outer settings are
ignored. The radius may exceed the arm width and the original overlap, but each
arm must keep a positive straight length. Arms are not consumed or extended.

Equal thickness `t` gives concentric circles of radius `r` and `r+t`. Different
thicknesses `tu`, `tv` give a circular inner arc and an elliptical outer arc with
semi-axes `r+tu` and `r+tv`, centred `r` from both inner walls and tangent to
both outer walls. Thickness changes smoothly between the arms. One segment gives
a straight full-thickness chamfer. See
[Extended geometry tools](geometry-tools.md) for **Preserve wall thickness**.

The mode requires the same direct parent and is one Undo/Redo step. The result
is ordinary convex brushes and needs no special export support.

## Textures on rounded corners

Mapping rotates progressively around shared edges, keeping scale and UV
continuity across segments. When both arms use the same material, mapping
follows the first arm and also adjusts the adjoining flat face of the second, so
its original offset may change. Inner and outer arcs are mapped separately.
Different materials stay separate, each unwrapped from its own arm. Top and
bottom faces keep their projection. Arcs use explicit UV axes; save in a format
with Valve 220 UVs to keep them exactly.

## MCP

`join_corner` works on exactly two selected brushes. The default
`dry_run: true` only validates and returns the cut plane's ID, normal and
distance, the removed overlap volume and the map revision.

```json
{"dry_run":true}
```

Rounding uses optional `outer` and `inner` objects, each with `radius` and
`segments` (default eight). Concentric mode needs both objects and
`concentric: true`; it uses the inner radius and segments and derives the outer
radius from the arm width:

```json
{"dry_run":true,"outer":{"radius":8},"inner":{"radius":8,"segments":8},"concentric":true}
```

A full curved wall:

```json
{"dry_run":true,"wedges":true,"outer":{"radius":8},"inner":{"radius":48,"segments":8}}
```

`wedges` needs both objects and cannot be combined with `concentric: true`. The
outer radius and segments are derived, so `outer.radius` is ignored; different
thicknesses use an ellipse automatically.

The response also includes `brush_count` and `added_brushes`.
`removed_overlap_volume` is the original box overlap, `removed_volume` also
includes outer rounding, and `added_volume` is the fill actually added after
any outer trimming.

To apply, pass the returned revision as `expected_revision` with
`dry_run: false`. A change to the selection or map invalidates the old preview.
The operation is one Undo step and needs no new `.map` format.

## Geometry

The shared cut plane contains the outer and inner edges of the overlap and is
parallel to the common depth. Each brush is clipped by the opposite half-space.
Before writing, both results are checked for positive volume and their sum must
match the union of the original boxes. Every variant validates all resulting
brushes for validity and convexity and makes no partial change.

## Not yet supported

- Picking the joint location by click when the overlap is ambiguous.
- Rotated parts, oblique angles and other profiles.
- Live in-map preview, width by dragging and segments by mouse wheel.
- Re-editable operation parameters.
