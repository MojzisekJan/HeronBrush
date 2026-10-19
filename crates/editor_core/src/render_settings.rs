//! The desktop frontend's filling of [`render::RenderSettings`].
//!
//! The `render` crate draws by a plain struct and knows nothing of the preference
//! store; this is the one place that maps the store onto it. It is its own file rather
//! than a corner of `preferences.rs` so the audit in `preferences_wired` sees these
//! fields being read -- the store's owners are excluded from that count on purpose.

use crate::preferences::pref;

/// The renderer's settings as the store has them. A future frontend fills the struct
/// however it likes (docs/android.md, step 3).
pub fn from_preferences() -> render::RenderSettings {
    pref(|p| render::RenderSettings {
        depth_of_field: None,
        show_axes: p.show_axes,
        procedural_sky: p.procedural_sky,
        background_color: p.background_color.rgb_f32(),
        x_axis_color: p.x_axis_color.rgb_f32(),
        y_axis_color: p.y_axis_color.rgb_f32(),
        z_axis_color: p.z_axis_color.rgb_f32(),
        selected_face_color: p.selected_face_color.rgb_f32(),
        locked_face_color: p.locked_face_color.rgb_f32(),
        handle_radius: p.handle_radius,
        realistic_shading: p.realistic_shading,
        shadows: p.shadows,
        shadow_map_size: p.shadow_map_size,
        shadow_bias: p.shadow_bias,
        shadow_softness: p.shadow_softness,
        contact_shadows: p.contact_shadows,
        contact_shadow_distance: p.contact_shadow_distance,
        contact_shadow_samples: p.contact_shadow_samples,
        ambient_occlusion: p.ambient_occlusion,
        ao_radius: p.ao_radius,
        ao_strength: p.ao_strength,
        reflections: p.reflections,
        reflection_distance: p.reflection_distance,
        reflection_samples: p.reflection_samples,
        reflection_strength: p.reflection_strength,
        bloom: p.bloom,
        bloom_threshold: p.bloom_threshold,
        bloom_intensity: p.bloom_intensity,
        accumulate: p.accumulate,
        accumulation_samples: p.accumulation_samples,
        antialiasing: p.antialiasing,
        exposure: p.exposure,
        sky_color: p.sky_color.rgb_f32(),
        horizon_color: p.horizon_color.rgb_f32(),
        ground_color: p.ground_color.rgb_f32(),
        sun_elevation: p.sun_elevation,
        sun_azimuth: p.sun_azimuth,
        sun_color: p.sun_color.rgb_f32(),
        sun_intensity: p.sun_intensity,
        ambient_intensity: p.ambient_intensity,
        default_roughness: p.default_roughness,
        default_metalness: p.default_metalness,
    })
}

/// The tone map the wgpu 3D view uses. Not part of [`render::RenderSettings`]: the GL
/// renderer has only ACES, and that struct is what it draws by.
pub fn tone_mapper() -> crate::preferences::ToneMapper {
    pref(|p| p.tone_mapper)
}

/// How much GPU memory the wgpu 3D view's material textures may take, in bytes.
pub fn texture_memory_bytes() -> u64 {
    u64::from(pref(|p| p.texture_memory_mb).max(16)) << 20
}

/// Whether light entities light the wgpu 3D view. Not part of [`render::RenderSettings`]
/// for the same reason as [`tone_mapper`]: the GL renderer has no entity lights.
pub fn entity_lights() -> bool {
    pref(|p| p.entity_lights)
}
