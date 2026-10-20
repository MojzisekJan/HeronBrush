//! Document operations for geometry.

use super::{Float, Document, Bbox3d, Brush, EXTRUDE_EPSILON, Edit, FaceRef, HashMap, HashSet, Node, NodeId, NodeKind, Plane3d, Polyhedron, Ray3d, Vec2f, Vec3d, brush_node, csg, node, patch_node, vec};

impl Document {
    /// Subtracts the selection from every other brush.
    ///
    /// A carve: select the shape of the hole you want, and it is taken
    /// out of everything it overlaps. The tools themselves are deleted, since their job
    /// was to describe the hole.
    ///
    /// Returns how many brushes the subtraction changed.
    /// Replaces the selected brushes with one convex brush enclosing all their corners.
    ///
    /// `csgConvexMerge`. Two brushes that together make a convex shape become one; two
    /// that do not become the smallest convex brush that holds both, which is what the
    /// operation means and why it is used deliberately rather than as a tidy-up.
    ///
    /// Returns whether anything was merged. Nothing happens with fewer than two brushes
    /// selected, or when their corners do not enclose a volume.
    pub fn merge_selection(&mut self) -> bool {
        let selected: Vec<(NodeId, Brush)> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| self.selection.contains(id))
            .map(|(id, brush)| (id, brush.clone()))
            .collect();
        if selected.len() < 2 {
            return false;
        }

        let points: Vec<Vec3d> = selected
            .iter()
            .flat_map(|(_, brush)| brush.vertex_positions())
            .collect();
        let Ok(geometry) = map_model::polyhedron::Polyhedron::from_points(&points, f64::ALMOST_ZERO)
        else {
            return false;
        };

        // The merged brush wears the materials of the faces it stands in for, or it would
        // come out in whatever the current material happens to be.
        let candidates: Vec<map_model::brush_face::BrushFace> = selected
            .iter()
            .flat_map(|(_, brush)| brush.faces().iter().cloned())
            .collect();
        let merged = Brush::from_polyhedron(geometry, &self.current_material)
            .with_face_attributes_from(&candidates);

        let removed: Vec<Node> = selected
            .iter()
            .map(|(id, brush)| brush_node(*id, brush.clone()))
            .collect();
        let node = Node::new(NodeKind::Brush(merged));
        let id = node.id;

