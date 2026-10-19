//! Read-only audit of reconstructed asset vertices on grid 1.
use geometry_math::bbox::Bbox3d;
use map_model::{brush::Brush, io::map_reader::read_map};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args().nth(1).unwrap_or_else(|| "assets".into());
    let mut paths = std::fs::read_dir(input)?.map(|p| p.map(|p| p.path())).collect::<Result<Vec<_>, _>>()?;
    paths.retain(|p| p.extension().is_some_and(|e| e == "map"));
    paths.sort();
    println!("asset\tbrushes\toff_grid_brushes\toff_grid_vertices\tmax_axis_rounding\taxis_aligned");
    for path in paths {
        let map = read_map(&std::fs::read_to_string(&path)?, None)?;
        if !map.warnings.is_empty() { return Err(format!("{}: parser warnings", path.display()).into()); }
        let (mut count, mut off_brushes, mut off_vertices, mut max_move, mut aligned) = (0, 0, 0, 0.0_f64, true);
        for entry in map.entities.iter().flat_map(|e| &e.brushes) {
            let brush = Brush::from_faces(&Bbox3d::from_scalars(-16384., 16384.), entry.faces.clone())?;
            count += 1;
            let mut off = false;
            for v in brush.geometry().vertices() {
                let distance = (0..3).map(|i| (v.position[i] - v.position[i].round()).abs()).fold(0.0_f64, f64::max);
                if distance > 1e-5 { off_vertices += 1; off = true; }
                max_move = max_move.max(distance);
            }
            off_brushes += usize::from(off);
            aligned &= brush.faces().iter().all(|f| (0..3).any(|i| f.plane.normal[i].abs() > 1.0 - 1e-7));
        }
        println!("{}\t{count}\t{off_brushes}\t{off_vertices}\t{max_move:.6}\t{aligned}", path.file_stem().unwrap().to_string_lossy());
    }
    Ok(())
}
