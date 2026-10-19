//! The right-click menu in a map view, as data: which commands it offers for the
//! selection and what the pointer is over -- group, layer, entity-class and material
//! actions -- and what each does. A frontend draws [`build`]'s sections and hands the
//! chosen [`Action`] to [`run`].

use std::collections::BTreeMap;

use geometry_math::vec::Vec3d;
use map_model::node::NodeId;

use crate::document::Document;
use crate::i18n::tr;
use crate::view2d::Probe;

/// What a menu item does.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Group,
    Ungroup,
    MergeGroups,
    AddToGroup,
    RemoveFromGroup,
    LinkedDuplicate,
    SelectLinked,
    SeparateLinked,
    ExtractLinked,
    MakeStructural,
    MoveToLayer(usize),
    MakeLayerActive(usize),
    SelectInLayer(usize),
    HideLayers,
    IsolateLayers,
    SelectAllInLayers,
    SelectClassname(String),
    MoveToEntity(NodeId),
    RevealMaterial(String),
    CopyMaterial(String),
    /// A point entity of this class, placed where the pointer was.
    CreatePoint(String, Vec3d),
    CreateBrushEntity(String),
    /// Round the picked edges: the Fillet tool with its settings open.
    Fillet,
}

/// One entry of the menu.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// A command.
    Action(String, Action),
    /// A submenu.
    Submenu(String, Vec<Item>),
}

/// What running an action asks of the frontend beyond redrawing and refreshing.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Nothing more.
    Done,
    /// Bring this material up in the browser: select it and show the Face tab.
    Reveal(String),
    /// Put this text on the clipboard.
    Copy(String),
    /// Switch to the Fillet tool, keeping the picked edges, and open its settings.
    Fillet,
}

/// The section for picked edges, put first in the menu: rounding them. `None` with no
/// edge picked.
pub fn edge_section(edges: usize) -> Option<Vec<Item>> {
    (edges > 0).then(|| vec![Item::Action(tr("Fillet").to_string(), Action::Fillet)])
}

/// The menu for the current selection and what the pointer is over, in sections. Empty
/// when there is nothing to offer, in which case no menu opens.
pub fn build(document: &Document, probe: &Probe) -> Vec<Vec<Item>> {
    let mut menu = Vec::new();
    let has_selection = document.has_selection();
    let open_group = document.open_group().is_some();
    let action = |label: &str, action: Action| Item::Action(label.to_string(), action);

    if has_selection {
        let mut groups = vec![
            action(tr("Group"), Action::Group),
            action(tr("Ungroup"), Action::Ungroup),
            action(tr("Merge groups"), Action::MergeGroups),
        ];
        if open_group {
            groups.push(action(tr("Add to group"), Action::AddToGroup));
            groups.push(action(tr("Take out of group"), Action::RemoveFromGroup));
        }
        menu.push(groups);

        menu.push(vec![
            action(tr("Create linked duplicate"), Action::LinkedDuplicate),
            action(tr("Select linked groups"), Action::SelectLinked),
            action(tr("Separate linked groups"), Action::SeparateLinked),
            action(tr("Extract linked groups"), Action::ExtractLinked),
        ]);

        let layers = document
            .layers()
            .into_iter()
            .map(|layer| Item::Action(layer.name.clone(), Action::MoveToLayer(layer.index)))
            .collect();
        let mut place = vec![
            Item::Submenu(tr("Move to layer").to_string(), layers),
            action(tr("Move selection out of its entity"), Action::MakeStructural),
        ];
        if let Some(entity) = probe.entity.as_ref().filter(|entity| entity.is_brush_entity) {
            place.push(Item::Action(
                format!("{} {}", tr("Move to entity"), entity.classname),
                Action::MoveToEntity(entity.id),
            ));
        }
        menu.push(place);

        // The layers the selection sits on, `collectContainingLayersUserSorted`.
        let containing = document.layers_of_selection();
        if !containing.is_empty() {
            let names = document.layers();
            let name_of = |index: usize| {
                names
                    .get(index)
                    .map_or_else(|| format!("layer {index}"), |layer| layer.name.clone())
            };
            let mut section = Vec::new();
            if containing.len() == 1 {
                let index = containing[0];
                section.push(Item::Action(
                    format!("{} {}", tr("Make active layer"), name_of(index)),
                    Action::MakeLayerActive(index),
                ));
                section.push(action(tr("Select all in layer"), Action::SelectInLayer(index)));
            } else {
                section.push(Item::Submenu(
                    tr("Make active layer").to_string(),
                    containing.iter().map(|&index| Item::Action(name_of(index), Action::MakeLayerActive(index))).collect(),
                ));
                section.push(action(tr("Select all in layers"), Action::SelectAllInLayers));
            }
            section.push(action(tr("Hide layers"), Action::HideLayers));
            section.push(action(tr("Isolate layers"), Action::IsolateLayers));
            menu.push(section);
        }
    }

    if let Some(entity) = &probe.entity {
        // "Select all <classname>", the class spelled into the label.
        menu.push(vec![Item::Action(
            format!("{} {}", tr("Select all"), entity.classname),
            Action::SelectClassname(entity.classname.clone()),
        )]);
    }

    if let Some(material) = &probe.material {
        menu.push(vec![
            Item::Action(format!("{} {material}", tr("Reveal in material browser")), Action::RevealMaterial(material.clone())),
            action(tr("Copy material name"), Action::CopyMaterial(material.clone())),
        ]);
    }

    // `makeEntityGroupsMenu`, always there: a point class placed where the pointer is, or
    // a brush class made out of the selection. This is what keeps the menu worth opening
    // over empty space with nothing selected.
    if let Some(definitions) = document.definitions() {
        let mut section = Vec::new();
        let at = probe.at;
        if let Some(points) = entity_groups(definitions, map_model::fgd::DefinitionKind::Point, &|name| Action::CreatePoint(name, at)) {
            section.push(Item::Submenu(tr("Create point entity").to_string(), points));
        }
        if let Some(brushes) = entity_groups(definitions, map_model::fgd::DefinitionKind::Solid, &Action::CreateBrushEntity) {
            section.push(Item::Submenu(tr("Create brush entity").to_string(), brushes));
        }
        if !section.is_empty() {
            menu.push(section);
        }
    }

    menu
}

