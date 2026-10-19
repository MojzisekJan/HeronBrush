// Colour each selected brush by how high its centre sits: a cool blue at the bottom of
// the selection, a warm yellow at the top, so a scene reads as a heightmap.
//
//   heron-brush --run-script assets/plugins/recolour_by_height.rhai scene.map

let sel = selection();
if sel.is_empty() { say("nothing selected"); return; }

// The span of centre heights across the selection.
let lo = 1e30;
let hi = -1e30;
for b in sel {
    let z = brush_center(b)[2];
    if z < lo { lo = z; }
    if z > hi { hi = z; }
}
let span = if hi - lo < 1.0 { 1.0 } else { hi - lo };

let low = [40, 90, 200];     // cool
let high = [235, 205, 70];   // warm
fn lerp(a, b, t) { (a + (b - a) * t).to_int() }

for b in sel {
    let t = (brush_center(b)[2] - lo) / span;
    let c = [lerp(low[0], high[0], t), lerp(low[1], high[1], t), lerp(low[2], high[2], t), 255];
    for f in range(0, face_count(b)) {
        set_face_color(b, f, c);
    }
}
say("recoloured " + sel.len() + " brushes by height");
