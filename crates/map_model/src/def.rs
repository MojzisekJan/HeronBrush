//! Reading the older `.def` entity definitions.
//!
//! Reads `.def` entity definitions from `/*QUAKED ... */` comment blocks, as used by Quake-lineage
//! game configurations.
//!
//! The result is [`crate::fgd::EntityDefinitions`], the same as an FGD gives, since what a
//! definition *is* does not depend on which file it was written in. What a `.def` head
//! line cannot say -- property types beyond the spawnflags -- is simply absent rather than
//! made up.
//!
//! ```text
//! /*QUAKED item_health (.3 .3 1) (-16 -16 -16) (16 16 16) ROTTEN MEGAHEALTH
//! Health box. Normally gives 25 points.
//! */
//!
//! /*QUAKED weapon_axe (0 .5 .8) (-16 -16 0) (16 16 32)
//! { model(":progs/g_axe.mdl"); }
//! Axe.
//! */
//! ```
//!
//! The head line is the class name, the colour the editor draws it in, then either the
//! box a point entity is drawn as or a `?` for one made of brushes, then the names of its
//! spawnflags in bit order. An optional `{ … }` block after it carries `model(…)` and
//! `base(…)` directives. Everything left is what the editor shows about the class.

use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

use crate::fgd::{
    DefinitionKind, EntityDefinition, EntityDefinitions, Flag, PropertyDefinition, PropertyType,
};

/// Why a `.def` file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefError {
    /// What went wrong.
    pub message: String,
    /// The 1-based line it happened on.
    pub line: usize,
}

impl std::fmt::Display for DefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for DefError {}

/// The key the spawnflags are written under, which is the same in every Quake-lineage
/// game and the only property a `.def` describes.
const SPAWNFLAGS: &str = "spawnflags";

/// Reads a `.def` file.
///
/// Anything outside a `/*QUAKED ... */` block is skipped: a `.def` is a C header with the
/// definitions written into its comments, and the code around them is not ours to read. A
/// block whose head line makes no sense is skipped with a warning rather than refused, so
/// one bad class does not cost the file.
pub fn read_def(source: &str) -> Result<EntityDefinitions, DefError> {
    let mut definitions = Vec::new();
    let mut warnings = Vec::new();

    let mut line_number = 1;
    let mut rest = source;
    while let Some(at) = rest.find("/*QUAKED") {
        line_number += rest[..at].matches('\n').count();
        let block_starts = at + "/*QUAKED".len();
        let Some(length) = rest[block_starts..].find("*/") else {
            return Err(DefError {
                message: "a QUAKED block is never closed".to_string(),
                line: line_number,
            });
        };
        let block = &rest[block_starts..block_starts + length];

        match read_block(block, line_number, &mut warnings) {
            Some(definition) => definitions.push(definition),
            None => warnings.push(format!(
                "line {line_number}: this QUAKED block names no class, so it is skipped"
            )),
        }

        line_number += block.matches('\n').count();
        rest = &rest[block_starts + length + 2..];
    }

    // A `.def` `{ base(Monster) }` directive names a base class exactly as an FGD's
    // `base(...)` does, so the same inheritance pass applies.
    Ok(crate::fgd::resolve_inheritance(definitions, warnings))
}

