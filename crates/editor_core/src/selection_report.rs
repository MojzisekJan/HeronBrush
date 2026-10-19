//! What is selected, said for an agent: the ids the MCP tools take, where each object is,
//! how big it is and what it is made of.
//!
//! The person selects in the window and tells an agent "change this"; the agent reads
//! the selection with the `get_selection` or `describe_selection` tools, or the person
//! pastes [`text`] (the inspector's Copy for agent) into the conversation. Ids are the
//! node ids the map file keeps (`// heron_brush id`) and every MCP tool names objects by, as
//! decimal strings.

use map_model::node::{Node, NodeId, NodeKind};
use serde_json::{json, Value};

use crate::document::Document;

/// The entity an object belongs to, as the report names it.
#[derive(Clone, Copy)]
struct Owner<'a> {
    id: NodeId,
    classname: &'a str,
    name: Option<&'a str>,
}

/// Every selected brush, entity, group and patch, as JSON: `brushes` with their
/// entity, bounds, size and materials; `entities` with their class, name, origin and
/// brush count; `groups` with their names; `patches` with their bounds.
pub fn describe(document: &Document) -> Value {
    let selection = document.selection_ids();
    let mut report = Report::default();
    for layer in &document.tree_world().layers {
        walk(&layer.children, None, &layer.name, selection, &mut report);
    }
    json!({
        "map": document.level_name(),
        "count": selection.len(),
        "brushes": report.brushes,
        "entities": report.entities,
        "groups": report.groups,
        "patches": report.patches,
    })
}

