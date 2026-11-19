//! Simulator Metal lacks cube arrays. Keep the six faces in a 2D array and map
//! directions explicitly; physical devices and desktop retain hardware cube sampling.
const FALLBACK: bool = cfg!(any(
    all(target_os = "ios", target_abi = "sim"),
    feature = "cube-array-fallback"
));

pub(super) fn view_dimension() -> wgpu::TextureViewDimension {
    if FALLBACK {
        wgpu::TextureViewDimension::D2Array
    } else {
        wgpu::TextureViewDimension::CubeArray
    }
}

pub(super) fn shader_source(source: String) -> String {
    if !FALLBACK {
        return source;
    }
    let source = source
        .replace("texture_cube_array<f32>", "texture_2d_array<f32>")
        .replace("texture_depth_cube_array", "texture_depth_2d_array")
        .replace("textureSampleLevel(map,linear_sampler,probe_direction(probe,world,direction),i32(i),roughness*probe.params.z)",
            "sample_cube_array(map,linear_sampler,probe_direction(probe,world,direction),i32(i),roughness*probe.params.z)")
        .replace("textureSampleCompareLevel(light_shadows, shadow_sampler, dir + across * tap.x + up * tap.y, slot, reference)",
            "compare_cube_array(light_shadows, shadow_sampler, dir + across * tap.x + up * tap.y, slot, reference)");
    format!("{}\n{source}", include_str!("shaders/cube_array.wgsl"))
}