/// Reads one block: its head line, an optional `{ ... }` directives block, then the text
/// under it. `line` is where the block starts, for the warnings.
fn read_block(
    block: &str,
    line: usize,
    warnings: &mut Vec<String>,
) -> Option<EntityDefinition> {
    let (head, body) = block.split_once('\n').unwrap_or((block, ""));

    // An optional `{ ... }` block between the header and description contains directives such as
    // `model(...)`.
    let (directives, rest) = split_directives_block(body);
    let description = rest
        .lines()
        .map(str::trim_end)
        .collect::<Vec<&str>>()
        .join("\n")
        .trim()
        .to_string();

    let mut head = Head::new(head);
    let name = head.word()?;
    if name.is_empty() {
        return None;
    }

    let color = head.parenthesised().and_then(|values| {
        let numbers = numbers(&values, 3)?;
        // A `.def` writes its colours as fractions, as `Color::parse` reads them.
        Some([
            (numbers[0] * 255.0).round().clamp(0.0, 255.0) as u8,
            (numbers[1] * 255.0).round().clamp(0.0, 255.0) as u8,
            (numbers[2] * 255.0).round().clamp(0.0, 255.0) as u8,
        ])
    });

    // Either two boxes' worth of numbers, or a `?` for a class made of brushes.
    let bounds = match head.parenthesised() {
        Some(min) => {
            let max = head.parenthesised()?;
            let (min, max) = (numbers(&min, 3)?, numbers(&max, 3)?);
            Some(Bbox3d::new(
                Vec3d::new(min[0], min[1], min[2]),
                Vec3d::new(max[0], max[1], max[2]),
            ))
        }
        None => None,
    };

    // What is left on the line is the spawnflags, in bit order. A `.def` gives them names
    // and nothing else, so each becomes a flag on the one property every class has.
    let flags: Vec<Flag> = head
        .rest()
        .split_whitespace()
        .filter(|word| *word != "?")
        .enumerate()
        .map(|(bit, description)| Flag {
            value: 1 << bit,
            description: description.to_string(),
            set_by_default: false,
        })
        .collect();

    let mut properties = if flags.is_empty() {
        Vec::new()
    } else {
        vec![PropertyDefinition {
            key: SPAWNFLAGS.to_string(),
            kind: PropertyType::Flags(flags),
            short_description: "Spawnflags".to_string(),
            default: None,
            long_description: None,
        }]
    };

    let mut model = None;
    let mut base_classes = Vec::new();
    for (keyword, arguments) in directives {
        match keyword.to_ascii_lowercase().as_str() {
            "model" | "studio" | "iconsprite" | "sprite" => {
                if arguments.trim().is_empty() {
                    continue;
                }
                match crate::model_definition::parse_model_argument(&arguments, line) {
                    Ok(parsed) => {
                        if let Some(note) = parsed.deprecation {
                            warnings.push(format!("line {line}: `{name}`: {note}"));
                        }
                        model = Some(parsed.definition);
                    }
                    Err(error) => warnings.push(format!(
                        "line {line}: `{name}`: `{keyword}(...)` did not parse as a model \
                         expression: {error}"
                    )),
                }
            }
            "base" => base_classes.extend(
                arguments
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
            ),
            "choice" => {
                if let Some((key, choices)) = parse_choice_directive(&arguments) {
                    properties.push(PropertyDefinition {
                        key,
                        kind: PropertyType::Choices(choices),
                        short_description: String::new(),
                        default: None,
                        long_description: None,
                    });
                }
            }
            // `default` describes a property this reader does not model.
            _ => {}
        }
    }

    Some(EntityDefinition {
        name: name.to_string(),
        kind: if bounds.is_some() {
            DefinitionKind::Point
        } else {
            DefinitionKind::Solid
        },
        description,
        base_classes,
        bounds,
        color,
        model,
        properties,
    })
}

/// Splits a `{ directive(args); … }` block off the front of a block body, returning the
/// directives as `(keyword, arguments)` pairs and the text left for the description. If
/// the body does not begin with `{`, there are no directives and all of it is the
/// description.
fn split_directives_block(body: &str) -> (Vec<(String, String)>, &str) {
    let trimmed = body.trim_start();
    if !trimmed.starts_with('{') {
        return (Vec::new(), body);
    }

    // Find the `}` that closes the block, counting nesting so a `model({ ... })` or a
    // `{{ ... }}` switch inside does not end it early.
    let mut depth = 0usize;
    let mut end = None;
    for (offset, c) in trimmed.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(offset);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(end) = end else {
        // Never closed: leave the body as the description rather than eat it.
        return (Vec::new(), body);
    };

    let inside = &trimmed[1..end];
    let rest = &trimmed[end + 1..];

    let directives = inside
        .split(';')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            let open = entry.find('(')?;
            let close = entry.rfind(')')?;
            if close < open {
                return None;
            }
            // The keyword is the first word; anything between it and the `(` -- a
            // `choice "key"` names its property there -- rides along with the arguments.
            let head = entry[..open].trim();
            let (keyword, between) =
                head.split_once(char::is_whitespace).unwrap_or((head, ""));
            let arguments =
                format!("{between} {}", &entry[open + 1..close]).trim().to_string();
            Some((keyword.to_string(), arguments))
        })
        .collect();

    (directives, rest)
}

/// Reads `.def` choices as a quoted property key followed by `( value, "name" )` pairs.
fn parse_choice_directive(arguments: &str) -> Option<(String, Vec<crate::fgd::Choice>)> {
    let arguments = arguments.trim();
    let key_end = arguments.strip_prefix('"')?.find('"')? + 1;
    let key = arguments[1..key_end].to_string();
    let mut rest = arguments[key_end + 1..].trim_start();

    let mut choices = Vec::new();
    while let Some(after_open) = rest.strip_prefix('(') {
        let close = after_open.find(')')?;
        let (value, name) = after_open[..close].split_once(',')?;
        let name = name.trim().trim_matches('"').to_string();
        choices.push(crate::fgd::Choice { value: value.trim().to_string(), description: name });
        rest = after_open[close + 1..].trim_start();
    }

    (!choices.is_empty()).then_some((key, choices))
}

