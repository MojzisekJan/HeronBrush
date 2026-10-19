//! Tests for reading entity definitions from an FGD file.
//!
//! The samples are in the shape real FGD files take, since the format has no
//! specification and what matters is that the files games actually ship parse.

use map_model::fgd::{read_fgd, DefinitionKind, PropertyType};
use geometry_math::vec::Vec3d;

const SAMPLE: &str = r#"
// A comment at the top, as every FGD has.
@baseclass = Appearflags
[
    spawnflags(flags) =
    [
        256 : "Not in Easy" : 0
        512 : "Not in Normal" : 0
        1024 : "Not in Hard" : 1
    ]
]

@baseclass = Targetname
[
    targetname(target_source) : "Name"
]

@PointClass base(Appearflags, Targetname) size(-16 -16 -24, 16 16 32) color(0 255 0) =
    info_player_start : "Player start"
[
    angle(integer) : "Direction" : 0 : "Which way the player faces."
]

@SolidClass base(Targetname) = func_door : "A door" +
    " that opens."
[
    speed(integer) : "Speed" : 100
    sounds(choices) : "Sound" : 0 =
    [
        0 : "None"
        1 : "Stone"
        2 : "Machine"
    ]
    wait(float) : "Wait" : 3
]

@PointClass size(-8 -8 -8, 8 8 8) color(255 255 0) model({ "path": "progs/light.mdl" }) =
    light : "A light"
[
    light(integer) : "Brightness" : 300
    _color(color255) : "Colour" : "255 255 255"
]
"#;

#[test]
fn every_class_is_read() {
    let definitions = read_fgd(SAMPLE).expect("the sample parses");

    assert_eq!(definitions.all().len(), 5);
    assert_eq!(definitions.placeable().count(), 3);

    assert!(definitions.find("Appearflags").is_some());
    assert!(definitions.find("info_player_start").is_some());
    assert!(definitions.find("nothing_like_this").is_none());
}

#[test]
fn a_base_class_is_not_placeable() {
    let definitions = read_fgd(SAMPLE).expect("the sample parses");
    let base = definitions.find("Appearflags").expect("it is defined");

    assert_eq!(base.kind, DefinitionKind::Base);
    assert!(!base.is_placeable());
}

#[test]
fn a_point_class_keeps_its_size_and_colour() {
    let definitions = read_fgd(SAMPLE).expect("the sample parses");
    let spawn = definitions
        .find("info_player_start")
        .expect("it is defined");

    assert_eq!(spawn.kind, DefinitionKind::Point);
    assert_eq!(spawn.description, "Player start");

    let bounds = spawn.bounds.expect("it has a size");
    assert_eq!(bounds.min, Vec3d::new(-16.0, -16.0, -24.0));
    assert_eq!(bounds.max, Vec3d::new(16.0, 16.0, 32.0));
    assert_eq!(spawn.color, Some([0, 255, 0]));
}

#[test]
fn a_description_split_across_lines_is_joined() {
    let definitions = read_fgd(SAMPLE).expect("the sample parses");
    let door = definitions.find("func_door").expect("it is defined");

    assert_eq!(door.kind, DefinitionKind::Solid);
    assert_eq!(door.description, "A door that opens.");
}

#[test]
fn properties_keep_their_type_name_and_default() {
    let definitions = read_fgd(SAMPLE).expect("the sample parses");
    let spawn = definitions
        .find("info_player_start")
        .expect("it is defined");

    let angle = spawn.property("angle").expect("it has an angle");
    assert_eq!(angle.kind, PropertyType::Integer);
    assert_eq!(angle.short_description, "Direction");
    assert_eq!(angle.default.as_deref(), Some("0"));
    assert_eq!(
        angle.long_description.as_deref(),
        Some("Which way the player faces.")
    );
}

#[test]
fn a_float_property_is_told_from_an_integer_one() {
    let definitions = read_fgd(SAMPLE).expect("the sample parses");
    let door = definitions.find("func_door").expect("it is defined");

    assert_eq!(
        door.property("speed").expect("it has a speed").kind,
        PropertyType::Integer
    );
    assert_eq!(
        door.property("wait").expect("it has a wait").kind,
        PropertyType::Float
    );
}