/// The selection as a few lines of plain text, to paste into a conversation with an
/// agent driving the editor over MCP.
pub fn text(document: &Document) -> String {
    let report = describe(document);
    let list = |key: &str| report[key].as_array().cloned().unwrap_or_default();
    let (brushes, entities, groups, patches) = (list("brushes"), list("entities"), list("groups"), list("patches"));
    if brushes.is_empty() && entities.is_empty() && groups.is_empty() && patches.is_empty() {
        return format!("Nothing is selected in Heron Brush ({}).", report["map"].as_str().unwrap_or(""));
    }
    let mut out = format!(
        "Selected in Heron Brush, map \"{}\": {} brushes, {} entities, {} groups, {} patches.\n",
        report["map"].as_str().unwrap_or(""),
        brushes.len(),
        entities.len(),
        groups.len(),
        patches.len()
    );
    let number = |v: &Value| v.as_f64().map_or_else(|| "?".to_string(), |f| format!("{}", (f * 100.0).round() / 100.0));
    let triple = |v: &Value| format!("({} {} {})", number(&v[0]), number(&v[1]), number(&v[2]));
    for brush in &brushes {
        let materials: Vec<&str> = brush["materials"].as_array().map(|m| m.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
        out.push_str(&format!(
            "- brush {} in {}, {} units, from {} to {}, materials {}\n",
            brush["id"].as_str().unwrap_or(""),
            brush["entity"]["classname"].as_str().unwrap_or("worldspawn"),
            triple(&brush["size"]).trim_matches(|c| c == '(' || c == ')').replace(' ', " x "),
            triple(&brush["min"]),
            triple(&brush["max"]),
            materials.join(", ")
        ));
    }
    for entity in &entities {
        let name = entity["name"].as_str().map(|n| format!(" \"{n}\"")).unwrap_or_default();
        let at = if entity["origin"].is_null() { String::new() } else { format!(" at {}", triple(&entity["origin"])) };
        out.push_str(&format!(
            "- entity {} {}{name}{at}, {} brushes\n",
            entity["id"].as_str().unwrap_or(""),
            entity["classname"].as_str().unwrap_or(""),
            entity["brushes"]
        ));
    }
    for group in &groups {
        out.push_str(&format!("- group {} \"{}\"\n", group["id"].as_str().unwrap_or(""), group["name"].as_str().unwrap_or("")));
    }
    for patch in &patches {
        out.push_str(&format!("- patch {} from {} to {}\n", patch["id"].as_str().unwrap_or(""), triple(&patch["min"]), triple(&patch["max"])));
    }
    out.push_str("The ids are what the Heron Brush MCP tools take (describe_brush, select, translate_selection and the rest).");
    out
}

#[derive(Default)]
struct Report {
    brushes: Vec<Value>,
    entities: Vec<Value>,
    groups: Vec<Value>,
    patches: Vec<Value>,
}

fn walk(nodes: &[Node], owner: Option<Owner<'_>>, layer: &str, selection: &std::collections::HashSet<NodeId>, report: &mut Report) {
    for node in nodes {
        let selected = selection.contains(&node.id);
        match &node.kind {
            NodeKind::Brush(brush) => {
                if selected {
                    let bounds = brush.bounds();
                    let mut materials: Vec<&str> = brush.faces().iter().map(|face| face.material_name.as_str()).collect();
                    materials.sort_unstable();
                    materials.dedup();
                    report.brushes.push(json!({
                        "id": node.id.0.to_string(),
                        "entity": owner.map_or_else(
                            || json!({"classname": "worldspawn"}),
                            |o| json!({"id": o.id.0.to_string(), "classname": o.classname, "name": o.name}),
                        ),
                        "layer": layer,
                        "min": [tidy(bounds.min.x()), tidy(bounds.min.y()), tidy(bounds.min.z())],
                        "max": [tidy(bounds.max.x()), tidy(bounds.max.y()), tidy(bounds.max.z())],
                        "size": [
                            tidy(bounds.max.x() - bounds.min.x()),
                            tidy(bounds.max.y() - bounds.min.y()),
                            tidy(bounds.max.z() - bounds.min.z()),
                        ],
                        "faces": brush.faces().len(),
                        "materials": materials,
                    }));
                }
            }
            NodeKind::Entity(entity) => {
                let classname = entity.classname().unwrap_or("");
                let name = entity.property("targetname").or_else(|| entity.property("name"));
                if selected {
                    let brushes = count_brushes(&entity.children);
                    report.entities.push(json!({
                        "id": node.id.0.to_string(),
                        "classname": classname,
                        "name": name,
                        "layer": layer,
                        "origin": entity.origin().map(|o| [tidy(o.x()), tidy(o.y()), tidy(o.z())]),
                        "brushes": brushes,
                    }));
                }
                walk(&entity.children, Some(Owner { id: node.id, classname, name }), layer, selection, report);
            }
            NodeKind::Group(group) => {
                if selected {
                    report.groups.push(json!({"id": node.id.0.to_string(), "name": group.name, "linked": group.linked_group_id.is_some()}));
                }
                walk(&group.children, owner, layer, selection, report);
            }
            NodeKind::Layer(inner) => walk(&inner.children, owner, &inner.name, selection, report),
            NodeKind::Patch(patch) => {
                if selected {
                    let (min, max) = patch_bounds(patch);
                    report.patches.push(json!({"id": node.id.0.to_string(), "layer": layer, "min": min, "max": max}));
                }
            }
        }
    }
}

fn patch_bounds(patch: &map_model::io::map_reader::Patch) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for point in &patch.control_points {
        let p = [point.position.x(), point.position.y(), point.position.z()];
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    (min.map(tidy), max.map(tidy))
}

/// A coordinate to a ten-thousandth of a unit: the plane intersections that give a
/// brush's corners leave noise like 128.00000000000023, which an agent should not have
/// to read through.
fn tidy(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

/// The brushes under `nodes`, inside groups too.
fn count_brushes(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| match &node.kind {
            NodeKind::Brush(_) => 1,
            NodeKind::Group(group) => count_brushes(&group.children),
            NodeKind::Layer(layer) => count_brushes(&layer.children),
            NodeKind::Entity(entity) => count_brushes(&entity.children),
            NodeKind::Patch(_) => 0,
        })
        .sum()
}
