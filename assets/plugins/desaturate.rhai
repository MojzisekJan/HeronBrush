// Turn the painted colours on the selection to greyscale.
//
// A solid colour becomes its luminance (0.299 R + 0.587 G + 0.114 B); a gradient's two
// ends each do the same; alpha and angle are kept. A face with no painted colour is left
// alone. Run with:
//
//   heron-brush --run-script assets/plugins/desaturate.rhai coloured.map grey.map

fn grey(rgb) {
    let l = (0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2]).to_int();
    [l, l, l, rgb[3]]
}

let painted = 0;
for b in selection() {
    for f in range(0, face_count(b)) {
        let c = face_color(b, f);
        if c == () { continue; }
        if c[0] == "grad" {
            set_face_color(b, f, ["grad", grey(c[1]), grey(c[2]), c[3]]);
        } else {
            set_face_color(b, f, grey(c));
        }
        painted += 1;
    }
}
say("desaturated " + painted + " faces");
