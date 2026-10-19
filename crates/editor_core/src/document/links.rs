//! The entity link graph: which entity fires which, as nodes and arrows for the Entity
//! links window and the `entity_graph` MCP tool, and making or breaking a link.
//!
//! The rules are those the views draw links by ([`Document::entity_links`]) and the
//! validator checks: a targeting key (`target`, `killtarget`, and whatever the FGD
//! declares `target_destination`, with or without a trailing number) names an entity's
//! name (`targetname`, or a key the FGD declares `target_source`).

use std::collections::{HashMap, HashSet};

use geometry_math::vec::Vec3d;
use map_model::node::{self, NodeId};

use super::{entity_touched, fold_contents, Document};

/// An entity in the graph: one that names itself or aims at another.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub id: NodeId,
    pub classname: String,
    /// What others aim at it by, if it answers to anything.
    pub name: Option<String>,
    /// Where it is in the map: its origin, or the middle of its brushes.
    pub position: Option<Vec3d>,
    pub selected: bool,
}

/// One link: `from` aims at `to` with `key` = `value`.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub key: String,
    pub value: String,
}

/// Every entity that takes part in linking, and every link between them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntityGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl EntityGraph {
    /// Each node's column and row for drawing: a node's column is the length of the
    /// longest chain of links leading into it, so arrows run left to right. The last
    /// column is ordered by class and name; each column before it by where the nodes it
    /// aims at sit, so the arrows cross as little as one sweep can manage. Cycles are cut
    /// where they close.
    pub fn layout(&self) -> HashMap<NodeId, (usize, usize)> {
        let ids: Vec<NodeId> = self.nodes.iter().map(|n| n.id).collect();
        let mut column: HashMap<NodeId, usize> = ids.iter().map(|id| (*id, 0)).collect();
        // Longest path by relaxing the edges at most once per node: enough for a DAG, and
        // a cycle simply stops growing then.
        for _ in 0..ids.len() {
            let mut changed = false;
            for edge in &self.edges {
                if edge.from == edge.to {
                    continue;
                }
                let next = column[&edge.from] + 1;
                if next > column[&edge.to] && next < ids.len() {
                    column.insert(edge.to, next);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut by_column: Vec<Vec<&GraphNode>> = Vec::new();
        for node in &self.nodes {
            let c = column[&node.id];
            if by_column.len() <= c {
                by_column.resize(c + 1, Vec::new());
            }
            by_column[c].push(node);
        }
        let mut placed: HashMap<NodeId, (usize, usize)> = HashMap::new();
        for c in (0..by_column.len()).rev() {
            // The mean row of what each node aims at in later columns, where those are
            // placed already; nodes aiming at nothing placed go last, by name.
            let target_row = |node: &GraphNode| -> f64 {
                let rows: Vec<f64> = self
                    .edges
                    .iter()
                    .filter(|e| e.from == node.id)
                    .filter_map(|e| placed.get(&e.to).map(|(_, row)| *row as f64))
                    .collect();
                if rows.is_empty() { f64::INFINITY } else { rows.iter().sum::<f64>() / rows.len() as f64 }
            };
            let mut nodes = by_column[c].clone();
            nodes.sort_by(|a, b| {
                target_row(a)
                    .partial_cmp(&target_row(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| (&a.classname, &a.name).cmp(&(&b.classname, &b.name)))
            });
            for (row, node) in nodes.iter().enumerate() {
                placed.insert(node.id, (c, row));
            }
        }
        placed
    }
}

impl Document {
    /// The entity link graph ([`EntityGraph`]).
    pub fn entity_graph(&self) -> EntityGraph {
        let entities = node::entities_with_ids(&self.tree.world);
        let keys = self.link_keys();
        let mut graph = EntityGraph::default();
        let mut answers: HashMap<NodeId, Vec<&str>> = HashMap::new();
        for (id, entity) in &entities {
            let classname = entity.classname().unwrap_or("");
            if classname.eq_ignore_ascii_case("worldspawn") {
                continue;
            }
            let names: Vec<&str> = entity.properties.iter().filter(|(k, v)| keys.is_source(k) && !v.is_empty()).map(|(_, v)| v.as_str()).collect();
            let aims = entity.properties.iter().any(|(k, v)| keys.is_target(k) && !v.is_empty());
            if names.is_empty() && !aims {
                continue;
            }
            let position = entity.origin().or_else(|| fold_contents(&entity.children, &self.selection).bounds.map(|b| b.center()));
            graph.nodes.push(GraphNode {
                id: *id,
                classname: classname.to_string(),
                name: names.first().map(|n| n.to_string()),
                position,
                selected: entity_touched(*id, &entity.children, &self.selection),
            });
            answers.insert(*id, names);
        }
        for (id, entity) in &entities {
            if !answers.contains_key(id) {
                continue;
            }
            for (key, value) in &entity.properties {
                if !keys.is_target(key) || value.is_empty() {
                    continue;
                }
                for node in &graph.nodes {
                    if node.id != *id && answers[&node.id].iter().any(|name| name == value) {
                        graph.edges.push(GraphEdge { from: *id, to: node.id, key: key.clone(), value: value.clone() });
                    }
                }
            }
        }
        graph
    }

    /// Makes `from` aim at `to`, as one undo step: `to` keeps its name, or is given one
    /// from its class (`func_door_2`), and `from` takes it under its first free targeting
    /// key (`target`, then `target2`, ...). Returns the name linked by.
    pub fn link_entities(&mut self, from: NodeId, to: NodeId) -> Result<String, String> {
        if from == to {
            return Err("an entity cannot aim at itself".into());
        }
        let entities = node::entities_with_ids(&self.tree.world);
        let find = |id: NodeId| entities.iter().find(|(e, _)| *e == id).map(|(_, entity)| *entity);
        let (Some(source), Some(target)) = (find(from), find(to)) else {
            return Err("both ends must be entities".into());
        };
        let keys = self.link_keys();
        let existing = target.properties.iter().find(|(k, v)| keys.is_source(k) && !v.is_empty()).map(|(_, v)| v.clone());
        let name = match existing {
            Some(name) => name,
            None => {
                let taken: HashSet<&str> = entities.iter().flat_map(|(_, e)| e.properties.iter().filter(|(k, _)| keys.is_source(k)).map(|(_, v)| v.as_str())).collect();
                let stem = target.classname().unwrap_or("entity").to_string();
                (1..).map(|n| if n == 1 { stem.clone() } else { format!("{stem}_{n}") }).find(|candidate| !taken.contains(candidate.as_str())).unwrap_or(stem)
            }
        };
        if source.properties.iter().any(|(k, v)| keys.is_target(k) && *v == name) {
            return Err(format!("it already aims at {name}"));
        }
        let key = (1..)
            .map(|n| if n == 1 { "target".to_string() } else { format!("target{n}") })
            .find(|key| !source.properties.iter().any(|(k, v)| k == key && !v.is_empty()))
            .unwrap_or_else(|| "target".into());
        let set_name = target.properties.iter().all(|(k, v)| !(keys.is_source(k) && !v.is_empty()));
        self.run_as_one_edit(|document| {
            if set_name {
                document.set_entity_property(to, "targetname", &name);
            }
            document.set_entity_property(from, &key, &name);
        });
        Ok(name)
    }

    /// Breaks the link `from` makes under `key`, as one undo step.
    pub fn unlink_entities(&mut self, from: NodeId, key: &str) -> bool {
        self.remove_entity_property_on(&[from], key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::WorldBounds;
    use map_model::io::map_reader::read_map;

    fn sample() -> Document {
        let source = r#"
{
"classname" "worldspawn"
}
{
"classname" "func_button"
"target" "gate"
"origin" "0 0 0"
}
{
"classname" "trigger_once"
"target" "gate"
"killtarget" "lamp"
"origin" "64 0 0"
}
{
"classname" "func_door"
"targetname" "gate"
"origin" "128 0 0"
}
{
"classname" "light"
"targetname" "lamp"
"origin" "0 64 0"
}
{
"classname" "info_player_start"
"origin" "0 0 32"
}
"#;
        Document::from_map(&read_map(source, None).expect("the sample parses"), WorldBounds::default(), None)
    }

    fn id_of(graph: &EntityGraph, classname: &str) -> NodeId {
        graph.nodes.iter().find(|n| n.classname == classname).map(|n| n.id).expect(classname)
    }

    /// The graph holds the entities that link and the links between them, and lays them
    /// out with every arrow running to a later column.
    #[test]
    fn the_graph_holds_the_links_and_lays_them_left_to_right() {
        let document = sample();
        let graph = document.entity_graph();
        let classes: HashSet<&str> = graph.nodes.iter().map(|n| n.classname.as_str()).collect();
        assert_eq!(classes, ["func_button", "trigger_once", "func_door", "light"].into_iter().collect());
        assert_eq!(graph.edges.len(), 3);
        let (button, trigger, door, light) = (id_of(&graph, "func_button"), id_of(&graph, "trigger_once"), id_of(&graph, "func_door"), id_of(&graph, "light"));
        assert!(graph.edges.iter().any(|e| e.from == button && e.to == door && e.key == "target"));
        assert!(graph.edges.iter().any(|e| e.from == trigger && e.to == light && e.key == "killtarget"));
        let layout = graph.layout();
        for edge in &graph.edges {
            assert!(layout[&edge.from].0 < layout[&edge.to].0, "{edge:?}");
        }
        assert_eq!(layout[&button].0, 0);
        assert_eq!(layout[&door].0, 1);
    }

    /// The views' links carry their key: the killtarget one says it kills, and selecting
    /// a brush entity's brush counts as selecting the entity.
    #[test]
    fn view_links_know_their_key() {
        let document = sample();
        let links = document.entity_links();
        let kill = links.iter().find(|link| link.key == "killtarget").expect("the killtarget link");
        assert!(kill.kills() && kill.name == "lamp");
        assert_eq!(kill.label(), "killtarget → lamp");
        assert!(links.iter().filter(|link| link.key == "target").all(|link| !link.kills()));
    }

    /// Linking names an unnamed target from its class, uses the next free key, and is
    /// one undo step; unlinking removes the key.
    #[test]
    fn linking_and_unlinking_are_single_steps() {
        let mut document = sample();
        let graph = document.entity_graph();
        let (button, door) = (id_of(&graph, "func_button"), id_of(&graph, "func_door"));
        // The player start has no name: linking the button to it names it.
        let player = node::entities_with_ids(document.tree_world()).iter().find(|(_, e)| e.classname() == Some("info_player_start")).map(|(id, _)| *id).unwrap();
        let name = document.link_entities(button, player).unwrap();
        assert_eq!(name, "info_player_start");
        let graph = document.entity_graph();
        assert!(graph.edges.iter().any(|e| e.from == button && e.to == player && e.key == "target2"));
        assert!(document.link_entities(button, door).is_err(), "already linked");
        document.undo();
        let graph = document.entity_graph();
        assert!(!graph.nodes.iter().any(|n| n.id == player), "undo took both the name and the link");
        assert!(document.unlink_entities(button, "target"));
        assert!(!document.entity_graph().edges.iter().any(|e| e.from == button));
    }
}