#[test]
fn a_choices_property_keeps_its_options() {
    let definitions = read_fgd(SAMPLE).expect("the sample parses");
    let door = definitions.find("func_door").expect("it is defined");

    let PropertyType::Choices(choices) = &door.property("sounds").expect("it has sounds").kind
    else {
        panic!("sounds should be a choices property");
    };
    assert_eq!(choices.len(), 3);
    assert_eq!(choices[0].value, "0");
    assert_eq!(choices[0].description, "None");
    assert_eq!(choices[2].description, "Machine");
}

#[test]
fn a_flags_property_keeps_its_bits_and_their_defaults() {
    let definitions = read_fgd(SAMPLE).expect("the sample parses");
    let base = definitions.find("Appearflags").expect("it is defined");

    let PropertyType::Flags(flags) = &base
        .property("spawnflags")
        .expect("it has spawnflags")
        .kind
    else {
        panic!("spawnflags should be a flags property");
    };
    assert_eq!(flags.len(), 3);
    assert_eq!(flags[0].value, 256);
    assert_eq!(flags[0].description, "Not in Easy");
    assert!(!flags[0].set_by_default);
    // The third is set by default.
    assert!(flags[2].set_by_default);
}

#[test]
fn a_class_inherits_its_base_classes_properties() {
    let definitions = read_fgd(SAMPLE).expect("the sample parses");
    let spawn = definitions
        .find("info_player_start")
        .expect("it is defined");

    // Its own property comes first, then the inherited ones.
    assert_eq!(spawn.properties[0].key, "angle");
    assert!(spawn.property("spawnflags").is_some());
    assert!(spawn.property("targetname").is_some());
}

#[test]
fn a_class_overrides_a_property_it_inherits() {
    let source = r#"
@baseclass = Base [ speed(integer) : "Base speed" : 10 ]
@SolidClass base(Base) = fast [ speed(integer) : "Fast speed" : 500 ]
"#;
    let definitions = read_fgd(source).expect("it parses");
    let fast = definitions.find("fast").expect("it is defined");

    let speed = fast.property("speed").expect("it has a speed");
    assert_eq!(speed.short_description, "Fast speed");
    assert_eq!(speed.default.as_deref(), Some("500"));
    // And it appears only once.
    assert_eq!(fast.properties.iter().filter(|p| p.key == "speed").count(), 1);
}

#[test]
fn appearance_is_inherited_where_a_class_gives_none() {
    let source = r#"
@baseclass size(-4 -4 -4, 4 4 4) color(1 2 3) = Look []
@PointClass base(Look) = thing : "A thing" []
"#;
    let definitions = read_fgd(source).expect("it parses");
    let thing = definitions.find("thing").expect("it is defined");

    assert_eq!(thing.color, Some([1, 2, 3]));
    assert_eq!(thing.bounds.expect("inherited").max, Vec3d::new(4.0, 4.0, 4.0));
}

#[test]
fn a_model_expression_resolves_to_a_path() {
    let definitions = read_fgd(SAMPLE).expect("the sample parses");
    let light = definitions.find("light").expect("it is defined");
    assert_eq!(
        light.model.as_ref().unwrap().default_model_specification().unwrap().path,
        std::path::PathBuf::from("progs/light.mdl")
    );

    // The bare quoted form too.
    let source = r#"@PointClass model("progs/x.mdl") = x : "X" []"#;
    let definitions = read_fgd(source).expect("it parses");
    assert_eq!(
        definitions.find("x").unwrap().model.as_ref().unwrap()
            .default_model_specification().unwrap().path,
        std::path::PathBuf::from("progs/x.mdl")
    );
}

#[test]
fn a_model_expression_can_depend_on_the_entitys_properties() {
    let source =
        "@PointClass model({{ spawnflags == 2 -> \"a.mdl\", \"b.mdl\" }}) = x : \"X\" []";
    let definitions = read_fgd(source).expect("it parses");
    let model = definitions.find("x").unwrap().model.clone().unwrap();

    // With nothing set, the fallback case.
    assert_eq!(
        model.default_model_specification().unwrap().path,
        std::path::PathBuf::from("b.mdl")
    );

    // With the flag, the first case.
    let vars: expression_lang::VariableTable =
        [("spawnflags", expression_lang::Value::integer(2))].into_iter().collect();
    assert_eq!(
        model.model_specification(&vars).unwrap().path,
        std::path::PathBuf::from("a.mdl")
    );
}