/// The head line, read a piece at a time.
struct Head<'a> {
    rest: &'a str,
}

impl<'a> Head<'a> {
    fn new(line: &'a str) -> Self {
        Head { rest: line.trim() }
    }

    /// The next word, or `None` at the end of the line.
    fn word(&mut self) -> Option<&'a str> {
        let rest = self.rest.trim_start();
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        if end == 0 {
            return None;
        }
        let (word, left) = rest.split_at(end);
        self.rest = left;
        Some(word)
    }

    /// What is inside the next `(...)`, if the line goes on with one.
    fn parenthesised(&mut self) -> Option<String> {
        let rest = self.rest.trim_start();
        let inside = rest.strip_prefix('(')?;
        let end = inside.find(')')?;
        self.rest = &inside[end + 1..];
        Some(inside[..end].to_string())
    }

    /// Everything not read yet.
    fn rest(&self) -> &'a str {
        self.rest.trim()
    }
}

/// The numbers in a `(...)`, if there are the right number of them.
fn numbers(inside: &str, wanted: usize) -> Option<Vec<f64>> {
    let read: Vec<f64> = inside
        .split_whitespace()
        .filter_map(|word| word.parse::<f64>().ok())
        .collect();
    (read.len() == wanted).then_some(read)
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUAKE: &str = r#"
// A header, as a .def file is one.
/*QUAKED item_health (.3 .3 1) (-16 -16 -16) (16 16 16) ROTTEN MEGAHEALTH
Health box. Normally gives 25 points.
*/

/*QUAKED func_door (0 .5 .8) ? START_OPEN DOOR_DONT_LINK GOLD_KEY SILVER_KEY TOGGLE
A door. Brushes make its shape.
*/
"#;

    #[test]
    fn a_point_class_keeps_its_box_its_colour_and_its_flags() {
        let read = read_def(QUAKE).expect("it reads");
        let health = read.find("item_health").expect("it is there");

        assert_eq!(health.kind, DefinitionKind::Point);
        assert_eq!(
            health.bounds,
            Some(Bbox3d::new(Vec3d::fill(-16.0), Vec3d::fill(16.0)))
        );
        assert_eq!(health.color, Some([77, 77, 255]), "fractions become bytes");
        assert!(health.description.starts_with("Health box"));

        let flags = health.property("spawnflags").expect("it has spawnflags");
        let PropertyType::Flags(flags) = &flags.kind else {
            panic!("spawnflags is not a set of flags");
        };
        assert_eq!(flags.len(), 2);
        assert_eq!(flags[0].value, 1);
        assert_eq!(flags[0].description, "ROTTEN");
        assert_eq!(flags[1].value, 2);
        assert_eq!(flags[1].description, "MEGAHEALTH");
    }

    #[test]
    fn a_question_mark_means_the_class_is_made_of_brushes() {
        let read = read_def(QUAKE).expect("it reads");
        let door = read.find("func_door").expect("it is there");

        assert_eq!(door.kind, DefinitionKind::Solid);
        assert_eq!(door.bounds, None);
        let flags = door.property("spawnflags").expect("it has spawnflags");
        let PropertyType::Flags(flags) = &flags.kind else {
            panic!("spawnflags is not a set of flags");
        };
        assert_eq!(flags.len(), 5, "the `?` is not one of them");
        assert_eq!(flags[4].value, 16);
    }

    #[test]
    fn both_classes_are_placeable_and_in_the_order_the_file_has_them() {
        let read = read_def(QUAKE).expect("it reads");
        let names: Vec<&str> = read.placeable().map(|class| class.name.as_str()).collect();
        assert_eq!(names, vec!["item_health", "func_door"]);
    }

    #[test]
    fn a_block_that_is_never_closed_is_an_error() {
        let outcome = read_def("/*QUAKED thing (1 1 1) ?\nno end in sight\n");
        assert!(outcome.is_err());
    }

    #[test]
    fn a_block_with_no_class_name_is_skipped_with_a_warning() {
        let read = read_def("/*QUAKED\n*/\n/*QUAKED light (1 1 0) (-8 -8 -8) (8 8 8)\n*/")
            .expect("one bad block does not cost the file");
        assert_eq!(read.placeable().count(), 1);
        assert_eq!(read.warnings.len(), 1);
    }

    #[test]
    fn anything_outside_a_quaked_block_is_not_read() {
        let read = read_def("int x = 3;\n/* a plain comment */\nvoid f(void) {}\n")
            .expect("it reads");
        assert_eq!(read.placeable().count(), 0);
    }

    /// The template the shipped `.def` files and `tst_DefParser` use: a `{ ... }` block
    /// between the head line and the description.
    fn one_class_with_model(model: &str) -> crate::fgd::EntityDefinition {
        let source = format!(
            "/*QUAKED monster_zombie (1 0 0) (-16 -16 -24) (16 16 32) Crucified ambush\n\
             {{\n  model({model});\n}}\n\
             A shambling corpse.\n*/"
        );
        read_def(&source)
            .expect("it reads")
            .find("monster_zombie")
            .cloned()
            .expect("it is there")
    }

    #[test]
    fn a_model_directive_in_the_body_block_is_read() {
        let class = one_class_with_model(r#"":progs/zombie.mdl""#);
        let spec = class
            .model
            .as_ref()
            .expect("it has a model")
            .default_model_specification()
            .unwrap();
        assert_eq!(spec.path, std::path::PathBuf::from("progs/zombie.mdl"));
        // The block is not left in the description.
        assert!(class.description.starts_with("A shambling corpse"));
        assert!(!class.description.contains("model("));
    }

    #[test]
    fn a_body_block_model_may_be_an_el_switch() {
        // `parseELModelDefinition` from tst_DefParser.
        let class = one_class_with_model(
            r#"{{ spawnflags == 1 -> ":maps/b_shell1.bsp", ":maps/b_shell0.bsp" }}"#,
        );
        let model = class.model.as_ref().expect("it has a model");
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
    }

    #[test]
    fn a_body_block_model_may_use_the_legacy_dynamic_syntax() {
        // `parseLegacyDynamicModelDefinition` from tst_DefParser.
        let class =
            one_class_with_model(r#"pathKey = "model" skinKey = "skin" frameKey = "frame""#);
        let model = class.model.as_ref().expect("it has a model");
        let vars: expression_lang::VariableTable = [
            ("model", expression_lang::Value::string("maps/b_shell1.bsp")),
            ("skin", expression_lang::Value::integer(1)),
            ("frame", expression_lang::Value::integer(2)),
        ]
        .into_iter()
        .collect();
        let spec = model.model_specification(&vars).unwrap();
        assert_eq!(spec.path, std::path::PathBuf::from("maps/b_shell1.bsp"));
        assert_eq!((spec.skin_index, spec.frame_index), (1, 2));
    }

    #[test]
    fn a_class_with_no_body_block_keeps_all_its_text_as_description() {
        let read = read_def(QUAKE).expect("it reads");
        let health = read.find("item_health").expect("it is there");
        assert_eq!(health.model, None);
        assert!(health.description.starts_with("Health box"));
    }

    #[test]
    fn a_choice_directive_becomes_a_choices_property() {
        let source = "\
/*QUAKED worldspawn (0 0 0) ?
{
choice \"worldtype\" (
(0, \"medieval\")
(1, \"metal\")
(2, \"base\")
);
}
The world.
*/";
        let read = read_def(source).expect("it reads");
        let world = read.find("worldspawn").expect("it is there");
        let prop = world.property("worldtype").expect("the choice became a property");
        match &prop.kind {
            crate::fgd::PropertyType::Choices(choices) => {
                let pairs: Vec<(&str, &str)> = choices
                    .iter()
                    .map(|c| (c.value.as_str(), c.description.as_str()))
                    .collect();
                assert_eq!(
                    pairs,
                    vec![("0", "medieval"), ("1", "metal"), ("2", "base")]
                );
            }
            other => panic!("worldtype is {other:?}"),
        }
        assert!(world.description.starts_with("The world"));
    }

    #[test]
    fn a_base_directive_pulls_in_the_base_classs_model() {
        let source = "\
/*QUAKED Monster (1 0 0) ?
{
model(\":progs/soldier.mdl\");
}
The base for the ordinary soldiers.
*/
/*QUAKED monster_army (1 0 0) (-16 -16 -24) (16 16 40)
{
base(Monster);
}
A regular soldier.
*/";
        let read = read_def(source).expect("it reads");
        let army = read.find("monster_army").expect("it is there");
        assert_eq!(
            army.model.as_ref().and_then(|m| m.default_model_specification().ok()).map(|s| s.path),
            Some(std::path::PathBuf::from("progs/soldier.mdl")),
            "inherited the model from the base"
        );
    }
}
