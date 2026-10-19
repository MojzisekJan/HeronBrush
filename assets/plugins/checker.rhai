// Paint the selected faces in two alternating greys by face index, for blocking out a
// level before any materials are on it.
//
//   heron-brush --run-script assets/plugins/checker.rhai level.map

let light = [180, 180, 185, 255];
let dark = [90, 90, 96, 255];
let n = 0;
for b in selection() {
    for f in range(0, face_count(b)) {
        set_face_color(b, f, if n % 2 == 0 { light } else { dark });
        n += 1;
    }
}
say("checkered " + n + " faces");