#[test]
fn an_unparseable_model_expression_is_reported() {
    let source = r#"@PointClass model(@@@) = x : "X" []"#;
    let definitions = read_fgd(source).expect("it still parses");

    assert_eq!(definitions.find("x").expect("defined").model, None);
    assert!(
        definitions.warnings.iter().any(|w| w.contains("model expression")),
        "{:?}",
        definitions.warnings
    );
}

#[test]
fn the_legacy_static_model_syntax_is_read_as_a_fallback() {
    // `parseLegacyStaticModelDefinition`: a comma-separated list of cases, each guarded
    // by a property test, tried last-first.
    let source = r#"@PointClass model(":maps/b_shell0.bsp", ":maps/b_shell1.bsp" spawnflags = 1) = x : "X" []"#;
    let definitions = read_fgd(source).expect("it parses");
    let model = definitions.find("x").unwrap().model.clone().unwrap();

    assert_eq!(
        model.default_model_specification().unwrap().path,
        std::path::PathBuf::from("maps/b_shell0.bsp")
    );

    let vars: expression_lang::VariableTable =
        [("spawnflags", expression_lang::Value::integer(1))].into_iter().collect();
    assert_eq!(
        model.model_specification(&vars).unwrap().path,
        std::path::PathBuf::from("maps/b_shell1.bsp")
    );

    // And it is called out as deprecated.
    assert!(
        definitions.warnings.iter().any(|w| w.contains("legacy model syntax")),
        "{:?}",
        definitions.warnings
    );
}

#[test]
fn the_legacy_dynamic_model_syntax_reads_path_skin_and_frame_from_properties() {
    // `parseLegacyDynamicModelDefinition`.
    let source =
        r#"@PointClass model(pathKey = "model" skinKey = "skin" frameKey = "frame") = x : "X" []"#;
    let definitions = read_fgd(source).expect("it parses");
    let model = definitions.find("x").unwrap().model.clone().unwrap();

    let just_path: expression_lang::VariableTable =
        [("model", expression_lang::Value::string("maps/b_shell1.bsp"))].into_iter().collect();
    let spec = model.model_specification(&just_path).unwrap();
    assert_eq!(spec.path, std::path::PathBuf::from("maps/b_shell1.bsp"));
    assert_eq!((spec.skin_index, spec.frame_index), (0, 0));

    let all: expression_lang::VariableTable = [
        ("model", expression_lang::Value::string("maps/b_shell1.bsp")),
        ("skin", expression_lang::Value::integer(1)),
        ("frame", expression_lang::Value::integer(2)),
    ]
    .into_iter()
    .collect();
    let spec = model.model_specification(&all).unwrap();
    assert_eq!(spec.path, std::path::PathBuf::from("maps/b_shell1.bsp"));
    assert_eq!((spec.skin_index, spec.frame_index), (1, 2));
}

#[test]
fn a_legacy_model_case_carries_its_skin_and_frame() {
    // `parseLegacyModelWithParseError`: two cases, the second with a skin and a frame.
    let source = r#"@PointClass model(":progs/polyp.mdl" 0 153, ":progs/polyp.mdl" startonground = "1") = monster_polyp : "Polyp" []"#;
    let definitions = read_fgd(source).expect("it parses");
    let model = definitions.find("monster_polyp").unwrap().model.clone().unwrap();

    // Nothing set: the static case with the skin and frame.
    let spec = model.default_model_specification().unwrap();
    assert_eq!(spec.path, std::path::PathBuf::from("progs/polyp.mdl"));
    assert_eq!((spec.skin_index, spec.frame_index), (0, 153));

    // The flag set: the guarded case, which names only a path.
    let vars: expression_lang::VariableTable =
        [("startonground", expression_lang::Value::string("1"))].into_iter().collect();
    let spec = model.model_specification(&vars).unwrap();
    assert_eq!(spec.path, std::path::PathBuf::from("progs/polyp.mdl"));
    assert_eq!(spec.frame_index, 0);
}

