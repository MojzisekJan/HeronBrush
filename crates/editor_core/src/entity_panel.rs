//! What the entity inspector shows, apart from how: several selected entities merged into
//! one property list, a `choices` property's menu, and a colour property read.

use map_model::fgd::Choice;

/// The value shown when the selected entities do not agree on a property.
pub const MULTIPLE_VALUES: &str = "<multiple>";

/// Merges the selected entities into one for the property grid: a key every entity has,
/// with the shared value, or [`MULTIPLE_VALUES`] where they differ.
pub fn merge_entities<'a>(
    mut entities: impl Iterator<Item = &'a map_model::node::EntityNode> + Clone,
) -> map_model::node::EntityNode {
    let Some(first) = entities.next() else {
        return map_model::node::EntityNode {
            properties: Vec::new(),
            children: Vec::new(),
        };
    };
    let rest: Vec<&map_model::node::EntityNode> = entities.collect();

    let properties = first
        .properties
        .iter()
        .filter(|(key, _)| rest.iter().all(|e| e.property(key).is_some()))
        .map(|(key, value)| {
            let shared = rest.iter().all(|e| e.property(key) == Some(value.as_str()));
            (
                key.clone(),
                if shared {
                    value.clone()
                } else {
                    MULTIPLE_VALUES.to_string()
                },
            )
        })
        .collect();

    map_model::node::EntityNode {
        properties,
        children: first.children.clone(),
    }
}

/// The dropdown contents for a `choices` property: a label and a value per entry, and
/// which one is selected.
pub struct ChoiceMenu {
    pub labels: Vec<String>,
    pub values: Vec<String>,
    pub selected: usize,
}

/// `SmartChoiceEditor`'s editable combo box: the class's choices, plus -- when the value
/// the map wrote is not one of them -- that value as a last entry, so it is shown and kept
/// rather than silently swapped for the first choice.
pub fn choice_menu(choices: &[Choice], value: &str) -> ChoiceMenu {
    let mut labels: Vec<String> = choices
        .iter()
        .map(|choice| format!("{} — {}", choice.value, choice.description))
        .collect();
    let mut values: Vec<String> = choices.iter().map(|c| c.value.clone()).collect();

    let selected = choices
        .iter()
        .position(|c| c.value == value)
        .unwrap_or_else(|| {
            labels.push(format!("{value} — (not a listed choice)"));
            values.push(value.to_string());
            values.len() - 1
        });

    ChoiceMenu {
        labels,
        values,
        selected,
    }
}

/// A colour written as three numbers, as the map format holds one.
///
/// Some games write them from 0 to 255 and some from 0 to 1; a value above 1 can only be
/// the first, which is what tells them apart.
pub fn parse_colour(value: &str) -> Option<[f32; 3]> {
    let parts: Vec<f32> = value
        .split_whitespace()
        .filter_map(|part| part.parse::<f32>().ok())
        .collect();
    if parts.len() < 3 {
        return None;
    }

    let scale = if parts.iter().any(|part| *part > 1.0) {
        1.0 / 255.0
    } else {
        1.0
    };
    Some([
        (parts[0] * scale).clamp(0.0, 1.0),
        (parts[1] * scale).clamp(0.0, 1.0),
        (parts[2] * scale).clamp(0.0, 1.0),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_model::fgd::Choice;

    #[test]
    fn a_choice_menu_keeps_a_value_the_class_does_not_list() {
        let choices = vec![
            Choice {
                value: "0".to_string(),
                description: "Off".to_string(),
            },
            Choice {
                value: "1".to_string(),
                description: "On".to_string(),
            },
        ];

        // A listed value selects its own entry, and adds nothing.
        let menu = choice_menu(&choices, "1");
        assert_eq!(menu.values, vec!["0", "1"]);
        assert_eq!(menu.selected, 1);

        // A value the map wrote that is not listed is appended and selected.
        let menu = choice_menu(&choices, "7");
        assert_eq!(menu.values, vec!["0", "1", "7"]);
        assert_eq!(menu.selected, 2);
        assert!(menu.labels[2].contains("not a listed choice"));
    }

    #[test]
    fn a_colour_in_bytes_is_read_as_bytes() {
        let colour = parse_colour("255 128 0").expect("three numbers");
        geometry_math::assert_approx_eq!(colour[0], 1.0, 1e-3);
        geometry_math::assert_approx_eq!(colour[1], 128.0 / 255.0, 1e-3);
        geometry_math::assert_approx_eq!(colour[2], 0.0, 1e-3);
    }

    #[test]
    fn a_colour_in_fractions_is_read_as_fractions() {
        // Nothing above one, so the numbers can only be fractions -- reading them as bytes
        // would make every such colour almost black.
        let colour = parse_colour("1 0.5 0").expect("three numbers");
        geometry_math::assert_approx_eq!(colour[0], 1.0, 1e-3);
        geometry_math::assert_approx_eq!(colour[1], 0.5, 1e-3);
        geometry_math::assert_approx_eq!(colour[2], 0.0, 1e-3);
    }

    #[test]
    fn a_colour_with_more_than_three_numbers_uses_the_first_three() {
        // Some games put a brightness after the colour.
        let colour = parse_colour("255 255 255 300").expect("three numbers");
        geometry_math::assert_approx_eq!(colour[0], 1.0, 1e-3);
    }

    #[test]
    fn anything_that_is_not_a_colour_is_not_one() {
        assert!(parse_colour("").is_none());
        assert!(parse_colour("255 128").is_none());
        assert!(parse_colour("red").is_none());
    }

    #[test]
    fn a_colour_out_of_range_is_brought_back_into_it() {
        let colour = parse_colour("300 -20 128").expect("three numbers");
        geometry_math::assert_approx_eq!(colour[0], 1.0, 1e-3);
        geometry_math::assert_approx_eq!(colour[1], 0.0, 1e-3);
    }
}
