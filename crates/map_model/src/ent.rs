//! Reading `.ent` entity definitions.
//!
//! Reads Radiant XML entity definitions. A `<classes>` root contains `<point>` or `<group>`
//! classes, typed properties and spawnflag elements. Top-level `<list>` elements declare reusable
//! choice types; class text supplies the description.
//!
//! The result is [`crate::fgd::EntityDefinitions`], as the FGD and `.def` readers give,
//! since what a definition *is* does not depend on which file it was written in.
//!
//! ```text
//! <?xml version="1.0"?>
//! <classes>
//!   <point name="_skybox" color="0.77 0.88 1.0" box="-4 -4 -4 4 4 4">
//!     -------- KEYS --------
//!     <real key="_scale" name="Scale" value="64">Scaling factor.</real>
//!   </point>
//! </classes>
//! ```
//!
//! Uses `quick-xml` to build a small tree of elements and text.

use quick_xml::events::Event;
use quick_xml::Reader;
use geometry_math::bbox::Bbox3d;
use geometry_math::vec::Vec3d;

use crate::fgd::{
    Choice, DefinitionKind, EntityDefinition, EntityDefinitions, Flag, PropertyDefinition,
    PropertyType,
};
use crate::model_definition::ModelDefinition;

/// Why a `.ent` file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntError {
    /// What went wrong.
    pub message: String,
    /// The 1-based line it happened on.
    pub line: usize,
}

impl std::fmt::Display for EntError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for EntError {}

/// The key the spawnflags are written under, `EntityPropertyKeys::Spawnflags`.
const SPAWNFLAGS: &str = "spawnflags";

/// One node of the parsed document.
#[derive(Debug)]
enum Node {
    Element(Element),
    Text(String),
}

/// An element with its attributes and children, which is all `EntParser` asks of
/// tinyxml2.
#[derive(Debug)]
struct Element {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<Node>,
    /// The 1-based line the start tag is on, for the warnings.
    line: usize,
}

impl Element {
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn has_attribute(&self, name: &str) -> bool {
        self.attribute(name).is_some()
    }

    /// `parseString`: the attribute's text, or nothing at all when it is missing.
    fn string(&self, name: &str) -> String {
        self.attribute(name).unwrap_or_default().to_string()
    }

    fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|child| match child {
            Node::Element(element) => Some(element),
            Node::Text(_) => None,
        })
    }

    fn first_element(&self, name: &str) -> Option<&Element> {
        self.elements().find(|element| element.name == name)
    }

    /// Joins the first and last text child to form the description, omitting intervening property
    /// elements.
    fn text(&self) -> String {
        let first = match self.children.first() {
            Some(Node::Text(text)) => text.as_str(),
            _ => "",
        };
        let last = if self.children.len() > 1 {
            match self.children.last() {
                Some(Node::Text(text)) => text.as_str(),
                _ => "",
            }
        } else {
            ""
        };
        format!("{first}{last}")
    }
}