        self.selection = std::iter::once(id).collect();
        self.apply_and_record(Edit::Batch(vec![
            Edit::Removed {
                layer: self.current_layer,
                nodes: removed,
            },
            Edit::Added {
                layer: self.current_layer,
                nodes: vec![node],
            },
        ]));
        true
    }

    /// Replaces each selected brush with the walls of a room the same size.
    ///
    /// `csgHollow`: the brush shrunk by one grid step is taken out of itself, which leaves
    /// the shell. A brush too small to shrink is left as it is.
    ///
    /// Returns how many brushes were hollowed.
    pub fn hollow_selection(&mut self, thickness: f64) -> usize {
        self.hollow_selection_options(thickness,map_model::hollow::Direction::Inward,&[],false).unwrap_or(0)
    }

    pub fn subtract_selection(&mut self) -> usize {
        let all = node::brushes_with_ids(&self.tree.world);
        let tools: Vec<Brush> = all
            .iter()
            .filter(|(id, _)| self.selection.contains(id))
            .map(|(_, brush)| (*brush).clone())
            .collect();
        if tools.is_empty() {
            return 0;
        }

        // Only a brush whose box meets a tool's can be cut by it; the rest of the map is
        // left alone without being copied (copying every unselected brush of a big map
        // was most of the cost).
        let tool_bounds: Vec<geometry_math::bbox::Bbox3d> = tools.iter().map(|tool| tool.bounds()).collect();
        let targets: Vec<(NodeId, Brush)> = all
            .iter()
            .filter(|(id, _)| !self.selection.contains(id))
            .filter(|(_, brush)| {
                let bounds = brush.bounds();
                tool_bounds.iter().any(|tool| tool.intersects(&bounds))
            })
            .map(|(id, brush)| (*id, (*brush).clone()))
            .collect();

        let mut removed: Vec<Node> = Vec::new();
        let mut added: Vec<Node> = Vec::new();
        let mut changed = 0usize;

        for (id, target) in targets {
            let pieces = csg::subtract_all(&target, &tools);
            // One piece with the same shape means nothing was taken out of it.
            let untouched = pieces.len() == 1
                && pieces[0].bounds() == target.bounds()
                && pieces[0].faces().len() == target.faces().len();
            if untouched {
                continue;
            }

            changed += 1;
            removed.push(brush_node(id, target));
            added.extend(
                pieces
                    .into_iter()
                    .map(|piece| Node::new(NodeKind::Brush(piece))),
            );
        }

        // The tools have done their job.
        removed.extend(
            node::brushes_with_ids(&self.tree.world)
                .into_iter()
                .filter(|(id, _)| self.selection.contains(id))
                .map(|(id, brush)| brush_node(id, brush.clone())),
        );

        if removed.is_empty() {
            return 0;
        }

        // Taking the tools out and putting the pieces in is one thing to whoever asked
        // for a subtraction, so it is one step to undo.
        let mut parts = vec![Edit::Removed {
            layer: 0,
            nodes: removed,
        }];
        if added.is_empty() {
            self.selection.clear();
        } else {
            self.selection = added.iter().map(|node| node.id).collect();
            parts.push(Edit::Added {
                layer: 0,
                nodes: added,
            });
        }
        self.apply_and_record(Edit::Batch(parts));
        changed
    }

    /// Returns the selected brushes, for the handle tools to gather handles from.
    pub fn selected_brushes(&self) -> Vec<&Brush> {
        node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| self.selection.contains(id))
            .map(|(_, brush)| brush)
            .collect()
    }

    /// Whether a material stays where it is on a face as the face moves.
    ///
    /// `Preferences::AlignmentLock`.
    pub(super) fn alignment_lock(&self) -> bool {
        crate::preferences::pref(|p| p.alignment_lock)
    }

    /// How large the named material is, in pixels.
    ///
    /// `BrushFace::textureSize`, which is one by one when there is no material: the size
    /// only decides how far an offset drifts before it wraps, and with nothing loaded
    /// there is nothing to wrap against.
    #[cfg(test)]
    pub(super) fn material_size(&self, name: &str) -> geometry_math::vec::Vec2f {
        match self.materials.get(name) {
            Some(material) => geometry_math::vec::Vec2f::new(
                (material.width.max(1)) as f32,
                (material.height.max(1)) as f32,
            ),
            None => geometry_math::vec::Vec2f::new(1.0, 1.0),
        }
    }

    /// Every selected patch.
    pub fn selected_patches(&self) -> Vec<&map_model::io::map_reader::Patch> {
        node::patches_by_layer(&self.tree.world)
            .into_iter()
            .filter(|(id, _, _)| self.selection.contains(id))
            .map(|(_, patch, _)| patch)
            .collect()
    }

    /// Moves the given control points by the given delta, across every selected patch.
    ///
    /// `transformControlPoints` in `Map_Patches`: every selected patch moves whichever of
    /// its control points are at one of the given positions, and a patch with none is
    /// left as it is. A patch takes any move -- unlike a brush, it cannot collapse -- so
    /// there is nothing here to refuse.
    pub fn move_control_points(&mut self, positions: &[Vec3d], delta: Vec3d) -> bool {
        if positions.is_empty() || delta == Vec3d::zero() {
            return false;
        }

        let epsilon = crate::handle_tool::DRAG_EPSILON;
        let mut before = Vec::new();
        let mut after = Vec::new();

        for (id, patch, _) in node::patches_by_layer(&self.tree.world) {
            if !self.selection.contains(&id) {
                continue;
            }

            let mut moved = patch.clone();
            let mut any = false;
            for point in &mut moved.control_points {
                if positions
                    .iter()
                    .any(|position| vec::squared_distance(*position, point.position) <= epsilon)
                {
                    point.position += delta;
                    any = true;
                }
            }
            if !any {
                continue;
            }

            before.push(patch_node(id, patch.clone()));
            after.push(patch_node(id, moved));
        }

        if after.is_empty() {
            return false;
        }

        self.apply_and_record(Edit::Changed { before, after });
        true
    }

    /// Changes the control-point grid of every selected patch, in one undo step.
    ///
    /// `ControlPointToolPage`'s spin boxes. `resize` is `map_model::patch_edit`'s add or remove
    /// for rows or columns; a patch it cannot apply to is left as it was. Returns how many
    /// patches changed.
    pub fn resize_selected_patches(
        &mut self,
        resize: impl Fn(&map_model::io::map_reader::Patch) -> Option<map_model::io::map_reader::Patch>,
    ) -> usize {
        let mut before = Vec::new();
        let mut after = Vec::new();
        for (id, patch, _) in node::patches_by_layer(&self.tree.world) {
            if !self.selection.contains(&id) {
                continue;
            }
            let Some(resized) = resize(patch) else {
                continue;
            };
            before.push(patch_node(id, patch.clone()));
            after.push(patch_node(id, resized));
        }
        if before.is_empty() {
            return 0;
        }
        let count = after.len();
        self.apply_and_record(Edit::Changed { before, after });
        count
    }

    /// Puts a corner where a split was taken hold of, moved by the drag so far.
    ///
    /// `addVertex` and the drag that follows it. A point sitting on an edge or a face is
    /// not a corner of the hull, so the corner is added where it is going rather than
    /// where it started: dragging the middle of a top edge upwards leaves a ridge. Brushes
    /// the point does not touch are left alone. Returns how many brushes changed.
    pub fn split_handle(&mut self, at: Vec3d, delta: Vec3d) -> usize {
        if delta == Vec3d::zero() {
            return 0;
        }

        let mut before = Vec::new();
        let mut after = Vec::new();
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            if !self.selection.contains(&id) || !brush.contains_point(at) {
                continue;
            }
            match brush.split_vertex(at, delta) {
                Ok(split) => {
                    before.push(brush_node(id, brush.clone()));
                    after.push(brush_node(id, split));
                }
                Err(_) => continue,
            }
        }

        if before.is_empty() {
            return 0;
        }
        let changed = before.len();
        self.apply_and_record(Edit::Changed { before, after });
        self.mark_changed();
        changed
    }

    /// Draws the selected faces in and pushes them along their normals, in one step.
    ///
    /// Insets a face and optionally moves it along its normal, tilting adjacent faces to meet it.
    /// Leaves faces unchanged if the inset would consume them. Returns the number of bevelled
    /// faces.
    pub fn bevel_faces(&mut self, faces: &[FaceRef], inset: f64, depth: f64) -> usize {
        self.bevel_faces_options(faces,inset,depth,true).unwrap_or(0)
    }

    /// Stages every brush before committing, so a refused region changes nothing.
    pub fn bevel_faces_options(&mut self,faces:&[FaceRef],inset:f64,depth:f64,region:bool)->Result<usize,String> {
        if inset==0.0 && depth==0.0 {return Ok(0);}
        if self.in_transaction() {return Err("Finish the current drag first.".into());}
        let mut seen=HashSet::new();let unique:Vec<_>=faces.iter().copied().filter(|face|seen.insert(*face)).collect();let faces=unique.as_slice();
        let mut groups:HashMap<NodeId,Vec<usize>>=HashMap::new();
        for face in faces {groups.entry(face.node).or_default().push(face.face);}
        let mut seams=Vec::new();
        if region {
            let mut edges=std::collections::BTreeMap::<[[i64;3];2],Vec<(NodeId,Plane3d,Vec3d,Vec3d)>>::new();
            for face in faces {
                let brush=self.brush(face.node).ok_or("A selected brush no longer exists.")?;
                let polygon=brush.face_polygon(face.face).ok_or("A selected face no longer exists.")?;
                let plane=brush.faces()[face.face].plane;
                for (&a,&b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
                    let mut key=[a,b].map(|p|[(p[0]*1e6).round() as i64,(p[1]*1e6).round() as i64,(p[2]*1e6).round() as i64]);key.sort();
                    edges.entry(key).or_default().push((face.node,plane,a,b));
                }
            }
            for edge in edges.values() {
                if let [(a,pa,start,end),(b,pb,_,_)]=edge.as_slice() {
                    if a!=b && vec::length(pa.normal-pb.normal)<1e-7 && (pa.distance-pb.distance).abs()<1e-7 {seams.push((*start,*end));}
                }
            }
        }
        let mut replacements=HashMap::new();let mut count=0;
        for (id,mut indices) in groups {
            if !self.is_editable(id) {return Err("Every selected face must be editable.".into());}
            let brush=self.brush(id).ok_or("A selected brush no longer exists.")?;
            indices.sort_unstable();indices.dedup();count+=indices.len();
            let result=if region {brush.bevel_face_region_with_seams(&indices,inset,depth,&seams)} else {
                let mut working=brush.clone();
                for index in indices {
                    let plane=brush.faces().get(index).ok_or("A selected face no longer exists.")?.plane;
                    let index=working.faces().iter().position(|f|vec::length(f.normal()-plane.normal)<1e-7)
                        .ok_or("An earlier inset removed a selected face.")?;
                    working=working.bevel_face(index,inset,depth).map_err(|_|"A selected face cannot take this inset.")?;
                }
                Ok(working)
            }.map_err(|_|"The face region cannot take this inset/depth without collapsing or separating its boundary.")?;
            replacements.insert(id,vec![result]);
        }
        if !replacements.is_empty() {self.replace_geometry_in_place(replacements);}
        Ok(count)
    }

    /// Puts a brush on each of the given faces, or takes one out of it.
    ///
    /// The face drawn in by `inset` and raised by `depth`: a panel when the depth is
    /// positive, and, when it is negative, the shape of a recess, which is then taken out
    /// of the brush it stands on. Returns how many faces got one.
    pub fn panel_on_faces(&mut self, faces: &[FaceRef], inset: f64, depth: f64) -> usize {
        if depth == 0.0 {
            return 0;
        }

        let faces: Vec<FaceRef> = faces.to_vec();
        let mut added = Vec::new();
        let mut carved_before = Vec::new();
        let mut carved_after = Vec::new();

        for face in faces {
            let Some(brush) = self.brush(face.node).cloned() else {
                continue;
            };
            let Ok(panel) = brush.panel_on_face(face.face, inset, depth) else {
                continue;
            };

            if depth > 0.0 {
                added.push(Node::new(NodeKind::Brush(panel)));
                continue;
            }
            // A recess: what is left of the brush once the shape is taken out of it.
            let left = map_model::csg::subtract(&brush, &panel);
            if left.is_empty() {
                continue;
            }
            carved_before.push(brush_node(face.node, brush));
            carved_after.push(left);
        }

        let count = added.len() + carved_before.len();
        if count == 0 {
            return 0;
        }

        let mut parts = Vec::new();
        if !carved_before.is_empty() {
            // The brush goes and its pieces take its place, which is what a carve is.
            parts.push(Edit::Removed {
                layer: self.current_layer,
                nodes: carved_before,
            });
            parts.push(Edit::Added {
                layer: self.current_layer,
                nodes: carved_after
                    .into_iter()
                    .flatten()
                    .map(|piece| Node::new(NodeKind::Brush(piece)))
                    .collect(),
            });
        }
        if !added.is_empty() {
            parts.push(Edit::Added {
                layer: self.current_layer,
                nodes: added,
            });
        }

        self.face_selection.clear();
        self.apply_and_record(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Edit::Batch(parts)
        });
        self.mark_changed();
        count
    }

    /// Takes the given corners off every selected brush that has them.
    ///
    /// Deletes selected vertices, or the corners represented by selected edge/face handles. Brushes
    /// that would lose their volume are unchanged. Returns the number of modified brushes.
    pub fn remove_handles(&mut self, vertices: &[Vec3d]) -> usize {
        if vertices.is_empty() {
            return 0;
        }

        let epsilon = crate::handle_tool::DRAG_EPSILON;
        let mut before = Vec::new();
        let mut after = Vec::new();
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            if !self.selection.contains(&id) {
                continue;
            }
            match brush.remove_vertices(vertices, epsilon) {
                Ok(left) => {
                    before.push(brush_node(id, brush.clone()));
                    after.push(brush_node(id, left));
                }
                // Nothing of this brush was picked, or what is left would enclose
                // nothing: either way it stays as it is.
                Err(_) => continue,
            }
        }

        if before.is_empty() {
            return 0;
        }
        let changed = before.len();
        self.apply_and_record(Edit::Changed { before, after });
        self.mark_changed();
        changed
    }

    /// Moves the given brush vertices by the given delta, across every selected brush.
    ///
    /// Moves matching vertices in selected brushes. The operation is atomic: if any affected brush
    /// would collapse, none are changed.
    pub fn move_handles(&mut self, vertices: &[Vec3d], delta: Vec3d) -> bool {
        if vertices.is_empty() || delta == Vec3d::zero() {
            return false;
        }

        use rayon::prelude::*;
        let epsilon = crate::handle_tool::DRAG_EPSILON;
        let uv_lock = crate::preferences::pref(|p| p.uv_lock);
        let materials = &self.materials;
        // The handles sorted along x, so each brush is offered only the ones inside its
        // box: offering every handle to every selected brush was quadratic in a drag of
        // the top faces of a thousand brushes.
        let mut sorted: Vec<Vec3d> = vertices.to_vec();
        sorted.sort_by(|a, b| a.x().total_cmp(&b.x()));
        let near = |brush: &Brush| -> Vec<Vec3d> {
            let bounds = brush.bounds();
            let from = sorted.partition_point(|v| v.x() < bounds.min.x() - epsilon);
            sorted[from..]
                .iter()
                .take_while(|v| v.x() <= bounds.max.x() + epsilon)
                .filter(|v| {
                    (1..3).all(|axis| v[axis] >= bounds.min[axis] - epsilon && v[axis] <= bounds.max[axis] + epsilon)
                })
                .copied()
                .collect()
        };
        let selected: Vec<(NodeId, &Brush)> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| self.selection.contains(id))
            .collect();
        let moved = |brush: &Brush| {
            let handles = near(brush);
            if handles.is_empty() {
                return Err(map_model::brush::VertexError::NoVerticesMatched);
            }
            brush.move_vertices_with_uv_lock(&handles, delta, epsilon, uv_lock, |name| super::transforms::material_size_in(materials, name))
        };
        let results: Vec<_> = if selected.len() >= 64 {
            selected.par_iter().map(|(_, brush)| moved(brush)).collect()
        } else {
            selected.iter().map(|(_, brush)| moved(brush)).collect()
        };

        // Inside a transaction only the new state is applied, so the old is not copied.
        let record = self.transaction.is_none();
        let mut before = Vec::new();
        let mut after = Vec::new();
        for ((id, brush), result) in selected.into_iter().zip(results) {
            match result {
                Ok(moved) => {
                    if record {
                        before.push(brush_node(id, brush.clone()));
                    }
                    after.push(brush_node(id, moved));
                }
                // A brush with none of these vertices is not part of this drag.
                Err(map_model::brush::VertexError::NoVerticesMatched) => {}
                // Anything else means the drag would destroy a brush, so none of it
                // happens.
                Err(_) => return false,
            }
        }

        if after.is_empty() {
            return false;
        }

        self.apply_and_record(Edit::Changed { before, after });
        true
    }

    /// Rounds the given edges on every selected brush that has them.
    ///
    /// `edges` are the ends of the edges picked with the edge tool, and a brush that has
    /// none of them is not part of this. Every edge of one brush is rounded in one go, so
    /// two rounded edges that meet leave a mitre rather than one cutting into the other.
    ///
    /// Nothing happens at all if any brush cannot take the radius, which is the same rule
    /// a vertex drag goes by: a fillet that ate one of several selected brushes would be
    /// a surprise to undo.
    pub fn fillet_edges(
        &mut self,
        edges: &[(Vec3d, Vec3d)],
        fillet: &map_model::fillet::Fillet,
    ) -> Result<usize, map_model::fillet::FilletError> {
        let mut wanted: Vec<(NodeId, Vec<(Vec3d, Vec3d)>)> = Vec::new();
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            if !self.selection.contains(&id) {
                continue;
            }
            let mine: Vec<(Vec3d, Vec3d)> = edges
                .iter()
                .copied()
                .filter(|(from, to)| map_model::fillet::has_edge(brush, *from, *to))
                .collect();
            if !mine.is_empty() {
                wanted.push((id, mine));
            }
        }
        self.round(wanted, fillet)
    }

    /// Rounds every edge around each selected face: the loop the face is bounded by.
    ///
    /// This is the other way to ask for a fillet -- a face rather than its edges one at a
    /// time -- and it goes through the same rounding, so a corner where two of the loop's
    /// edges meet is mitred like any other.
    pub fn fillet_face_loops(
        &mut self,
        fillet: &map_model::fillet::Fillet,
    ) -> Result<usize, map_model::fillet::FilletError> {
        let mut wanted: Vec<(NodeId, Vec<(Vec3d, Vec3d)>)> = Vec::new();
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            let mut edges: Vec<(Vec3d, Vec3d)> = Vec::new();
            for index in 0..brush.faces().len() {
                let reference = FaceRef {
                    node: id,
                    face: index,
                };
                if !self.is_face_selected(reference) {
                    continue;
                }
                if let Some(face)=brush.geometry_face_of(index) {edges.extend(map_model::fillet::face_edges(brush,face));}
            }
            if !edges.is_empty() {
                wanted.push((id, edges));
            }
        }
        self.round(wanted, fillet)
    }

    /// Rounds the edges named for each brush, and records the lot as one edit.
    pub(super) fn round(
        &mut self,
        wanted: Vec<(NodeId, Vec<(Vec3d, Vec3d)>)>,
        fillet: &map_model::fillet::Fillet,
    ) -> Result<usize, map_model::fillet::FilletError> {
        let mut before = Vec::new();
        let mut after = Vec::new();
        for (id, edges) in wanted {
            let Some(node) = node::find_node(&self.tree.world, id) else {
                continue;
            };
            let NodeKind::Brush(brush) = &node.kind else {
                continue;
            };
            let rounded = map_model::fillet::fillet_edges(brush, &edges, fillet)?;
            before.push(brush_node(id, brush.clone()));
            after.push(brush_node(id, rounded));
        }

        let count = after.len();
        if count > 0 {
            self.apply_and_record(Edit::Changed { before, after });
        }
        Ok(count)
    }

    /// Moves the plane of the given face on every selected brush that has it.
    ///
    /// `extrudeBrushes` in `Map_Geometry`. The face is named by its vertices rather than
    /// by an index, so a drag started on one brush reaches the matching face on all the
    /// others: extruding a wall built from several brushes moves the whole wall.
    ///
    /// If any brush cannot take the move nothing moves at all, which is what stops a drag
    /// from clipping one brush away while stretching the rest.
    pub fn extrude_faces(&mut self, polygon: &[Vec3d], delta: Vec3d) -> bool {
        // A drag that ended where it started changes nothing, and recording it would
        // leave a step in the history that undoes nothing. `translate_selection` and
        // `move_handles` refuse the same way.
        if delta == Vec3d::zero() {
            return false;
        }
        self.extrude_face_planes(&[(polygon.to_vec(), delta)])
    }

    /// Moves the planes of several faces at once on every selected brush that has them.
    ///
    /// Dragging a brush's corner in a flat view moves two walls together, and they have to
    /// move in one step: rebuilding the brush after the first would leave the second face's
    /// vertices somewhere else, so it would no longer be found by the polygon the drag
    /// started with.
    pub fn extrude_face_planes(&mut self, moves: &[(Vec<Vec3d>, Vec3d)]) -> bool {
        // A drag that ended where it started changes nothing, and recording it would
        // leave a step in the history that undoes nothing. `translate_selection` and
        // `move_handles` refuse the same way.
        if moves.iter().all(|(_, delta)| *delta == Vec3d::zero()) {
            return false;
        }
        self.change_selected_brushes(|world_bounds, brush| {
            let planned: Vec<(usize, Vec3d)> = moves
                .iter()
                .filter(|(_, delta)| *delta != Vec3d::zero())
                .filter_map(|(polygon, delta)| {
                    brush
                        .find_face_by_polygon(polygon, EXTRUDE_EPSILON)
                        .map(|index| (index, *delta))
                })
                .collect();
            // A brush without any of these faces is not part of this drag.
            if planned.is_empty() {
                return Ok(None);
            }
            brush
                .move_boundaries(world_bounds, &planned)
                .map(Some)
                .map_err(|_| ())
        })
    }

    /// Moves the given face's vertices, leaving the rest of the brush where it is.
    ///
    /// `ExtrudeTool::slide`, which is Alt during an extrude drag: the face slides across
    /// the brush rather than the brush stretching to follow it.
    pub fn slide_faces(&mut self, polygon: &[Vec3d], delta: Vec3d) -> bool {
        // A drag that ended where it started changes nothing, and recording it would
        // leave a step in the history that undoes nothing. `translate_selection` and
        // `move_handles` refuse the same way.
        if delta == Vec3d::zero() {
            return false;
        }
        self.change_selected_brushes(|_, brush| {
            match brush.find_face_by_polygon(polygon, EXTRUDE_EPSILON) {
                None => Ok(None),
                Some(index) => {
                    let Some(vertices) = brush.face_polygon(index) else {
                        return Ok(None);
                    };
                    brush
                        .move_vertices(&vertices, delta, EXTRUDE_EPSILON)
                        .map(Some)
                        .map_err(|_| ())
                }
            }
        })
    }

    /// Splits the given face off each selected brush that has it.
    ///
    /// `splitBrushesOutward` and `splitBrushesInward`, which is Ctrl during an extrude
    /// drag. Dragging a face outwards leaves the brush alone and adds the slab the face
    /// swept; dragging it inwards cuts the brush in two and keeps both halves. Which of
    /// those happens is decided by the direction, not by another key.
    pub fn split_faces(&mut self, polygon: &[Vec3d], delta: Vec3d) -> bool {
        // A drag that ended where it started changes nothing, and recording it would
        // leave a step in the history that undoes nothing. `translate_selection` and
        // `move_handles` refuse the same way.
        if delta == Vec3d::zero() {
            return false;
        }
        let world_bounds = self.world_bounds;
        let selected: Vec<(NodeId, Brush)> = self.selected_brushes_with_ids();

        let mut before: Vec<Node> = Vec::new();
        let mut after: Vec<Node> = Vec::new();
        let mut added: Vec<Node> = Vec::new();

        for (id, brush) in selected {
            let Some(index) = brush.find_face_by_polygon(polygon, EXTRUDE_EPSILON) else {
                continue;
            };
            let face_plane = brush.faces()[index].plane;
            let outward = vec::dot(face_plane.normal, delta) > 0.0;

            if outward {
                // The brush with its boundary moved, cut back to just the new slab: what
                // the face swept, and nothing that was already there.
                let Ok(stretched) = brush.move_boundary(&world_bounds, index, delta) else {
                    return false;
                };
                let Some(slab) = stretched.clip(&face_plane.flip()).clipped() else {
                    return false;
                };
                added.push(Node::new(NodeKind::Brush(slab)));
            } else {
                // The plane where the face has been dragged to, cutting the brush in two.
                let cut = Plane3d::from_anchor_and_normal(
                    face_plane.anchor() + delta,
                    face_plane.normal,
                );
                let Some(front) = brush.clip(&cut.flip()).clipped() else {
                    return false;
                };
                let Some(back) = brush.clip(&cut).clipped() else {
                    return false;
                };
                before.push(brush_node(id, brush));
                after.push(brush_node(id, front));
                added.push(Node::new(NodeKind::Brush(back)));
            }
        }

        if added.is_empty() {
            return false;
        }

        if !after.is_empty() {
            self.apply_and_record(Edit::Changed { before, after });
        }
        // The new brushes join the selection, so a second drag carries on from here.
        self.selection.extend(added.iter().map(|node| node.id));
        self.apply_and_record(Edit::Added {
            layer: 0,
            nodes: added,
        });
        true
    }

    /// Adds a brush filling what the given face would sweep, leaving the originals alone.
    ///
    /// `stampBrushes`, which is Shift+Ctrl+Alt during an extrude drag. The new brush is
    /// the hull of the face's vertices where they are and where they are dragged to, so
    /// dragging inwards would put it inside the brush it came from and is refused.
    pub fn stamp_faces(&mut self, polygon: &[Vec3d], delta: Vec3d) -> bool {
        // A drag that ended where it started changes nothing, and recording it would
        // leave a step in the history that undoes nothing. `translate_selection` and
        // `move_handles` refuse the same way.
        if delta == Vec3d::zero() {
            return false;
        }
        let selected = self.selected_brushes_with_ids();
        let mut added: Vec<Node> = Vec::new();

        for (_, brush) in selected {
            let Some(index) = brush.find_face_by_polygon(polygon, EXTRUDE_EPSILON) else {
                continue;
            };
            if vec::dot(brush.faces()[index].plane.normal, delta) <= 0.0 {
                return false;
            }
            let Some(vertices) = brush.face_polygon(index) else {
                continue;
            };

            let mut points = vertices.clone();
            points.extend(vertices.iter().map(|point| *point + delta));
            let Ok(geometry) =
                Polyhedron::from_points(&points, EXTRUDE_EPSILON)
            else {
                return false;
            };
            added.push(Node::new(NodeKind::Brush(Brush::from_polyhedron(
                geometry,
                &brush.faces()[index].material_name,
            ))));
        }

        if added.is_empty() {
            return false;
        }
        self.selection = added.iter().map(|node| node.id).collect();
        self.apply_and_record(Edit::Added {
            layer: 0,
            nodes: added,
        });
        true
    }

    /// The selected brushes with the ids they are stored under.
    pub(super) fn selected_brushes_with_ids(&self) -> Vec<(NodeId, Brush)> {
        node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| self.selection.contains(id))
            .map(|(id, brush)| (id, brush.clone()))
            .collect()
    }

    /// Replaces every selected brush with what the given change makes of it.
    ///
    /// A brush the change has nothing to say about is left alone; a change that fails on
    /// any brush leaves the document untouched, so a drag never half-applies.
    fn change_selected_brushes(
        &mut self,
        mut change: impl FnMut(&Bbox3d, &Brush) -> Result<Option<Brush>, ()>,
    ) -> bool {
        let world_bounds = self.world_bounds;
        let mut before = Vec::new();
        let mut after = Vec::new();

        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            if !self.selection.contains(&id) {
                continue;
            }
            match change(&world_bounds, brush) {
                Ok(Some(changed)) => {
                    before.push(brush_node(id, brush.clone()));
                    after.push(brush_node(id, changed));
                }
                Ok(None) => {}
                Err(()) => return false,
            }
        }

        if after.is_empty() {
            return false;
        }
        self.apply_and_record(Edit::Changed { before, after });
        true
    }

    /// Cuts every selected brush along the clip tool's plane.
    ///
    /// `ClipTool::performClip`: the halves that survive replace the brushes they came
    /// from and become the new selection, in one undoable step. Returns how many brushes
    /// were cut.
    pub fn clip_selection(&mut self, clip: &crate::clip_tool::ClipTool) -> usize {
        if !clip.can_clip() {
            return 0;
        }

        fn rewrite(source: &Node, selected: &HashSet<NodeId>, clip: &crate::clip_tool::ClipTool, bounds: &Bbox3d, picked: &mut HashSet<NodeId>) -> Option<Vec<Node>> {
            if let NodeKind::Brush(brush) = &source.kind {
                if !selected.contains(&source.id) { return None; }
                return Some(clip.clip_brush(bounds, brush).into_iter().enumerate().map(|(index, brush)| {
                    let node = Node::with_id(if index == 0 { source.id } else { NodeId::next() }, NodeKind::Brush(brush));
                    picked.insert(node.id); node
                }).collect());
            }
            let children = match &source.kind {
                NodeKind::Group(group) => &group.children,
                NodeKind::Entity(entity) => &entity.children,
                NodeKind::Layer(layer) => &layer.children,
                _ => return None,
            };
            let mut changed = false;
            let after: Vec<Node> = children.iter().flat_map(|child| match rewrite(child, selected, clip, bounds, picked) {
                Some(nodes) => { changed = true; nodes },
                None => vec![child.clone()],
            }).collect();
            if !changed { return None; }
            let mut parent = source.clone();
            match &mut parent.kind {
                NodeKind::Group(group) => group.children = after,
                NodeKind::Entity(entity) => entity.children = after,
                NodeKind::Layer(layer) => layer.children = after,
                _ => unreachable!(),
            }
            Some(vec![parent])
        }
        let count = node::brushes_with_ids(&self.tree.world).iter().filter(|(id,_)|self.selection.contains(id)).count();
        if count == 0 { return 0; }
        let mut edits = Vec::new(); let mut picked = HashSet::new();
        for (layer, contents) in self.tree.world.layers.iter().enumerate() {
            for source in &contents.children {
                if let Some(mut replacements) = rewrite(source, &self.selection, clip, &self.world_bounds, &mut picked) {
                    if replacements.is_empty() { edits.push(Edit::Removed { layer, nodes: vec![source.clone()] }); }
                    else {
                        let first = replacements.remove(0);
                        edits.push(Edit::Changed { before:vec![source.clone()], after:vec![first] });
                        if !replacements.is_empty() { edits.push(Edit::Added { layer, nodes:replacements }); }
                    }
                }
            }
        }
        self.apply_and_record(Edit::Batch(edits));
        self.selection = picked;
        count
    }

    /// Replaces the selection with the part all of its brushes have in common.
    ///
    /// Returns whether they had anything in common.
    pub fn intersect_selection(&mut self) -> bool {
        let selected: Vec<(NodeId, Brush)> = node::brushes_with_ids(&self.tree.world)
            .into_iter()
            .filter(|(id, _)| self.selection.contains(id))
            .map(|(id, brush)| (id, brush.clone()))
            .collect();
        if selected.len() < 2 {
            return false;
        }

        let mut common = selected[0].1.clone();
        for (_, brush) in &selected[1..] {
            match csg::intersect(&common, brush) {
                Some(overlap) => common = overlap,
                None => return false,
            }
        }

        self.apply_and_record(Edit::Removed {
            layer: 0,
            nodes: selected
                .into_iter()
                .map(|(id, brush)| brush_node(id, brush))
                .collect(),
        });
        let node = Node::new(NodeKind::Brush(common));
        self.selection = std::iter::once(node.id).collect();
        self.apply_and_record(Edit::Added {
            layer: 0,
            nodes: vec![node],
        });
        true
    }

    /// Adds the given brushes to the default layer and selects them.
    ///
    /// New geometry goes into the default layer, which is where it belongs when no other
    /// layer is current.
    pub fn add_brushes(&mut self, brushes: Vec<Brush>) {
        if brushes.is_empty() {
            return;
        }
        let nodes: Vec<Node> = brushes
            .into_iter()
            .map(|brush| Node::new(NodeKind::Brush(brush)))
            .collect();

        self.selection = nodes.iter().map(|node| node.id).collect();
        let layer = self.current_layer();
        self.apply_and_record(Edit::Added { layer, nodes });
    }

    /// Deletes everything selected.
    ///
    /// Returns how many brushes were deleted.
    pub fn delete_selection(&mut self) -> usize {
        if self.selection.is_empty() { return 0; }
        let mut parts = Vec::new();
        let mut count = 0;
        let selected: Vec<_> = self.selection.iter().copied().collect();
        for (layer, contents) in self.tree.world.layers.iter().enumerate() {
            let mut removed = Vec::new();
            for root in &contents.children {
                if self.selection.contains(&root.id) {
                    removed.push(root.clone());
                    count += 1;
                } else {
                    let mut ids = HashSet::new();
                    super::add_ids_under(root, &mut ids);
                    let selected_count = ids.intersection(&self.selection).count();
                    if selected_count > 0 {
                        let mut after = root.clone();
                        let children = match &mut after.kind {
                            NodeKind::Group(g) => &mut g.children,
                            NodeKind::Entity(e) => &mut e.children,
                            NodeKind::Layer(l) => &mut l.children,
                            _ => continue,
                        };
                        count += selected_count;
                        node::remove_nodes(children, &selected);
                        parts.push(Edit::Changed { before: vec![root.clone()], after: vec![after] });
                    }
                }
            }
            if !removed.is_empty() {
                let before: Vec<_> = contents.children.iter().map(|n| n.id).collect();
                let after = before.iter().copied().filter(|id| !self.selection.contains(id)).collect();
                parts.push(Edit::Reordered { layer, before, after });
                parts.push(Edit::Removed { layer, nodes: removed });
            }
        }
        if parts.is_empty() { return 0; }
        self.apply_and_record(Edit::Batch(parts));
        count
    }

    /// The outlines of the faces a Shift-drag from the pointer would extrude: the face
    /// under the ray, and every face coplanar with it (same normal) on any selected
    /// brush.
    ///
    /// `ExtrudeTool::getDragHandles` -> `collectCoplanarFaces`, without the opposing-seam
    /// case. Empty when nothing selected, the selection is not all brushes, or the ray
    /// misses.
    pub fn extrude_preview_faces(&self, ray: &Ray3d) -> Vec<Vec<Vec3d>> {
        if !self.selection_is_only_brushes() {
            return Vec::new();
        }
        let Some(hit) = self.pick(ray) else {
            return Vec::new();
        };
        let Some(face_index) = hit.face else {
            return Vec::new();
        };
        let Some(hit_brush) = self.brush(hit.node) else {
            return Vec::new();
        };
        let reference = hit_brush
            .geometry()
            .face_plane(map_model::polyhedron::FaceId(face_index as u32));
        self.coplanar_selected_face_outlines(reference.normal, reference.distance)
    }

    /// Finds the nearest valid corner beyond both visible selection bounds.
    pub fn extrude_corner_2d(
        &self,
        ray: &Ray3d,
    ) -> Option<(Brush, crate::extrude_tool::HorizonCorner)> {
        let axes: Vec<_> = (0..3)
            .filter_map(|i| {
                let mut axis = Vec3d::zero();
                axis[i] = 1.0;
                (vec::dot(axis, ray.direction).abs() < 1e-6).then_some(axis)
            })
            .collect();
        if axes.len() != 2 {
            return None;
        }
        let directions = crate::extrude_tool::corner_directions(
            &self.selection_bounds()?,
            ray.origin,
            [axes[0], axes[1]],
        )?;
        self.selected_brushes()
            .into_iter()
            .filter_map(|brush| {
                crate::extrude_tool::find_horizon_corner(brush, directions, ray.origin)
                    .map(|corner| (brush.clone(), corner))
            })
            .min_by(|(_, a), (_, b)| {
                vec::squared_distance(a.handle, ray.origin)
                    .total_cmp(&vec::squared_distance(b.handle, ray.origin))
            })
    }

    /// The same preview for a flat view, where a face is seen edge-on and there is
    /// nothing to pick under the pointer.
    ///
    /// `ExtrudeTool::pick2D`: the reference face is the one behind the silhouette edge
    /// nearest the pointer, chosen across the selected brushes the way `begin_extrude_2d`
    /// chooses the edge to drag.
    pub fn extrude_preview_faces_2d(&self, ray: &Ray3d) -> Vec<Vec<Vec3d>> {
        if !self.selection_is_only_brushes() {
            return Vec::new();
        }
        let reference = self
            .selected_brushes()
            .into_iter()
            .filter_map(|brush| crate::extrude_tool::find_closest_horizon_edge(brush, ray))
            .min_by(|a, b| {
                a.from_ray
                    .partial_cmp(&b.from_ray)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        match reference {
            // The handle sits on the edge, which lies in the face's plane, so the plane's
            // offset is the handle projected onto its normal.
            Some(edge) => self.coplanar_selected_face_outlines(
                edge.normal,
                vec::dot(edge.normal, edge.handle),
            ),
            None => Vec::new(),
        }
    }

    /// Outlines every selected brush's face lying in the given plane -- the coplanar set
    /// `collectCoplanarFaces` gathers, without the opposing-seam case.
    fn coplanar_selected_face_outlines(
        &self,
        reference_normal: Vec3d,
        reference_distance: f64,
    ) -> Vec<Vec<Vec3d>> {
        let mut outlines = Vec::new();
        for (id, brush) in node::brushes_with_ids(&self.tree.world) {
            if !self.selection.contains(&id) {
                continue;
            }
            let geometry = brush.geometry();
            for index in 0..geometry.face_count() {
                let plane = geometry.face_plane(map_model::polyhedron::FaceId(index as u32));
                if vec::dot(plane.normal, reference_normal) > 0.999
                    && (plane.distance - reference_distance).abs() < 0.1
                {
                    outlines.push(
                        geometry.face_positions(map_model::polyhedron::FaceId(index as u32)),
                    );
                }
            }
        }
        outlines
    }

    /// Replaces every selected brush face with the flat Bézier patch (or patches, for a
    /// non-quad face) covering it, deleting the brushes those faces belong to.
    ///
    /// Converts selected faces to patches with `points` control points per axis. The new patches
    /// are selected in one undo step.
    pub fn convert_selection_to_patches(&mut self, points: usize) -> usize {
        let refs: Vec<FaceRef> = self.face_selection.iter().copied().collect();
        if refs.is_empty() {
            return 0;
        }

        let mut sources: Vec<(map_model::brush_face::BrushFace, Vec<Vec3d>)> = Vec::new();
        let mut doomed_ids: HashSet<NodeId> = HashSet::new();
        for reference in &refs {
            let Some(brush) = self.brush(reference.node) else {
                continue;
            };
            let polygon = brush
                .geometry()
                .face_positions(map_model::polyhedron::FaceId(reference.face as u32));
            let Some(face) = brush
                .source_face_index(reference.face)
                .and_then(|index| brush.faces().get(index))
            else {
                continue;
            };
            sources.push((face.clone(), polygon));
            doomed_ids.insert(reference.node);
        }

        let layer = self.current_layer();
        let mut patch_nodes: Vec<Node> = Vec::new();
        for (face, polygon) in &sources {
            // The patch keeps the map format's UV coordinates, which count copies of the
            // material rather than texels, so the converter needs its size.
            let size = self
                .materials
                .get(&face.material_name)
                .map(|m| Vec2f::new(m.width.max(1) as f32, m.height.max(1) as f32))
                .unwrap_or(Vec2f::new(64.0, 64.0));
            for patch in map_model::face_to_patch::create_patches(face, polygon, points, points, size) {
                patch_nodes.push(Node::new(NodeKind::Patch(patch)));
            }
        }
        if patch_nodes.is_empty() {
            return 0;
        }

        let count = patch_nodes.len();
        let new_ids: Vec<NodeId> = patch_nodes.iter().map(|node| node.id).collect();
        let doomed: Vec<Node> = doomed_ids
            .iter()
            .filter_map(|id| node::find_node(&self.tree.world, *id).cloned())
            .collect();

        self.apply_and_record(Edit::Batch(vec![
            Edit::Removed {
                layer: 0,
                nodes: doomed,
            },
            Edit::Added {
                layer,
                nodes: patch_nodes,
            },
        ]));

        self.face_selection.clear();
        self.selection = new_ids.into_iter().collect();
        self.mark_changed();
        count
    }
}
