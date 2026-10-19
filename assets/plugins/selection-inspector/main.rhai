// Inspect the selection and move brushes through explicit panel actions.
let selected = selection();
let step = setting("step") ?? 16.0;
if action() == "raise" || action() == "lower" {
    let dz = if action() == "raise" { step } else { -step };
    for brush in selected { translate(brush, [0.0, 0.0, dz]); }
    select(selected);
}
let faces = 0;
for brush in selected { faces += face_count(brush); }
ui_text("Current selection. Choose brushes to inspect or move them.");
ui_value("Brushes", selected.len());
ui_value("Faces", faces);
ui_value("Step (map units)", step);
ui_progress("Brush budget (1000)", selected.len().to_float() / 1000.0);
