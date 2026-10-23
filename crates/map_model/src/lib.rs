//! The map document model for Heron Brush.
//!
//! Editable brushes, entities and their document tree.
//!
//! ## Scope
//!
//! Geometry and document operations are independent of the target game. Readers provide
//! interoperability with supported game formats.
//!
//! The map file format is the plain text `.map` a Quake-lineage compiler reads. Both UV
//! dialects are supported — `Standard`, which derives a face's UV axes from the
//! coordinate plane it is most nearly parallel to, and `Valve` 220, which stores them —
//! along with the optional Quake 2 surface flags. See [`map_format::MapFormat`].
#![forbid(unsafe_code)]

pub mod asset_fs;
pub mod brush;
pub mod brush_face;
pub mod csg;
pub mod def;
pub mod ent;
pub mod entity_model;
pub mod entity_properties;
pub mod entity_rotation;
pub mod face_to_patch;
pub mod fgd;
pub mod fillet;
pub mod fm_model;
pub mod glb_model;
pub mod ids;
pub mod io;
pub mod leak;
pub mod legacy_model_definition;
pub mod m8;
pub mod map_format;
pub mod material;
pub mod md2_model;
pub mod md3_model;
pub mod merge;
pub mod model_definition;
pub mod model_pbr;
pub mod node;
pub mod obj_model;
pub mod obj_to_brushes;
pub mod pak;
pub mod patch_edit;
pub mod patch_mesh;
pub mod pcx;
pub mod pick;
pub mod polyhedron;
pub mod portal;
pub mod shader;
pub mod shapes;
pub mod sprite;
pub mod stylized_text;
pub mod text;
pub mod uv;
pub mod uv_alignment;
pub mod voxel;
pub mod wal;

pub mod hollow;
