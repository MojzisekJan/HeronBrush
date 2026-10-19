//! Miter rectangular window jambs and lintels without changing their union.
//! Usage: miter_tower input.map output.map. Never overwrites either existing file.
use geometry_math::{bbox::Bbox3d, plane::Plane3d, vec::{self, Vec3d}};
use map_model::{brush::{Brush, ClipOutcome}, csg, polyhedron::Polyhedron, io::{map_reader::read_map,map_writer::write_map}};
use std::{collections::HashSet, path::PathBuf};
fn clip(b: &Brush, p: &Plane3d) -> Option<Brush> {
    match b.clip(p) { ClipOutcome::Unchanged=>Some(b.clone()), ClipOutcome::Clipped(b)=>Some(b), ClipOutcome::Empty=>None }
}
fn material(b: &Brush) -> Option<String> {
    let names: HashSet<_> = b.faces().iter().filter(|f| f.material_name.as_str()!="skip").map(|f|f.material_name.to_string()).collect();
    if names.len()==1 { names.into_iter().next() } else { None }
}
fn repartition(a: &Brush,b: &Brush,p:&Plane3d,world:&Bbox3d)->Option<Brush> {
    let points:Vec<_>=[a,b].into_iter().filter_map(|b|clip(b,p)).flat_map(|b|b.vertex_positions()).collect();
    if points.len()<4 {return None;}
    let hull=Polyhedron::from_points(&points,1e-7).ok()?;
    let raw=Brush::from_polyhedron(hull,"skip");
    let mut faces=Vec::new();
    for f in raw.faces() {
        let mut face=f.clone();
        if vec::length(face.plane.normal-p.normal)<1e-6 && (face.plane.distance-p.distance).abs()<1e-5 {
            face.material_name="skip".into();
        } else {
            // Only existing boundary planes may survive. Prefer a visible source surface.
            let original=[a,b].into_iter().flat_map(|b|b.faces()).filter(|old|
                vec::length(old.plane.normal-f.plane.normal)<1e-6 && (old.plane.distance-f.plane.distance).abs()<1e-5)
                .min_by_key(|old|old.material_name.as_str()=="skip")?;
            let points=face.points;let plane=face.plane;
            face=original.clone();face.points=points;face.plane=plane;
        }
        faces.push(face);
    }
    Brush::from_faces(world,faces).ok()
}
fn remainder(a:&Brush,b:&Brush,c:&Brush)->f64 {csg::subtract_all(a,&[b.clone(),c.clone()]).iter().map(Brush::volume).sum()}
fn equivalent(a:&Brush,b:&Brush,c:&Brush,d:&Brush)->bool {
    [remainder(a,c,d),remainder(b,c,d),remainder(c,a,b),remainder(d,a,b)].into_iter().all(|v|v<0.0001)
}
fn same(a:&Brush,b:&Brush)->bool {
    let pts=b.vertex_positions();
    a.vertex_positions().iter().all(|p|pts.iter().any(|q|vec::length(*p-*q)<1e-5)) && a.vertex_positions().len()==pts.len()
}
fn main() {
    let args:Vec<_>=std::env::args().skip(1).collect();assert_eq!(args.len(),2,"input.map output.map");
    let output=PathBuf::from(&args[1]);assert!(!output.exists(),"output exists");
    let source=std::fs::read_to_string(&args[0]).unwrap();let mut map=read_map(&source,None).unwrap();assert!(map.warnings.is_empty());
    let world=Bbox3d::from_scalars(-16384.,16384.);let mut changes=0;let mut changed=HashSet::new();let mut contacts=0;
    for (entity_index,entity) in map.entities.iter_mut().enumerate() {
        let mut brushes:Vec<_>=entity.brushes.iter().map(|b|Brush::from_faces(&world,b.faces.clone()).unwrap()).collect();
        let materials:Vec<_>=brushes.iter().map(material).collect();
        for i in 0..brushes.len() { for j in 0..brushes.len() {
            if i==j || materials[i].is_none() || materials[i]!=materials[j] {continue;}
            let (a,b)=(&brushes[i],&brushes[j]);let (ba,bb)=(a.bounds(),b.bounds());
            // i is a narrow upright jamb, j is the four-unit lintel above it.
            let mut candidate=None;
            for u in 0..2 {
                let depth=1-u;
                if ba.max[depth]<bb.min[depth]-1e-5 || bb.max[depth]<ba.min[depth]-1e-5 {continue;}
                if ((ba.max[u]-ba.min[u])-4.).abs()>1e-5 || ba.max.z()-ba.min.z()<8. {continue;}
                if ((bb.max.z()-bb.min.z())-3.).abs()>1e-5 || bb.max[u]-bb.min[u]<12. {continue;}
                if (ba.min.z()-bb.max.z()).abs()>1e-5 {continue;}
                let (sign,inner)=if (ba.min[u]-bb.min[u]).abs()<1e-5 {(-1.,ba.max[u])}
                    else if (ba.max[u]-bb.max[u]).abs()<1e-5 {(1.,ba.min[u])} else {continue;};
                let mut normal=Vec3d::new(0.,0.,-1.);normal[u]=-sign;
                candidate=Some(Plane3d{normal:vec::normalize(normal),distance:(-bb.max.z()-sign*inner)/2f64.sqrt()});
            }
            let Some(p)=candidate else {continue;}; println!("candidate lower {i},{j}");
            let visible:Vec<_>=[a,b].into_iter().flat_map(|b|b.faces()).filter(|f|f.material_name.as_str()!="skip").collect();
            let first=visible[0];
            if visible.iter().any(|f|f.paint()!=first.paint() || f.surface_material!=first.surface_material || f.uv_attributes!=first.uv_attributes || f.uv_coord_system!=first.uv_coord_system || f.primitive_uv!=first.primitive_uv || f.paint().is_some_and(|c|c.alpha()!=255)) {continue;}
            let Some(na)=repartition(a,b,&p,&world) else {continue;};
            let Some(nb)=repartition(b,a,&p.flip(),&world) else {continue;};
            if same(a,&na)&&same(b,&nb) {contacts+=1;continue;}
            if !equivalent(a,b,&na,&nb) {continue;}
            if csg::intersect(&na,&nb).is_some_and(|v|v.volume()>0.0001) {continue;}
            // Attribute transfer may not expose a previously internal helper plane.
            println!("miter entity={entity_index} brushes={i},{j} z={}..{} material={}",ba.min.z(),ba.max.z(),materials[i].as_deref().unwrap());
            brushes[i]=na;brushes[j]=nb;changes+=1;changed.insert((entity_index,i));changed.insert((entity_index,j));
        }}
        for (index,brush) in brushes.into_iter().enumerate() {if changed.contains(&(entity_index,index)) {entity.brushes[index].faces=brush.faces().to_vec();}}
    }
    let mut text=String::new();
    for line in source.lines().take_while(|l|!l.starts_with("// Format:")) {text.push_str(line);text.push('\n');}
    text.push_str(&write_map(&map));
    let check=read_map(&text,None).unwrap();assert!(check.warnings.is_empty());
    for b in check.entities.iter().flat_map(|e|&e.brushes) {assert!(Brush::from_faces(&world,b.faces.clone()).unwrap().volume()>0.);}
    std::fs::write(&output,text).unwrap();
    println!("DONE: {changes} joins, {} changed brushes, {contacts} already matching",changed.len());
}
