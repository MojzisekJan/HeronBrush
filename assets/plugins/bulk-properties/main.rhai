// Set a project property on selected point/brush entities, as one Undo step.
let chosen = selected_entities();
let key = setting("key");
let value = setting("value");
let changed = 0;
if action() == "apply" {
    for entity in chosen { if set_property(entity, key, value) { changed += 1; } }
}
ui_value("Selected entities", chosen.len());
ui_value("Property", key);
ui_value("Value", value);
if action() == "apply" { ui_value("Changed entities", changed); }