#[test]
fn a_model_maps_scale_is_read_from_the_entitys_properties() {
    let source =
        r#"@PointClass model({ "path": "x.mdl", "scale": scale }) = x : "X" []"#;
    let definitions = read_fgd(source).expect("it parses");
    let model = definitions.find("x").unwrap().model.clone().unwrap();

    // No `scale` property: the `1, 1, 1` fallback.
    assert_eq!(model.resolve_scale(&[], None), Vec3d::new(1.0, 1.0, 1.0));

    // A `scale` property: a number is uniform, `"x y z"` is per-axis.
    assert_eq!(
        model.resolve_scale(&[("scale".to_string(), "2".to_string())], None),
        Vec3d::new(2.0, 2.0, 2.0)
    );
    assert_eq!(
        model.resolve_scale(&[("scale".to_string(), "1 2 3".to_string())], None),
        Vec3d::new(1.0, 2.0, 3.0)
    );
}

#[test]
fn a_missing_base_class_is_reported_rather_than_swallowed() {
    let source = r#"@PointClass base(NotDefined) = x : "X" []"#;
    let definitions = read_fgd(source).expect("it still parses");

    assert!(definitions.find("x").is_some());
    assert!(
        definitions
            .warnings
            .iter()
            .any(|w| w.contains("NotDefined")),
        "{:?}",
        definitions.warnings
    );
}

#[test]
fn a_cycle_in_the_base_classes_does_not_hang() {
    let source = r#"
@baseclass base(B) = A [ a(integer) : "A" ]
@baseclass base(A) = B [ b(integer) : "B" ]
@PointClass base(A) = thing : "T" []
"#;
    let definitions = read_fgd(source).expect("it parses");
    let thing = definitions.find("thing").expect("it is defined");

    // Both properties are picked up, and the walk terminates.
    assert!(thing.property("a").is_some());
    assert!(thing.property("b").is_some());
}

#[test]
fn a_class_with_no_property_list_is_allowed() {
    let source = r#"@PointClass = marker : "A marker""#;
    let definitions = read_fgd(source).expect("it parses");

    let marker = definitions.find("marker").expect("it is defined");
    assert!(marker.properties.is_empty());
    assert_eq!(marker.description, "A marker");
}

#[test]
fn file_level_directives_are_skipped_with_a_warning() {
    let source = r#"
@include "base.fgd"
@mapsize(-16384, 16384)
@PointClass = thing : "T" []
"#;
    let definitions = read_fgd(source).expect("it parses");

    assert!(definitions.find("thing").is_some());
    assert_eq!(definitions.warnings.len(), 2, "{:?}", definitions.warnings);
}

