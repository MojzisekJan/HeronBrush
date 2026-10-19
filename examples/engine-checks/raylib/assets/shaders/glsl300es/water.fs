#version 300 es

#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

// A stylized water surface. Not the kit's screen-space shader (that wants a
// depth and colour buffer raylib does not hand a DrawMesh call); this is
// world-space wave normals, fresnel, a sun glint and a two-tone tint.

in vec2 fragTexCoord;
in vec3 vWorld;
in vec3 vNormal;

uniform sampler2D texture0;
uniform float uTime;
uniform vec3 uCam;
uniform vec3 uSun; // the way the sunlight travels

out vec4 finalColor;

float wave(vec2 p, float t)
{
    return sin(p.x*1.7 + t*1.2)*0.5 + sin(p.y*1.9 - t*0.9)*0.5
         + sin((p.x + p.y)*2.7 + t*1.9)*0.25 + sin((p.x - p.y)*3.9 - t*2.4)*0.15;
}

void main()
{
    float h = wave(vWorld.xz, uTime);
    vec3 tx = texture(texture0, fragTexCoord*0.25 + vec2(uTime*0.02, uTime*0.015)).rgb;
    vec3 N = normalize(vNormal + vec3(dFdx(h), 0.0, dFdy(h))*2.2);
    vec3 V = normalize(uCam - vWorld);
    float fres = pow(1.0 - clamp(dot(N, V), 0.0, 1.0), 3.0);
    vec3 shallow = vec3(0.32, 0.72, 0.74);
    vec3 deep = vec3(0.02, 0.24, 0.30);
    float look = clamp(abs(V.y), 0.0, 1.0);
    vec3 base = mix(deep, shallow, 0.15 + look*0.6);
    base *= 0.85 + 0.3*tx.r;
    vec3 L = normalize(-uSun);
    vec3 H = normalize(L + V);
    float spec = pow(max(dot(N, H), 0.0), 120.0);
    vec3 col = base + fres*0.35 + spec*vec3(1.0, 0.97, 0.85);
    float a = clamp(mix(0.55, 0.92, fres) + spec, 0.0, 0.96);
    finalColor = vec4(col, a);
}
