// Same face convention as lights::FACES and the environment cube: +X,-X,+Y,-Y,+Z,-Z.
// Unlike hardware cube sampling, bilinear taps clamp at face edges on this fallback.
fn cube_array_uv(direction:vec3<f32>)->vec3<f32> {
    let a=abs(direction);
    var face=0.0;
    var uv=vec2<f32>(0.0);
    var major=max(a.x,max(a.y,a.z));
    if (a.x>=a.y && a.x>=a.z) {
        if (direction.x>=0.0) {uv=vec2<f32>(-direction.z,-direction.y);}
        else {face=1.0;uv=vec2<f32>(direction.z,-direction.y);}
    } else if (a.y>=a.z) {
        if (direction.y>=0.0) {face=2.0;uv=vec2<f32>(direction.x,direction.z);}
        else {face=3.0;uv=vec2<f32>(direction.x,-direction.z);}
    } else {
        if (direction.z>=0.0) {face=4.0;uv=vec2<f32>(direction.x,-direction.y);}
        else {face=5.0;uv=vec2<f32>(-direction.x,-direction.y);}
    }
    return vec3<f32>(uv/max(major,0.000001)*0.5+0.5,face);
}
fn sample_cube_array(map:texture_2d_array<f32>,linear_sampler:sampler,direction:vec3<f32>,slot:i32,lod:f32)->vec4<f32> {
    let uv=cube_array_uv(direction);
    return textureSampleLevel(map,linear_sampler,uv.xy,slot*6+i32(uv.z),lod);
}
fn compare_cube_array(map:texture_depth_2d_array,comparison:sampler_comparison,direction:vec3<f32>,slot:i32,reference:f32)->f32 {
    let uv=cube_array_uv(direction);
    return textureSampleCompareLevel(map,comparison,uv.xy,slot*6+i32(uv.z),reference);
}
