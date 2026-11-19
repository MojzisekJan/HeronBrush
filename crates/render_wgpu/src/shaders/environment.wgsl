// Three-band sky, as the GL renderer's `sky()`: ground, horizon haze, zenith, plus the
// sun's glint when `sun_weight` > 0.
fn sky_light(direction: vec3<f32>, roughness: f32, sun_weight: f32) -> vec3<f32> {
    let up = clamp(direction.z, -1.0, 1.0);
    let above = mix(frame.horizon.rgb, frame.sky.rgb, pow(clamp(up, 0.0, 1.0), 0.45));
    let below = mix(frame.horizon.rgb, frame.ground.rgb, pow(clamp(-up, 0.0, 1.0), 0.35));
    var result = select(below, above, up >= 0.0);
    if (sun_weight > 0.0) {
        let to_sun = normalize(-frame.light_dir.xyz);
        let alignment = clamp(dot(direction, to_sun), 0.0, 1.0);
        let sharpness = mix(2000.0, 8.0, clamp(roughness, 0.0, 1.0));
        result += frame.light_color.rgb * pow(alignment, sharpness) * (1.0 - roughness * 0.6) * sun_weight;
    }
    return result;
}

// The procedural sky as seen, without the sun's disc: zenith, horizon haze warmed
// towards the sun at sunset, ground, dimmed at night, and the glow round the sun. What
// the view shows behind the geometry and what the environment map is made from.
fn sky_radiance(direction: vec3<f32>) -> vec3<f32> {
    let up = clamp(direction.z, -1.0, 1.0);
    let sun = normalize(-frame.light_dir.xyz);
    let sun_height = sun.z;
    let daylight = smoothstep(-0.12, 0.08, sun_height);
    let sunset = (1.0 - smoothstep(0.04, 0.4, abs(sun_height))) * daylight;
    let facing = pow(max(dot(direction.xy, sun.xy), 0.0), 3.0);
    let horizon = mix(frame.horizon.rgb, vec3<f32>(1.0, 0.40, 0.12), sunset * facing * 0.55);
    let zenith = mix(frame.sky.rgb, frame.sky.rgb * vec3<f32>(0.65, 0.55, 0.85), sunset * 0.3);
    let above = mix(horizon, zenith, pow(max(up, 0.0), 0.38));
    let below = mix(horizon, frame.ground.rgb, pow(max(-up, 0.0), 0.35));
    var color = select(below, above, up >= 0.0) * mix(0.055, 1.0, daylight);
    let angle = acos(clamp(dot(direction, sun), -1.0, 1.0));
    let halo = 0.65 * exp(-angle * 18.0) + 0.12 * exp(-angle * 3.0);
    let warmth = mix(vec3<f32>(1.0), vec3<f32>(1.0, 0.48, 0.16), sunset);
    color += frame.sun_color.rgb * warmth * halo * smoothstep(-0.12, 0.06, up) * smoothstep(-0.06, 0.0, sun_height + radians(0.75));
    return color;
}

// Full-resolution scene-linear panorama; wrap longitude and clamp at the poles.
fn panorama_radiance(environment_map: texture_2d<f32>, direction: vec3<f32>) -> vec3<f32> {
    let angle=atan2(direction.y,direction.x)-frame.environment.z;
    let uv=vec2<f32>(fract(angle/(2.0*PI)+0.5),acos(clamp(direction.z,-1.0,1.0))/PI);
    let size=vec2<i32>(textureDimensions(environment_map));
    let position=vec2<f32>(uv.x*f32(size.x)-0.5,clamp(uv.y*f32(size.y)-0.5,0.0,f32(size.y-1)));
    let base=vec2<i32>(floor(position));
    let weight=fract(position);
    let x0=((base.x%size.x)+size.x)%size.x;
    let x1=(x0+1)%size.x;
    let y0=clamp(base.y,0,size.y-1);
    let y1=min(y0+1,size.y-1);
    let a=mix(textureLoad(environment_map,vec2<i32>(x0,y0),0).rgb,textureLoad(environment_map,vec2<i32>(x1,y0),0).rgb,weight.x);
    let b=mix(textureLoad(environment_map,vec2<i32>(x0,y1),0).rgb,textureLoad(environment_map,vec2<i32>(x1,y1),0).rgb,weight.x);
    return min(mix(a,b,weight.y)*frame.environment.y,vec3<f32>(65504.0));
}