/// The classes of one kind, grouped by the part of the name before the first `_`.
///
/// Groups entity classes by classname prefixes such as `func_`, `trigger_` and `item_`.
/// Singleton prefixes and names without underscores appear directly in the menu.
fn entity_groups(
    definitions: &map_model::fgd::EntityDefinitions,
    kind: map_model::fgd::DefinitionKind,
    make: &dyn Fn(String) -> Action,
) -> Option<Vec<Item>> {
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for definition in definitions.placeable() {
        if definition.kind != kind || definition.name.eq_ignore_ascii_case("worldspawn") {
            continue;
        }
        let prefix = definition.name.split_once('_').map_or("", |(head, _)| head);
        groups.entry(prefix).or_default().push(&definition.name);
    }
    if groups.is_empty() {
        return None;
    }
    let mut menu = Vec::new();
    for (prefix, mut names) in groups {
        names.sort_unstable();
        let leaves = names.iter().map(|name| Item::Action((*name).to_string(), make((*name).to_string())));
        if prefix.is_empty() || names.len() == 1 {
            menu.extend(leaves);
        } else {
            menu.push(Item::Submenu(prefix.to_string(), leaves.collect()));
        }
    }
    Some(menu)
}

/// Does what a menu item says. `grid_step` is the step a linked duplicate sits over by.
pub fn run(action: &Action, document: &mut Document, grid_step: f64) -> Outcome {
    match action {
        Action::Group => match document.group_selection("Group") {
            Some(_) => crate::say!("grouped the selection"),
            None => crate::say!("select something to group"),
        },
        Action::Ungroup => {
            let count = document.ungroup_selection();
            crate::say!("took apart {count} groups");
        }
        Action::MergeGroups => {
            let count = document.merge_selected_groups();
            crate::say!("merged {count} groups into one");
        }
        Action::AddToGroup => {
            let count = document.add_selection_to_open_group();
            crate::say!("moved {count} objects into the open group");
        }
        Action::RemoveFromGroup => {
            let count = document.remove_selection_from_open_group();
            crate::say!("moved {count} objects out of the open group");
        }
        Action::LinkedDuplicate => match document.create_linked_duplicate(grid_step) {
            Some(_) => crate::say!("made a linked duplicate"),
            None => crate::say!("select a whole group to make a linked duplicate"),
        },
        Action::SelectLinked => {
            document.select_linked_groups();
            crate::say!("selected the linked set");
        }
        Action::SeparateLinked => {
            let count = document.separate_selected_linked_groups();
            crate::say!("freed {count} groups from their set");
        }
        Action::ExtractLinked => match document.extract_linked_groups() {
            Some(groups) => crate::say!("extracted a linked set of {} groups", groups.len()),
            None => crate::say!("open a linked group and select some but not all of it to extract"),
        },
        Action::MakeStructural => {
            let count = document.make_structural();
            crate::say!("moved {count} objects out of their entity");
        }
        Action::MoveToLayer(index) => {
            let moved = document.move_selection_to_layer(*index);
            crate::say!("moved {moved} objects to the layer");
        }
        Action::MakeLayerActive(index) => document.set_current_layer(*index),
        Action::SelectInLayer(index) => {
            let count = document.select_layer(*index);
            crate::say!("selected {count} objects on the layer");
        }
        Action::HideLayers => {
            let layers = document.layers_of_selection();
            for index in &layers {
                document.set_layer_hidden(*index, true);
            }
            crate::say!("hid {} layers", layers.len());
        }
        Action::IsolateLayers => {
            let layers = document.layers_of_selection();
            document.isolate_layers(&layers);
            crate::say!("isolated {} layers", layers.len());
        }
        Action::SelectAllInLayers => {
            let layers = document.layers_of_selection();
            let count = document.select_layers(&layers);
            crate::say!("selected {count} objects on {} layers", layers.len());
        }
        Action::SelectClassname(name) => {
            let count = document.select_entities_with_classname(name);
            crate::say!("selected {count} objects of class {name}");
        }
        Action::MoveToEntity(id) => {
            let moved = document.move_selection_to_entity(*id);
            crate::say!("moved {moved} objects into the entity");
        }
        Action::RevealMaterial(name) => return Outcome::Reveal(name.clone()),
        Action::CopyMaterial(name) => {
            crate::say!("copied the material name");
            return Outcome::Copy(name.clone());
        }
        Action::CreatePoint(classname, at) => {
            document.create_point_entity(classname, *at);
            crate::say!("placed a {classname}");
        }
        Action::CreateBrushEntity(classname) => match document.create_brush_entity(classname) {
            Some(_) => crate::say!("made a {classname} out of the selection"),
            None => crate::say!("select some brushes to make a {classname}"),
        },
        Action::Fillet => return Outcome::Fillet,
    }
    Outcome::Done
}
