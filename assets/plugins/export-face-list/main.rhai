// Report on the selection rather than change it: how many brushes, how many faces, and
// how many of those faces carry a painted colour rather than a material.
//
//   heron-brush --run-script assets/plugins/export-face-list/main.rhai level.map
//
// Nothing is written to the map, so running it twice says the same thing twice. This is
// the "a plugin may just look" case, and the folder it lives in carries a plugin.toml
// so it is also the worked example of a manifest.

let brushes = selection();
let faces = 0;
let painted = 0;
let gradients = 0;

for b in brushes {
    let count = face_count(b);
    faces += count;
    for f in range(0, count) {
        let colour = face_color(b, f);
        if colour == () {
            continue;
        }
        painted += 1;
        // A gradient comes back as ["grad", from, to, angle]; a solid colour as [r,g,b,a].
        if colour[0] == "grad" {
            gradients += 1;
        }
    }
}

say(brushes.len() + " brushes, " + faces + " faces");
say(painted + " painted, " + gradients + " of them a gradient");
