// Stack copies of each selected brush: boxes the same size as the original, each one
// its own height further along, for a quick column or a rough staircase.
//
//   heron-brush --run-script assets/plugins/stack/main.rhai level.map
//
// How many, how far apart and which way are settings the manifest declares, so the
// Plugins window asks for them. Run headless and each falls back to its default here,
// since setting() answers () for a key nobody has answered.

let copies = setting("copies") ?? 5.0;
let spacing = setting("spacing") ?? 0.0;
let direction = setting("direction") ?? "up";

let way = if direction == "down" { -1.0 } else { 1.0 };

let made = 0;
for b in selection() {
    let box = brush_bounds(b);
    let lo = box[0];
    let hi = box[1];
    let step = (hi[2] - lo[2] + spacing) * way;
    for i in range(1, copies.to_int() + 1) {
        let dz = step * i;
        add_cuboid(
            [lo[0], lo[1], lo[2] + dz],
            [hi[0], hi[1], hi[2] + dz],
            (),
        );
        made += 1;
    }
}
say("stacked " + made + " copies");
