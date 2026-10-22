//! Heron Brush without a user interface: the document, its history, the tools, exports,
//! plugins, collaboration and MCP. The application crate, `heron-brush`, builds on it.

// Many modules began inside the application binary, where `pub(crate)` items needed no
// docs; moving them here made them `pub` without making them documented API yet.
#![allow(missing_docs)]

pub mod animation_curves;
pub mod animation_drag2d;
pub mod animation_editor;
pub mod animation_timeline;
pub mod assemble_tool;
pub mod camera2d;
pub mod cli;
pub mod clip_tool;
pub mod console;
pub mod corner_join;
pub mod entity_models;
pub mod extrude_tool;
pub mod godot_native_animation;
pub mod grid;
pub mod handle_drag;
pub mod handle_tool;
pub mod history;
pub mod keyframe_animation;
pub mod live_mcp;
pub mod manual;
pub mod plugin_manifest;
pub mod plugin_state;
pub mod scale_tool;
pub mod shape_tool;
pub mod shear_tool;
pub mod size_labels;
pub mod sweep_tool;
pub mod tool;
pub mod world;
pub use render::camera3d;
pub mod animation_library;
pub mod animation_path;
pub mod animation_pose;
pub mod animation_trigger;
pub mod assets;
pub mod autosave;
pub mod builtin_entities;
pub mod canvas;
pub mod collab;
pub mod compass;
pub mod defold_live;
pub mod document;
pub mod document_setup;
pub mod entity_icons;
pub mod export;
pub mod export_bindings;
pub mod export_settings;
pub mod favourite_maps;
pub mod fillet_tool;
pub mod flax_live;
pub mod games;
pub mod godot_animation;
pub mod godot_live;
pub mod i18n;
pub mod mcp;
pub mod motion_preview;
pub mod obj_export;
pub mod plugins;
pub mod portable_animation;
pub mod preferences;
pub mod raylib_animation;
pub mod raylib_water;
pub mod recent_documents;
pub mod render_settings;
pub mod rotate_tool;
pub mod sculpt;
pub mod unity_live;
pub mod validation;
pub mod view_filters;
pub use file_store as files;
pub mod plugin;
pub mod samples;
pub mod selection_report;
pub use render as renderer;
pub mod animation_mode;
#[cfg(feature = "test-support")]
pub mod bench_support;
pub mod commands;
pub mod context_menu;
pub mod entity_panel;
pub mod entity_placement;
pub mod input;
pub mod keymap;
pub mod overlay;
pub mod profile;
pub mod render_camera;
pub mod render_scene;
pub mod uv_view;
pub mod view2d;
pub mod view3d;

pub mod build_profiles;

pub mod custom_shortcuts;

#[cfg(not(target_arch = "wasm32"))]
pub mod updates;

mod export_hash;
mod imported_export;
