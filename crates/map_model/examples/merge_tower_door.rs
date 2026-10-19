//! Merge convex neighbours without snapping or deleting detail. Input and output map paths.
use geometry_math::{bbox::Bbox3d,vec::{self}};
use map_model::{brush::Brush,csg,polyhedron::Polyhedron,io::{map_reader::read_map,map_writer::write_map}};
fn signature(b:&Brush)->Option<String>{
 if b.faces().iter().any(|f| f.paint().is_some_and(|c| c.alpha()!=255 || !matches!(c,map_model::brush_face::FaceColor::Solid{..}))) {return None;}
 let mut keys:Vec<_>=b.faces().iter().filter(|f|f.material_name.as_str()!="skip").map(|f|format!("{} {:?} {:?} {:?} {:?} {:?} {:?}",f.material_name,f.color,f.surface_material,f.uv_attributes,f.uv_coord_system,f.primitive_uv,f.surface_attributes)).collect();keys.sort();keys.dedup();
 if keys.len()==1{keys.pop()}else{None}
}
fn main(){
 let args:Vec<_>=std::env::args().skip(1).collect();assert_eq!(args.len(),2);assert!(!std::path::Path::new(&args[1]).exists());
 let source=std::fs::read_to_string(&args[0]).unwrap();let mut map=read_map(&source,None).unwrap();let world=Bbox3d::from_scalars(-16384.,16384.);
 let e=&mut map.entities[0];let ids=[2366468980857635050u64,2366468980857635064u64];
 let ix:Vec<_>=ids.iter().map(|id|e.brushes.iter().position(|b|b.id==Some(*id)).unwrap()).collect();
 let a=Brush::from_faces(&world,e.brushes[ix[0]].faces.clone()).unwrap();let b=Brush::from_faces(&world,e.brushes[ix[1]].faces.clone()).unwrap();
 assert_eq!(signature(&a),signature(&b));
 let mut points=a.vertex_positions();points.extend(b.vertex_positions());
 let poly=Polyhedron::from_points(&points,1e-7).unwrap();let raw=Brush::from_polyhedron(poly,"skip");
 let mut faces=Vec::new();
 for f in raw.faces(){
 let old=[&a,&b].into_iter().flat_map(|b|b.faces()).filter(|o|vec::length(o.plane.normal-f.plane.normal)<1e-7&&(o.plane.distance-f.plane.distance).abs()<1e-6).min_by_key(|o|o.material_name.as_str()=="skip");
 let mut face=old.cloned().unwrap_or_else(||f.clone());face.points=f.points;face.plane=f.plane;faces.push(face);
 }
 let h=Brush::from_faces(&world,faces).unwrap();
 let extra:f64=csg::subtract_all(&h,&[a.clone(),b.clone()]).iter().map(Brush::volume).sum();
 println!("extra volume={extra}, original volume={}",a.volume()+b.volume());
 assert!(extra<0.12);
 assert!(![&a,&b].iter().any(|b|csg::subtract(b,&h).iter().map(Brush::volume).sum::<f64>()>0.000001));
 e.brushes[ix[0]].faces=h.faces().to_vec();e.brushes.remove(ix[1]);
 let mut text=String::new();for l in source.lines().take_while(|l|!l.starts_with("// Format:")){text.push_str(l);text.push('\n');}text.push_str(&write_map(&map));
 let check=read_map(&text,None).unwrap();for b in check.entities.iter().flat_map(|e|&e.brushes){assert!(Brush::from_faces(&world,b.faces.clone()).unwrap().volume()>0.);}
 std::fs::write(&args[1],text).unwrap();
}
