# Extended geometry tools

## Fillet

Radius and segment count have −/+ buttons. One segment gives a chamfer; the
maximum is 64. Edges, vertices and face loops can be selected; shared corners
can be cut or spherical. Clicking pins the edge selection for preview, Ctrl adds
edges, and **Apply** or Enter confirms. A click alone does not change the map.

**Inner L corner** adds a curved fill between two overlapping axis-aligned box
brushes. The arms may differ in width but must have the same depth and the same
direct parent. It uses the Fillet radius and segments; spherical corner settings
do not apply. The preview shows the added brushes, and the operation is one Undo
step.

Outer fillet UVs wrap across the segments around shared edges. With the same
material, the adjoining flat face is adjusted as well; different materials stay
separate. Spherical patches where several rounded edges meet are not guaranteed
to unwrap seamlessly. To save explicit UV axes exactly, use a format that
supports them, such as Valve 220.

## Sweep

The whole sweep is built and validated first. A collapsed segment or volumetric
self-intersection produces no partial result; the panel shows why it was
rejected. The total cell count, across all selected faces and repetitions, is
limited to 1024. For tight bends, shrink the profile, enlarge the path radius
or turn off integer snapping.

The sweep inherits the source face's material, colour and PBR settings. UVs
continue across neighbouring side faces; end caps are mapped separately.
Internal caps and fully shared contact faces use `skip`. Twisted profiles can
produce triangular side faces and extra UV seams.

New parts stay under the source brushes' parent, the source is kept, and only
the new parts are selected. The operation is one Undo step and rejects results
outside world bounds. Paths are Straight, Arc and S-bend; arbitrary polyline
paths are not supported.

## Hollow

The **Inset & bevel** panel has **Wall thickness**, a direction (**Inward**,
**Outward** or **Centered**) and **Hollow selection**. Inward keeps the outer
surface, Outward keeps the inner surface, and Centered splits the thickness
around the original boundary.

**Open selected faces** leaves openings in the selected faces, for example an
open-topped room or container. **Join convex pair** first merges two brushes
with the same direct parent and hollows their combined volume, so no double
wall remains between them. They must form a gap-free convex volume; the tool
does not fill a non-convex selection. These two options are separate modes.

New walls stay in the original entities, groups and layers. If any part fails,
the whole operation is rejected. Outward and Centered also reject results where
shells of different selected brushes intersect; use **Join convex pair** for
adjacent convex parts. The Hollow keyboard shortcut still uses one grid step
inward.

MCP `hollow` accepts `thickness`, `direction` (`inward`, `outward`,
`centered`), `open_selected_faces` and `join_convex`.

## Inset and bevel

**Connected face region** (on by default) computes all vertices of the original
brush together. Inset moves the outer boundary of the region and does not bevel
a shared edge twice. Coplanar faces on different brushes that share a full edge
keep their seam joined. Partially shared edges and T-junctions are not treated
as shared seams.

With the option off, faces are processed individually. In both modes the whole
selection is validated first, and an invalid region changes no brushes. Parents
and brush IDs are kept.

## Stairs

The Brush tool offers **Straight**, **Turn left**, **Turn right** and **Return
stairs**. Straight stairs can have a mid landing; turning stairs use a landing
joining both flights. The flight width must leave room for the run and landing;
for return stairs, both flights must fit side by side.

**Slab thickness** of zero gives solid stairs. A positive value creates a
continuous sloped underside slab, with thickness measured vertically, joined to
the landing. Geometry stays within the given bounds. These variants have even
step heights and at most 512 steps. Straight stairs without landing or slab use
the original generator.

MCP `add_shape` with `kind: "stairs"` accepts `stair_layout` (`straight`,
`left`, `right`, `return`), `flight_width`, `landing`, `slab_thickness`, `axis`
(`x` or `y`) and `ascending`.

## Corner tool limits, thickness and preview

The −/+ buttons test the next value against the actual geometry and are
disabled when that step cannot be built. Invalid manual input keeps the last
valid setting and preview, and shows the reason.

With **Full-thickness wedges**, **Preserve wall thickness** offsets each
segment's outer face perpendicular to its inner face, with neighbouring offset
lines meeting at one point. End points slide along the arms' outer walls and the
straight parts are trimmed to match. Equal-width arms thus keep the same
perpendicular thickness, even with one segment; for different widths the
thickness is interpolated by segment angle. This mode favours thickness over a
circular or elliptical outer outline; turning it off restores the outline mode.

Both previews share zoom and pan. The wheel zooms around the cursor and dragging
pans. Double-click or **Reset view** frames the whole corner; there are also
−/+ zoom buttons. MCP `join_corner` accepts `preserve_thickness: true` together
with `wedges: true`. See [Join corner](corner-join.md) for the tool itself.

## Adjustable shapes after drawing

After a drawing gesture in 2D or 3D, the generator stays attached to the new
shape. The Brush bar shows **Shape preview**, **Confirm** and **Cancel**, and
the outline uses the resize handle colour. Parameters change the last insertion
rather than creating a new object. All shape kinds are supported, including
hollow sphere and cone.

Enter accepts and Escape undoes the insertion. Changing the selection, switching
tools or making another edit detaches the generator. History stores only the
final geometry, so all parameter changes revert with one Undo; Redo restores
plain geometry, not the editable state. Invalid parameters do not change the
geometry. Shapes are not persistently parametric and add no map format metadata.
