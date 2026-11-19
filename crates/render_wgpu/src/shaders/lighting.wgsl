// Depth-aware visibility and octahedral bent normal, shared by forward lighting and SSR.
fn occlusion_at(map:texture_2d<f32>,pixel:vec2<f32>,depth:f32)->vec4<f32> {
    let size=vec2<i32>(textureDimensions(map));
    if (frame.ao.x<1.5) {return textureLoad(map,clamp(vec2<i32>(pixel),vec2<i32>(0),size-1),0);}
    let position=pixel/frame.ao.x-0.5;
    let base=vec2<i32>(floor(position));
    let fraction=fract(position);
    var visibility=0.0;var bent=vec3<f32>(0.0);var weights=0.0;
    for (var y=0;y<2;y+=1) {for (var x=0;x<2;x+=1) {
        let texel=textureLoad(map,clamp(base+vec2<i32>(x,y),vec2<i32>(0),size-1),0);
        let bilinear=select(1.0-fraction.x,fraction.x,x==1)*select(1.0-fraction.y,fraction.y,y==1);
        let weight=bilinear/(0.001+abs(texel.g-depth)/max(depth,1.0)*20.0)+0.00001;
        visibility+=texel.r*weight;bent+=octahedral_decode(texel.zw)*weight;weights+=weight;
    }}
    return vec4<f32>(visibility/weights,depth,octahedral_encode(normalize(bent)));
}
fn specular_cone_visibility(direction:vec3<f32>,bent:vec3<f32>,visibility:f32,roughness:f32)->f32 {
    // Approximate the visible hemisphere by a cone; broaden its edge for rough lobes.
    let horizon=sqrt(clamp(1.0-visibility,0.0,1.0));
    let edge=max(roughness*roughness,0.02);
    return mix(smoothstep(horizon-edge,horizon+edge,dot(direction,bent)),1.0,visibility*visibility);
}

fn probe_weight(probe:LocalProbe,world:vec3<f32>)->f32 {
    let relative=abs(world-probe.position.xyz)/probe.half_size.xyz;
    return 1.0-smoothstep(1.0-probe.half_size.w,1.0,max(relative.x,max(relative.y,relative.z)));
}
fn probe_direction(probe:LocalProbe,world:vec3<f32>,direction:vec3<f32>)->vec3<f32> {
    let safe=select(vec3<f32>(0.000001),direction,abs(direction)>vec3<f32>(0.000001));
    let lower=(probe.position.xyz-probe.half_size.xyz-world)/safe;
    let upper=(probe.position.xyz+probe.half_size.xyz-world)/safe;
    let exit=max(lower,upper);let distance=max(min(exit.x,min(exit.y,exit.z)),0.0);
    return world+direction*distance-probe.position.xyz;
}
fn local_specular(map:texture_cube_array<f32>,linear_sampler:sampler,world:vec3<f32>,direction:vec3<f32>,roughness:f32,global:vec3<f32>)->vec3<f32> {
    var total=vec3<f32>(0.0);var weights=0.0;
    for (var i=0u;i<u32(frame.local_probes.x);i+=1u) {
        let probe=probes[i];let weight=probe_weight(probe,world)*probe.params.x;
        if (weight>0.0) {total+=textureSampleLevel(map,linear_sampler,probe_direction(probe,world,direction),i32(i),roughness*probe.params.z).rgb*weight*probe.position.w;weights+=weight;}
    }
    return global*max(1.0-weights,0.0)+total/max(weights,1.0);
}
fn probe_irradiance(probe:LocalProbe,n:vec3<f32>)->vec3<f32> {
    var e=probe.sh[0].rgb*0.282095;
    e+=(2.0/3.0)*0.488603*(probe.sh[1].rgb*n.y+probe.sh[2].rgb*n.z+probe.sh[3].rgb*n.x);
    e+=0.25*(1.092548*(probe.sh[4].rgb*n.x*n.y+probe.sh[5].rgb*n.y*n.z+probe.sh[7].rgb*n.x*n.z)+0.315392*probe.sh[6].rgb*(3.0*n.z*n.z-1.0)+0.546274*probe.sh[8].rgb*(n.x*n.x-n.y*n.y));
    return max(e,vec3<f32>(0.0));
}
fn local_diffuse(world:vec3<f32>,normal:vec3<f32>,global:vec3<f32>)->vec3<f32> {
    var total=vec3<f32>(0.0);var weights=0.0;
    for (var i=0u;i<u32(frame.local_probes.x);i+=1u) {let probe=probes[i];let weight=probe_weight(probe,world)*probe.params.y;
        total+=probe_irradiance(probe,normal)*weight*probe.position.w;weights+=weight;
    }
    return global*max(1.0-weights,0.0)+total/max(weights,1.0);
}