/// Reads a `.ent` file.
///
/// Empty or whitespace-only files contain no definitions. Malformed XML is an error; invalid
/// classes or properties are skipped with warnings.
pub fn read_ent(source: &str) -> Result<EntityDefinitions, EntError> {
    let mut warnings = Vec::new();
    let Some(root) = parse_document(source)? else {
        return Ok(EntityDefinitions::new(Vec::new(), warnings));
    };

    let mut definitions: Vec<EntityDefinition> = Vec::new();
    let mut declarations: Vec<PropertyDefinition> = Vec::new();
    if let Some(classes) = root.first_element("classes") {
        for element in classes.elements() {
            match element.name.as_str() {
                "point" | "group" => {
                    let definition = read_class(element, &declarations, &mut warnings);
                    // `EntityDefinitionParser::parseDefinitions`: a class defined twice
                    // keeps the later one, with a word about it.
                    if let Some(at) = definitions.iter().position(|d| d.name == definition.name) {
                        warnings.push(format!(
                            "line {}: \"{}\" is defined more than once; the later one is kept",
                            element.line, definition.name
                        ));
                        definitions.remove(at);
                    }
                    definitions.push(definition);
                }
                "list" => {
                    if let Some(declaration) = read_list_declaration(element, &mut warnings) {
                        if declarations.iter().any(|d| d.key == declaration.key) {
                            warnings.push(format!(
                                "line {}: skipping duplicate entity property declaration \"{}\"",
                                element.line, declaration.key
                            ));
                        } else {
                            declarations.push(declaration);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(crate::fgd::resolve_inheritance(definitions, warnings))
}

/// Builds the element tree, or nothing for a document with no element in it.
fn parse_document(source: &str) -> Result<Option<Element>, EntError> {
    let mut reader = Reader::from_str(source);
    let line_of = |position: u64| source[..(position as usize).min(source.len())].matches('\n').count() + 1;

    // The document itself is a nameless element holding the top-level ones.
    let mut stack: Vec<Element> = vec![Element {
        name: String::new(),
        attributes: Vec::new(),
        children: Vec::new(),
        line: 1,
    }];

    loop {
        let event = reader.read_event().map_err(|error| EntError {
            message: error.to_string(),
            line: line_of(reader.error_position()),
        })?;
        match event {
            Event::Start(start) => {
                let element = element_from(&start, line_of(reader.buffer_position()))?;
                stack.push(element);
            }
            Event::Empty(start) => {
                let element = element_from(&start, line_of(reader.buffer_position()))?;
                push_child(&mut stack, Node::Element(element));
            }
            Event::End(_) => {
                let element = stack.pop().expect("the document element is never popped");
                if stack.is_empty() {
                    return Err(EntError {
                        message: "a closing tag with nothing open".to_string(),
                        line: line_of(reader.buffer_position()),
                    });
                }
                push_child(&mut stack, Node::Element(element));
            }
            Event::Text(text) => {
                let text = text.decode().map(|t| t.into_owned()).unwrap_or_default();
                push_text(&mut stack, &text);
            }
            Event::CData(data) => {
                let text = String::from_utf8_lossy(&data.into_inner()).into_owned();
                push_text(&mut stack, &text);
            }
            Event::GeneralRef(reference) => {
                // `&gt;` and the like arrive on their own; put back what they stand for.
                let name = reference.decode().map(|t| t.into_owned()).unwrap_or_default();
                let resolved = resolve_reference(&name).unwrap_or_else(|| format!("&{name};"));
                push_text(&mut stack, &resolved);
            }
            Event::Comment(_) | Event::Decl(_) | Event::PI(_) | Event::DocType(_) => {}
            Event::Eof => break,
        }
    }

    if stack.len() > 1 {
        let open = stack.last().expect("more than one");
        return Err(EntError {
            message: format!("<{}> is never closed", open.name),
            line: open.line,
        });
    }
    let document = stack.pop().expect("the document element");
    let has_elements = document.elements().next().is_some();
    Ok(has_elements.then_some(document))
}

/// An element from its start tag: the name and the attributes, unescaped.
fn element_from(start: &quick_xml::events::BytesStart<'_>, line: usize) -> Result<Element, EntError> {
    let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let mut attributes = Vec::new();
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|error| EntError {
            message: error.to_string(),
            line,
        })?;
        let key = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
        let raw = String::from_utf8_lossy(&attribute.value).into_owned();
        let value = quick_xml::escape::unescape(&raw)
            .map(|value| value.into_owned())
            .unwrap_or(raw);
        attributes.push((key, value));
    }
    Ok(Element {
        name,
        attributes,
        children: Vec::new(),
        line,
    })
}

fn push_child(stack: &mut [Element], child: Node) {
    stack
        .last_mut()
        .expect("something is always open")
        .children
        .push(child);
}

/// Adds text to the open element, joining it onto a text child already at the end, so
/// a reference in the middle of a run of text does not split it in two.
fn push_text(stack: &mut [Element], text: &str) {
    let children = &mut stack.last_mut().expect("something is always open").children;
    match children.last_mut() {
        Some(Node::Text(existing)) => existing.push_str(text),
        _ => children.push(Node::Text(text.to_string())),
    }
}

/// What a character or predefined entity reference stands for.
fn resolve_reference(name: &str) -> Option<String> {
    if let Some(rest) = name.strip_prefix('#') {
        let code = match rest.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => rest.parse::<u32>().ok()?,
        };
        return char::from_u32(code).map(String::from);
    }
    quick_xml::escape::resolve_predefined_entity(name).map(String::from)
}

/// `parseClassInfo`: a `<point>` or a `<group>`.
fn read_class(
    element: &Element,
    declarations: &[PropertyDefinition],
    warnings: &mut Vec<String>,
) -> EntityDefinition {
    let is_point = element.name == "point";
    let name = element.string("name");
    let color = read_color(element, warnings);
    let bounds = if is_point {
        read_bounds(element, warnings)
    } else {
        None
    };
    let model = if is_point {
        read_model(element, warnings)
    } else {
        None
    };
    // Trims description indentation at both ends.
    let description = element.text().trim().to_string();

    let mut properties = Vec::new();
    if let Some(spawnflags) = read_spawnflags(element, warnings) {
        properties.push(spawnflags);
    }
    for child in element.elements() {
        let Some(property) = read_property(child, declarations, warnings) else {
            continue;
        };
        if properties.iter().any(|p: &PropertyDefinition| p.key == property.key) {
            warnings.push(format!(
                "line {}: skipping duplicate entity property definition \"{}\"",
                child.line, property.key
            ));
            continue;
        }
        properties.push(property);
    }

    EntityDefinition {
        name,
        kind: if is_point {
            DefinitionKind::Point
        } else {
            DefinitionKind::Solid
        },
        description,
        base_classes: Vec::new(),
        bounds,
        color,
        model,
        properties,
    }
}

/// `parseColor`: three numbers, as fractions of one the way Radiant writes them, or as
/// bytes when any of them is past one.
fn read_color(element: &Element, warnings: &mut Vec<String>) -> Option<[u8; 3]> {
    let text = element.attribute("color")?;
    let numbers: Vec<f64> = text
        .split_whitespace()
        .filter_map(|word| word.parse::<f64>().ok())
        .collect();
    if numbers.len() < 3 {
        warnings.push(format!(
            "line {}: invalid colour \"{text}\" on {}",
            element.line, element.name
        ));
        return None;
    }
    let bytes = numbers.iter().any(|n| *n > 1.0);
    let channel = |value: f64| {
        let scaled = if bytes { value } else { value * 255.0 };
        scaled.round().clamp(0.0, 255.0) as u8
    };
    Some([channel(numbers[0]), channel(numbers[1]), channel(numbers[2])])
}

/// `parseBounds`: six numbers, the minimum corner then the maximum.
fn read_bounds(element: &Element, warnings: &mut Vec<String>) -> Option<Bbox3d> {
    let text = element.attribute("box")?;
    let numbers: Vec<f64> = text
        .split_whitespace()
        .filter_map(|word| word.parse::<f64>().ok())
        .collect();
    if numbers.len() != 6 {
        warnings.push(format!(
            "line {}: invalid bounding box \"{text}\" on {}",
            element.line, element.name
        ));
        return None;
    }
    Some(Bbox3d::new(
        Vec3d::new(numbers[0], numbers[1], numbers[2]),
        Vec3d::new(numbers[3], numbers[4], numbers[5]),
    ))
}

/// `parseModel`: the `model` attribute as an expression, or, when it is not one, as the
/// plain path it usually is.
fn read_model(element: &Element, warnings: &mut Vec<String>) -> Option<ModelDefinition> {
    let text = element.attribute("model")?;
    if text.trim().is_empty() {
        return None;
    }
    match crate::model_definition::parse_model_argument(text, element.line) {
        Ok(parsed) => {
            if let Some(note) = parsed.deprecation {
                warnings.push(format!("line {}: {note}", element.line));
            }
            Some(parsed.definition)
        }
        Err(_) => {
            // Treats a bare model path as a literal path specification.
            let quoted = format!("{:?}", text);
            crate::model_definition::parse_model_argument(&quoted, element.line)
                .ok()
                .map(|parsed| parsed.definition)
        }
    }
}

/// `parseSpawnflags`: the `<flag>` children, each `1 << bit`.
fn read_spawnflags(element: &Element, warnings: &mut Vec<String>) -> Option<PropertyDefinition> {
    let flags: Vec<Flag> = element
        .elements()
        .filter(|child| child.name == "flag")
        .filter_map(|flag| {
            let bit_text = flag.string("bit");
            let Ok(bit) = bit_text.trim().parse::<u32>() else {
                warnings.push(format!(
                    "line {}: invalid value \"{bit_text}\" for bit property definition",
                    flag.line
                ));
                return None;
            };
            let key = flag.string("key");
            let name = flag.string("name");
            Some(Flag {
                value: 1i64 << bit.min(62),
                description: if name.is_empty() { key } else { name },
                set_by_default: false,
            })
        })
        .collect();
    if flags.is_empty() && element.first_element("flag").is_none() {
        return None;
    }
    Some(PropertyDefinition {
        key: SPAWNFLAGS.to_string(),
        kind: PropertyType::Flags(flags),
        short_description: "Spawnflags".to_string(),
        default: None,
        long_description: None,
    })
}

/// `parseListDeclaration`: a `<list name="...">` of `<item name value/>`s.
fn read_list_declaration(element: &Element, warnings: &mut Vec<String>) -> Option<PropertyDefinition> {
    if !element.has_attribute("name") {
        warnings.push(format!("line {}: expected attribute 'name': list", element.line));
        return None;
    }
    let choices = element
        .elements()
        .filter(|item| item.name == "item")
        .filter_map(|item| {
            if !item.has_attribute("name") || !item.has_attribute("value") {
                warnings.push(format!(
                    "line {}: expected attributes 'name' and 'value': item",
                    item.line
                ));
                return None;
            }
            Some(Choice {
                value: item.string("value"),
                description: item.string("name"),
            })
        })
        .collect();
    Some(PropertyDefinition {
        key: element.string("name"),
        kind: PropertyType::Choices(choices),
        short_description: String::new(),
        default: None,
        long_description: None,
    })
}

/// `parsePropertyDefinition`: one property element, typed by its name. Anything that is
/// neither a known type nor a declared list is not a property at all.
fn read_property(
    element: &Element,
    declarations: &[PropertyDefinition],
    warnings: &mut Vec<String>,
) -> Option<PropertyDefinition> {
    let kind = match element.name.as_str() {
        "angle" | "angles" | "direction" | "texture" | "sound" | "model" => {
            PropertyType::Other(element.name.clone())
        }
        "boolean" => PropertyType::Boolean,
        "integer" => PropertyType::Integer,
        "real" => PropertyType::Float,
        "string" => PropertyType::String,
        "target" => PropertyType::TargetDestination,
        "targetname" => PropertyType::TargetSource,
        "color" => PropertyType::Color,
        other => declarations
            .iter()
            .find(|declaration| declaration.key == other)?
            .kind
            .clone(),
    };

    for attribute in ["key", "name"] {
        if !element.has_attribute(attribute) {
            warnings.push(format!(
                "line {}: expected attribute '{attribute}': {}",
                element.line, element.name
            ));
            return None;
        }
    }
    let key = element.string("key");
    let short_description = element.string("name");
    let long_description = element.text();
    let long_description = (!long_description.is_empty()).then_some(long_description);

    // Validates typed defaults; invalid numeric defaults become text with a warning.
    let mut kind = kind;
    let default = element.attribute("value").map(|value| match kind {
        PropertyType::Integer if value.trim().parse::<i64>().is_err() => {
            warnings.push(format!(
                "line {}: invalid default value '{value}' for integer property definition",
                element.line
            ));
            kind = PropertyType::Other("integer".to_string());
            value.to_string()
        }
        PropertyType::Float if value.trim().parse::<f64>().is_err() => {
            warnings.push(format!(
                "line {}: invalid default value '{value}' for float property definition",
                element.line
            ));
            kind = PropertyType::Other("real".to_string());
            value.to_string()
        }
        PropertyType::Boolean => match parse_boolean(value) {
            Some(truth) => if truth { "1" } else { "0" }.to_string(),
            None => {
                warnings.push(format!(
                    "line {}: invalid default value '{value}' for boolean property definition",
                    element.line
                ));
                kind = PropertyType::Other("boolean".to_string());
                value.to_string()
            }
        },
        _ => value.to_string(),
    });
    // Link properties have no default value.
    let default = match kind {
        PropertyType::TargetSource | PropertyType::TargetDestination => None,
        _ => default,
    };

    Some(PropertyDefinition {
        key,
        kind,
        short_description,
        default,
        long_description,
    })
}

/// `parseBoolean`: `true` and `false` in any case, else any whole number, nonzero being
/// true.
fn parse_boolean(text: &str) -> Option<bool> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("true") {
        return Some(true);
    }
    if text.eq_ignore_ascii_case("false") {
        return Some(false);
    }
    text.parse::<i64>().ok().map(|value| value != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn property(definition: &EntityDefinition, key: &str) -> PropertyDefinition {
        definition
            .property(key)
            .unwrap_or_else(|| panic!("{key} is defined"))
            .clone()
    }

    #[test]
    fn an_empty_file_is_no_definitions() {
        assert!(read_ent("").unwrap().all().is_empty());
        assert!(read_ent("     \n  \t \n  ").unwrap().all().is_empty());
    }

    #[test]
    fn malformed_xml_is_an_error() {
        let file = r#"<?xml version="1.0"?>
<classes>
    <point name="_skybox" color="0.77 0.88 1.0" box="-4 -4 -4 4 4 4">
</classes>"#;
        assert!(read_ent(file).is_err());
    }

    #[test]
    fn a_point_class_with_its_properties_and_the_text_around_them() {
        let file = r#"
<?xml version="1.0"?>
<!--
Quake3 Arena entity definition file for Q3Radiant
-->
<classes>
    <!-- Q3MAP2 ENTITIES -->
    <point name="_skybox" color="0.77 0.88 1.0" box="-4 -4 -4 4 4 4">
    -------- KEYS --------
    asdf<angle key="angle" name="Yaw Angle">Rotation angle of the sky surfaces.</angle>
    <angles key="angles" name="Pitch Yaw Roll">Individual control of PITCH, YAW, and ROLL (default 0 0 0).</angles>
    <real key="_scale" name="Scale" value="64">Scaling factor (default 64), good values are between 50 and 300, depending on the map.</real>
    -------- NOTES --------
    Compiler-only entity that specifies the origin of a skybox.
    </point>
</classes>
"#;
        let definitions = read_ent(file).unwrap();
        assert_eq!(definitions.all().len(), 1);
        let skybox = &definitions.all()[0];
        assert_eq!(skybox.name, "_skybox");
        assert_eq!(skybox.kind, DefinitionKind::Point);
        assert_eq!(skybox.color, Some([196, 224, 255]));
        assert_eq!(
            skybox.bounds,
            Some(Bbox3d::new(Vec3d::new(-4.0, -4.0, -4.0), Vec3d::new(4.0, 4.0, 4.0)))
        );
        // Descriptions join the first and last text children, excluding text between property
        // elements.
        assert_eq!(
            skybox.description,
            "-------- KEYS --------\n    asdf\n    -------- NOTES --------\n    Compiler-only entity that specifies the origin of a skybox."
        );
        assert_eq!(skybox.properties.len(), 3);
        let angle = property(skybox, "angle");
        assert_eq!(angle.kind, PropertyType::Other("angle".to_string()));
        assert_eq!(angle.short_description, "Yaw Angle");
        assert_eq!(
            angle.long_description.as_deref(),
            Some("Rotation angle of the sky surfaces.")
        );
        let scale = property(skybox, "_scale");
        assert_eq!(scale.kind, PropertyType::Float);
        assert_eq!(scale.default.as_deref(), Some("64"));
    }

    #[test]
    fn a_class_defined_twice_keeps_the_later_one_with_a_warning() {
        let file = r#"
<?xml version="1.0"?>
<classes>
  <point name="_skybox" color="1 0 0" box="-1 -1 -1 1 1 1"></point>
  <point name="_skybox" color="0 1 0" box="-2 -2 -2 2 2 2"></point>
</classes>
"#;
        let definitions = read_ent(file).unwrap();
        assert_eq!(definitions.all().len(), 1);
        assert_eq!(definitions.all()[0].color, Some([0, 255, 0]));
        assert_eq!(
            definitions.all()[0].bounds,
            Some(Bbox3d::new(Vec3d::new(-2.0, -2.0, -2.0), Vec3d::new(2.0, 2.0, 2.0)))
        );
        assert_eq!(definitions.warnings.len(), 1, "{:?}", definitions.warnings);
    }

    #[test]
    fn a_group_class_with_spawnflags_and_every_property_type() {
        let file = r#"
<?xml version="1.0"?>
<classes>
<group name="func_bobbing" color="0 .4 1">
Solid entity that oscillates back and forth in a linear motion.
-------- KEYS --------
<sound key="noise" name="Sound File">Path/name of .wav file to play.</sound>
<model key="model2" name="Model File">Path/name of model to include.</model>
<color key="color" name="Model Light Color" value="1 1 1">Color of constant light.</color>
-------- Q3MAP2 KEYS --------
<targetname key="targetname" name="Target Name">Used to attach a misc_model entity to this entity.</targetname>
<integer key="_castshadows" name="Shadow Caster Level" value="0">0 = no shadow casting. &gt; 1 = cast shadows on entities.</integer>
<texture key="_celshader" name="Cel Shader">Sets the cel shader used for this geometry.</texture>
-------- SPAWNFLAGS --------
<flag key="X_AXIS" name="X Axis" bit="0">Entity will bob along the X axis.</flag>
<flag key="Y_AXIS" name="Y Axis" bit="1">Entity will bob along the Y axis.</flag>
-------- NOTES --------
Target this entity with a misc_model.
</group>
</classes>"#;
        let definitions = read_ent(file).unwrap();
        let bobbing = &definitions.all()[0];
        assert_eq!(bobbing.name, "func_bobbing");
        assert_eq!(bobbing.kind, DefinitionKind::Solid);
        assert_eq!(bobbing.color, Some([0, 102, 255]));
        assert_eq!(bobbing.bounds, None);
        assert!(bobbing.description.starts_with("Solid entity that oscillates"));
        assert!(bobbing.description.ends_with("Target this entity with a misc_model."));

        let keys: Vec<&str> = bobbing.properties.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(
            keys,
            ["spawnflags", "noise", "model2", "color", "targetname", "_castshadows", "_celshader"]
        );
        let PropertyType::Flags(flags) = &property(bobbing, "spawnflags").kind else {
            panic!("spawnflags are flags");
        };
        assert_eq!(flags.len(), 2);
        assert_eq!((flags[0].value, flags[0].description.as_str()), (1, "X Axis"));
        assert_eq!((flags[1].value, flags[1].description.as_str()), (2, "Y Axis"));
        assert_eq!(property(bobbing, "noise").kind, PropertyType::Other("sound".to_string()));
        assert_eq!(property(bobbing, "color").kind, PropertyType::Color);
        assert_eq!(property(bobbing, "color").default.as_deref(), Some("1 1 1"));
        assert_eq!(property(bobbing, "targetname").kind, PropertyType::TargetSource);
        let shadows = property(bobbing, "_castshadows");
        assert_eq!(shadows.kind, PropertyType::Integer);
        assert_eq!(shadows.default.as_deref(), Some("0"));
        // The `&gt;` came back as what it stands for.
        assert_eq!(
            shadows.long_description.as_deref(),
            Some("0 = no shadow casting. > 1 = cast shadows on entities.")
        );
    }

    #[test]
    fn a_point_class_without_a_box_has_no_bounds_and_a_target_property() {
        let file = r#"
<?xml version="1.0"?>
  <classes>
    <point name= "linkEmitter" color="0.2 0.5 0.2 ">
      <target key="target" name="target"></target>
    </point>
  </classes>
"#;
        let definitions = read_ent(file).unwrap();
        let emitter = &definitions.all()[0];
        assert_eq!(emitter.name, "linkEmitter");
        // Missing bounds use the document default of eight units in each direction.
        assert_eq!(emitter.bounds, None);
        assert_eq!(property(emitter, "target").kind, PropertyType::TargetDestination);
        assert_eq!(property(emitter, "target").long_description, None);
    }

    #[test]
    fn a_declared_list_types_the_properties_that_name_it() {
        let file = r#"
<?xml version="1.0"?>
<classes>
<list name="colorIndex">
<item name="white" value="0"/>
<item name="red" value="1"/>
<item name="green" value="2"/>
</list>
<point name="_skybox" color="0.77 0.88 1.0" box="-4 -4 -4 4 4 4">
<colorIndex key="count" name="Text Color" value="0">Color of the location text.
0 : White (default)
1 : Red</colorIndex>
</point>
</classes>
"#;
        let definitions = read_ent(file).unwrap();
        let count = property(&definitions.all()[0], "count");
        assert_eq!(
            count.kind,
            PropertyType::Choices(vec![
                Choice { value: "0".into(), description: "white".into() },
                Choice { value: "1".into(), description: "red".into() },
                Choice { value: "2".into(), description: "green".into() },
            ])
        );
        assert_eq!(count.default.as_deref(), Some("0"));
        assert_eq!(count.short_description, "Text Color");
        assert_eq!(
            count.long_description.as_deref(),
            Some("Color of the location text.\n0 : White (default)\n1 : Red")
        );
    }

    #[test]
    fn a_boolean_reads_words_and_numbers() {
        let file = r#"
<?xml version="1.0"?>
<classes>
  <point name="_skybox" color="0.77 0.88 1.0" box="-4 -4 -4 4 4 4">
    <boolean key="prop_true"  name="true"  value="true" />
    <boolean key="prop_false" name="false" value="false" />
    <boolean key="prop_True"  name="True"  value="True" />
    <boolean key="prop_0"     name="0"     value="0" />
    <boolean key="prop_2"     name="2"     value="2" />
    <boolean key="prop_n1"    name="-1"    value="-1" />
    <boolean key="prop_none"  name="none" />
  </point>
</classes>
"#;
        let definitions = read_ent(file).unwrap();
        let skybox = &definitions.all()[0];
        for (key, expected) in [
            ("prop_true", Some("1")),
            ("prop_false", Some("0")),
            ("prop_True", Some("1")),
            ("prop_0", Some("0")),
            ("prop_2", Some("1")),
            ("prop_n1", Some("1")),
            ("prop_none", None),
        ] {
            let found = property(skybox, key);
            assert_eq!(found.kind, PropertyType::Boolean, "{key}");
            assert_eq!(found.default.as_deref(), expected, "{key}");
        }
    }

    #[test]
    fn a_number_that_is_not_one_becomes_text_with_a_warning() {
        let file = r#"
<?xml version="1.0"?>
<classes>
    <point name="_skybox" color="0.77 0.88 1.0" box="-4 -4 -4 4 4 4">
        <real key="_scale" name="Scale" value="asdf" />
        <integer key="_count" name="Count" value="many" />
    </point>
</classes>
"#;
        let definitions = read_ent(file).unwrap();
        let skybox = &definitions.all()[0];
        assert_eq!(property(skybox, "_scale").kind, PropertyType::Other("real".to_string()));
        assert_eq!(property(skybox, "_scale").default.as_deref(), Some("asdf"));
        assert_eq!(property(skybox, "_count").kind, PropertyType::Other("integer".to_string()));
        assert_eq!(definitions.warnings.len(), 2, "{:?}", definitions.warnings);
    }

    #[test]
    fn a_model_attribute_names_what_the_class_is_drawn_as() {
        let file = r#"
<classes>
  <point name="ammo_bfg" color=".3 .3 1" box="-16 -16 -16 16 16 16" model="models/powerups/ammo/bfgam.md3">BFG ammo.</point>
  <point name="item_switch" color=".3 .3 1" box="-16 -16 -16 16 16 16" model="{{ spawnflags == 1 -> 'a.md3', 'b.md3' }}"></point>
</classes>
"#;
        let definitions = read_ent(file).unwrap();
        let ammo = definitions.find("ammo_bfg").unwrap();
        let model = ammo.model.as_ref().expect("a model");
        assert_eq!(
            model.default_model_specification().unwrap().path,
            std::path::PathBuf::from("models/powerups/ammo/bfgam.md3")
        );
        let switch = definitions.find("item_switch").unwrap();
        let model = switch.model.as_ref().expect("a model");
        assert_eq!(
            model.resolve(&[("spawnflags".to_string(), "1".to_string())]).unwrap().path,
            std::path::PathBuf::from("a.md3")
        );
    }

    /// Reads a full Quake 3 `entities.ent` when `HERON_QUAKE3_ENT` names one; skipped
    /// otherwise, since the file is not part of this repository.
    #[test]
    fn a_full_quake_3_file_reads_whole() {
        let Some(path) = std::env::var_os("HERON_QUAKE3_ENT") else {
            return;
        };
        let Ok(source) = std::fs::read_to_string(&path) else {
            return;
        };
        let definitions = read_ent(&source).unwrap();
        // The file has 86 classes, most of them the Q3Map2 compiler's.
        assert!(definitions.all().len() > 80, "{} classes", definitions.all().len());
        let spawn = definitions.find("info_player_deathmatch").expect("a player start");
        assert_eq!(spawn.kind, DefinitionKind::Point);
        assert!(spawn.bounds.is_some());
        let bobbing = definitions.find("func_bobbing").expect("a mover");
        assert_eq!(bobbing.kind, DefinitionKind::Solid);
        assert!(bobbing.property("spawnflags").is_some());
        let with_models = definitions.all().iter().filter(|d| d.model.is_some()).count();
        assert!(with_models > 20, "{with_models} classes name a model");
    }

    #[test]
    fn a_duplicate_property_in_one_class_is_skipped_with_a_warning() {
        let file = r#"
<classes>
  <point name="x" color="1 1 1" box="-1 -1 -1 1 1 1">
    <string key="a" name="A" value="1" />
    <string key="a" name="A again" value="2" />
  </point>
</classes>
"#;
        let definitions = read_ent(file).unwrap();
        let x = &definitions.all()[0];
        assert_eq!(x.properties.len(), 1);
        assert_eq!(property(x, "a").default.as_deref(), Some("1"));
        assert_eq!(definitions.warnings.len(), 1);
    }
}