#[test]
fn an_include_pulls_in_another_files_classes() {
    use map_model::fgd::read_fgd_file;

    let dir = std::env::temp_dir().join(format!("heron-fgd-include-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("base.fgd"),
        "@BaseClass = Appear [ spawnflags(flags) = [ 256 : \"Not easy\" : 0 ] ]\n\
         @PointClass base(Appear) = shared : \"S\" []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("game.fgd"),
        "@include \"base.fgd\"\n@PointClass base(Appear) size(-8 -8 -8, 8 8 8) = thing : \"T\" []\n",
    )
    .unwrap();

    let definitions = read_fgd_file(dir.join("game.fgd")).expect("it reads");

    // Both files' classes are here, and the base class from the include resolved.
    assert!(definitions.find("shared").is_some());
    let thing = definitions.find("thing").expect("the including file's class");
    assert!(
        thing.property("spawnflags").is_some(),
        "inherited the base class from the included file"
    );
    assert!(
        definitions.warnings.is_empty(),
        "{:?}",
        definitions.warnings
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_include_cycle_is_broken_rather_than_looping() {
    use map_model::fgd::read_fgd_file;

    let dir = std::env::temp_dir().join(format!("heron-fgd-cycle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("a.fgd"),
        "@include \"b.fgd\"\n@PointClass = a : \"A\" []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("b.fgd"),
        "@include \"a.fgd\"\n@PointClass = b : \"B\" []\n",
    )
    .unwrap();

    let definitions = read_fgd_file(dir.join("a.fgd")).expect("it reads");
    assert!(definitions.find("a").is_some());
    assert!(definitions.find("b").is_some());
    assert!(
        definitions.warnings.iter().any(|w| w.contains("includes itself")),
        "{:?}",
        definitions.warnings
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_include_that_escapes_the_directory_is_refused() {
    use map_model::fgd::read_fgd_file;

    let dir = std::env::temp_dir().join(format!("heron-fgd-escape-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("game.fgd"),
        "@include \"../secrets.fgd\"\n@PointClass = thing : \"T\" []\n",
    )
    .unwrap();

    let definitions = read_fgd_file(dir.join("game.fgd")).expect("it reads");
    assert!(definitions.find("thing").is_some());
    assert!(
        definitions.warnings.iter().any(|w| w.contains("outside") || w.contains("under")),
        "{:?}",
        definitions.warnings
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn spawnflags_are_merged_across_inheritance_rather_than_shadowed() {
    let source = r#"
@baseclass = Appearflags
[
    spawnflags(flags) =
    [
        256 : "Not in Easy" : 0
        512 : "Not in Normal" : 0
        1024 : "Not in Hard" : 1
    ]
]
@PointClass base(Appearflags) = monster_x : "X"
[
    spawnflags(flags) =
    [
        1 : "Ambush" : 0
        2 : "Sight" : 1
    ]
]
"#;
    let definitions = read_fgd(source).expect("it parses");
    let monster = definitions.find("monster_x").expect("it is defined");
    let flags = match &monster.property("spawnflags").unwrap().kind {
        PropertyType::Flags(flags) => flags,
        other => panic!("spawnflags is {other:?}"),
    };

    let bits: Vec<i64> = flags.iter().map(|f| f.value).collect();
    assert_eq!(bits, vec![1, 2, 256, 512, 1024], "the class's own bits and the base's");

    // The class's own default (bit 2) and the base's (bit 1024) both carry over.
    let by_value = |v: i64| flags.iter().find(|f| f.value == v).unwrap();
    assert!(by_value(2).set_by_default);
    assert!(by_value(1024).set_by_default);
    assert!(!by_value(1).set_by_default);
    assert!(!by_value(256).set_by_default);
}

#[test]
fn an_unknown_property_type_is_kept_as_free_text() {
    let source = r#"@PointClass = x : "X" [ thing(some_new_type) : "Thing" ]"#;
    let definitions = read_fgd(source).expect("it parses");

    let property = definitions
        .find("x")
        .expect("defined")
        .property("thing")
        .expect("it has the property");
    assert_eq!(
        property.kind,
        PropertyType::Other("some_new_type".to_string())
    );
    // It still shows up, so the entity's data is not lost.
    assert_eq!(property.short_description, "Thing");
}

#[test]
fn an_empty_field_between_colons_is_allowed() {
    let source = r#"@PointClass = x : "X" [ a(integer) : "A" : : "Help" ]"#;
    let definitions = read_fgd(source).expect("it parses");

    let property = definitions
        .find("x")
        .expect("defined")
        .property("a")
        .expect("it has the property");
    assert_eq!(property.short_description, "A");
    assert_eq!(property.default, None);
    assert_eq!(property.long_description.as_deref(), Some("Help"));
}

#[test]
fn a_size_with_one_triple_is_centred_on_the_origin() {
    let source = r#"@PointClass size(32 32 32) = x : "X" []"#;
    let definitions = read_fgd(source).expect("it parses");

    let bounds = definitions
        .find("x")
        .expect("defined")
        .bounds
        .expect("it has a size");
    assert_eq!(bounds.min, Vec3d::new(-16.0, -16.0, -16.0));
    assert_eq!(bounds.max, Vec3d::new(16.0, 16.0, 16.0));
}

#[test]
fn a_broken_file_reports_where() {
    let error = read_fgd("@PointClass = x : \"X\" [ a(integer)").expect_err("unterminated");
    assert!(error.to_string().contains("line"), "{error}");

    let error = read_fgd("@NotAClass = x").expect_err("unknown class type");
    assert!(error.to_string().contains("NotAClass"), "{error}");
}
