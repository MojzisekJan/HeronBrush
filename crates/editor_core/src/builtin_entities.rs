//! The common HERON_BRUSH entity vocabulary is embedded so every installed editor has it.
use map_model::fgd::EntityDefinitions;

/// The same FGD is used by the editor and the export builder.
pub const SOURCE: &str = include_str!("../../../gamedata/heron_brush/heron_brush.fgd");

/// Parse the bundled, tested definitions without depending on a working directory.
pub fn definitions() -> EntityDefinitions {
    map_model::fgd::read_fgd(SOURCE).expect("bundled HERON_BRUSH FGD must be valid")
}

/// Explicit project definitions win on a classname collision. Unrelated built-ins remain.
pub fn with_project(project: EntityDefinitions) -> EntityDefinitions {
    let mut merged = definitions().all().to_vec();
    for definition in project.all() {
        if let Some(existing) = merged.iter_mut().find(|d| d.name == definition.name) {
            *existing = definition.clone();
        } else {
            merged.push(definition.clone());
        }
    }
    EntityDefinitions::new(merged, project.warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builtin_point_icons_load_without_external_files() {
        let mut document = crate::document::Document::new(crate::world::WorldBounds::default());
        document.create_point_entity("ambient_sound", geometry_math::vec::Vec3d::zero());
        for definition in definitions().placeable() {
            assert!(
                crate::entity_icons::kind(&definition.name).is_some(),
                "missing icon: {}",
                definition.name
            );
            if definition.kind == map_model::fgd::DefinitionKind::Point {
                let sprite = crate::entity_icons::sprite(&definition.name);
                assert!(sprite.pixels.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
                assert!(sprite.pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
                if definition.name != "light" {
                    assert!(document
                        .materials()
                        .get(&format!("#model/builtin/{}", definition.name))
                        .is_some());
                }
            }
        }
    }
    #[test]
    fn the_common_entities_are_available_without_a_game_configuration() {
        let document = crate::document::Document::new(crate::world::WorldBounds::default());
        let definitions = document.definitions().unwrap();
        for name in [
            "light",
            "light_spot",
            "light_sun",
            "ambient_sound",
            "info_player_start",
            "func_door",
            "func_button",
            "trigger_once",
            "trigger_multiple",
        ] {
            assert!(definitions.find(name).is_some(), "missing {name}");
        }
        let (classes, warnings) = crate::export::entity_classes(&document);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(classes["light"].directives.get("node"), Some("OmniLight3D"));
        assert_eq!(
            classes["trigger_multiple"].directives.get("collision"),
            Some("convex")
        );
        assert_eq!(
            classes["trigger_multiple"].directives.get("visuals"),
            Some("false")
        );
        assert!(classes["func_door"].properties.contains_key("targetname"));
    }
    #[test]
    fn project_classes_override_builtins_without_hiding_other_classes() {
        let custom = map_model::fgd::read_fgd("@PointClass = light : \"Custom light\" []").unwrap();
        let merged = with_project(custom);
        assert_eq!(merged.find("light").unwrap().description, "Custom light");
        assert!(merged.find("func_door").is_some());
        assert_eq!(merged.all().iter().filter(|d| d.name == "light").count(), 1);
    }

    /// A Quake map is offered Quake's classes, not `light_spot` or `prop_static`, but an
    /// entity of a built-in class it already has still finds its definition.
    #[test]
    fn a_game_without_the_builtins_is_not_offered_them_but_still_knows_them() {
        let fgd =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gamedata/Quake/Quake.fgd");
        let mut document = crate::document::Document::new(crate::world::WorldBounds::default());
        document.set_offer_builtin_entities(false);
        document.load_definitions(&fgd).expect("Quake's FGD loads");
        let offered = |document: &crate::document::Document| -> Vec<String> {
            document
                .definitions()
                .expect("definitions are loaded")
                .placeable()
                .map(|definition| definition.name.clone())
                .collect()
        };
        let names = offered(&document);
        assert!(names.iter().any(|name| name == "light"));
        assert!(names.iter().any(|name| name == "monster_army"));
        assert!(!names.iter().any(|name| name == "light_spot"));
        assert!(!names.iter().any(|name| name == "prop_static"));
        let definitions = document.definitions().expect("definitions are loaded");
        assert!(definitions.find("light_spot").is_some());

        document.set_offer_builtin_entities(true);
        assert!(offered(&document).iter().any(|name| name == "light_spot"));
    }
}
