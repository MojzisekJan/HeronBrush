//! Tests for model definition behavior.
#![allow(clippy::type_complexity)]

use expression_lang::{parse_expression, NullVariableStore, ParseMode, Value, VariableTable};
use geometry_math::vec::Vec3d;
use map_model::model_definition::{parse_model_definition, ModelDefinition, ModelSpecification};

fn make(expression: &str) -> ModelDefinition {
    parse_model_definition(expression).unwrap_or_else(|e| panic!("{expression:?}: {e}"))
}

fn spec(path: &str, skin: usize, frame: usize) -> ModelSpecification {
    ModelSpecification {
        path: path.into(),
        skin_index: skin,
        frame_index: frame,
    }
}

fn table(pairs: &[(&str, Value)]) -> VariableTable {
    pairs.iter().map(|(k, v)| (*k, v.clone())).collect()
}

#[test]
fn append() {
    let mut d1 = make(r#""maps/b_shell0.bsp""#);
    assert_eq!(
        d1.model_specification(&NullVariableStore).unwrap(),
        spec("maps/b_shell0.bsp", 0, 0)
    );

    d1.append(make(r#""maps/b_shell1.bsp""#));
    assert_eq!(
        d1.model_specification(&NullVariableStore).unwrap(),
        spec("maps/b_shell0.bsp", 0, 0)
    );
}

#[test]
fn model_specification() {
    let cases: Vec<(&str, Vec<(&str, Value)>, ModelSpecification)> = vec![
        (
            r#""maps/b_shell0.bsp""#,
            vec![],
            spec("maps/b_shell0.bsp", 0, 0),
        ),
        (
            r#"{ path: "maps/b_shell0.bsp", skin: 1, frame: 2 }"#,
            vec![],
            spec("maps/b_shell0.bsp", 1, 2),
        ),
        (
            "{{\n spawnflags == 1 -> \"maps/b_shell0.bsp\",\n \"maps/b_shell1.bsp\"\n}}",
            vec![],
            spec("maps/b_shell1.bsp", 0, 0),
        ),
        (
            "{{\n spawnflags == 1 -> \"maps/b_shell0.bsp\",\n \"maps/b_shell1.bsp\"\n}}",
            vec![("spawnflags", Value::integer(1))],
            spec("maps/b_shell0.bsp", 0, 0),
        ),
        (
            "{path: model, skin: skin, frame: frame}",
            vec![
                ("model", Value::string("maps/b_shell0.bsp")),
                ("skin", Value::integer(1)),
                ("frame", Value::integer(2)),
            ],
            spec("maps/b_shell0.bsp", 1, 2),
        ),
    ];

    for (expression, variables, expected) in cases {
        let definition = make(expression);
        assert_eq!(
            definition.model_specification(&table(&variables)).unwrap(),
            expected,
            "{expression:?}"
        );
    }
}

#[test]
fn default_model_specification() {
    let cases: Vec<(&str, ModelSpecification)> = vec![
        (r#""maps/b_shell0.bsp""#, spec("maps/b_shell0.bsp", 0, 0)),
        (
            r#"{ path: "maps/b_shell0.bsp", skin: 1, frame: 2 }"#,
            spec("maps/b_shell0.bsp", 1, 2),
        ),
        (
            "{{\n spawnflags == 1 -> \"maps/b_shell0.bsp\",\n \"maps/b_shell1.bsp\"\n}}",
            spec("maps/b_shell1.bsp", 0, 0),
        ),
        (
            "{path: model, skin: skin, frame: frame}",
            ModelSpecification::default(),
        ),
        (r#"{ path: 123, skin: 1, frame: 2 }"#, spec("", 1, 2)),
        (
            "{ \n path: \"maps/b_shell0.bsp\", \n scale : radius * 64 }",
            spec("maps/b_shell0.bsp", 0, 0),
        ),
    ];

    for (expression, expected) in cases {
        let definition = make(expression);
        assert_eq!(
            definition.default_model_specification().unwrap(),
            expected,
            "{expression:?}"
        );
    }
}

#[test]
fn scale() {
    let cases: Vec<(&str, Option<&str>, Vec3d)> = vec![
        (r#""maps/b_shell0.bsp""#, None, Vec3d::new(1.0, 1.0, 1.0)),
        (
            r#""maps/b_shell0.bsp""#,
            Some("2"),
            Vec3d::new(2.0, 2.0, 2.0),
        ),
        (
            r#""maps/b_shell0.bsp""#,
            Some("modelscale"),
            Vec3d::new(4.0, 4.0, 4.0),
        ),
        (
            r#"{ path: "maps/b_shell0.bsp", skin: 1, frame: 2, scale: 1.5 }"#,
            None,
            Vec3d::new(1.5, 1.5, 1.5),
        ),
        (
            r#"{ path: "maps/b_shell0.bsp", skin: 1, frame: 2, scale: 1.5 }"#,
            Some("modelscale"),
            Vec3d::new(1.5, 1.5, 1.5),
        ),
        (
            r#"{ path: "maps/b_shell0.bsp", skin: 1, frame: 2, scale: '1.5' }"#,
            None,
            Vec3d::new(1.5, 1.5, 1.5),
        ),
        (
            r#"{ path: "maps/b_shell0.bsp", skin: 1, frame: 2, scale: '1 2 3' }"#,
            None,
            Vec3d::new(1.0, 2.0, 3.0),
        ),
        (
            r#"{ path: "maps/b_shell0.bsp", skin: 1, frame: 2, scale: modelscale }"#,
            None,
            Vec3d::new(4.0, 4.0, 4.0),
        ),
        (
            r#"{ path: "maps/b_shell0.bsp", skin: 1, frame: 2, scale: [modelscale, modelscale_vec] }"#,
            None,
            Vec3d::new(4.0, 4.0, 4.0),
        ),
        (
            r#"{ path: "maps/b_shell0.bsp", skin: 1, frame: 2, scale: [modelscale_vec, modelscale] }"#,
            None,
            Vec3d::new(5.0, 6.0, 7.0),
        ),
    ];

    let variables = table(&[
        ("modelscale", Value::integer(4)),
        ("modelscale_vec", Value::string("5, 6, 7")),
    ]);

    for (expression, default_scale, expected) in cases {
        let definition = make(expression);
        let default_expression =
            default_scale.map(|s| parse_expression(ParseMode::Strict, s).unwrap());
        let got = definition
            .scale(&variables, default_expression.as_ref())
            .unwrap_or_else(|e| panic!("{expression:?}: {e}"));
        assert_eq!(got, expected, "{expression:?}");
    }
}
