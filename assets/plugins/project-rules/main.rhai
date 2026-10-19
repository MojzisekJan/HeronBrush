// Project-owned rules. This panel reports issues and never edits the map.
let all = entities();
let names = #{};
let issues = 0;
for entity in all {
    let name = property(entity, "targetname") ?? "";
    if name != "" {
        if name in names { ui_text("Duplicate targetname: " + name); issues += 1; }
        names[name] = true;
    }
    if (property(entity, "classname") ?? "") == setting("classname") {
        if (property(entity, setting("required")) ?? "") == "" {
            ui_text("Missing " + setting("required") + " on " + setting("classname")); issues += 1;
        }
    }
}
for entity in all {
    let target = property(entity, "target") ?? "";
    if target != "" && !(target in names) { ui_text("Unresolved target: " + target); issues += 1; }
}
ui_value("Entities checked", all.len());
ui_value("Issues", issues);
