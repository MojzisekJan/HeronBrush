// What the albedo texture gives at a texel, or -- while the texture is still being
// streamed in -- the material's average colour (sRGB bytes packed in factors.w).
fn albedo_texel(m: Material, sampled: vec4<f32>) -> vec4<f32> {
    if ((m.flags & MATERIAL_PENDING) == 0u) {
        return sampled;
    }
    let average = unpack4x8unorm(bitcast<u32>(m.factors.w));
    let c = average.rgb;
    let linear = select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
    return vec4<f32>(linear, average.a);
}
