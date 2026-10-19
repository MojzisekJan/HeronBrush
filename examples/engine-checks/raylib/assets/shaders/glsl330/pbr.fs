#version 330

// Metallic-roughness PBR for the exported glTF materials: fragment stage.
//
// Base colour (texture x factor), metallic and roughness factors, emission
// (factor x strength, times the emissive texture when there is one), alpha
// (opaque, mask or blend) and a tangent-space normal map in the glTF/OpenGL
// convention (+Y of the map points along -V). Light: one shadowed sun, a
// sky/horizon/ground ambient and up to four point lights, after the editor's
// own viewport (GGX, Smith, Schlick; Karis' analytic environment BRDF), then
// the editor's default AgX tone mapping and sRGB output.

in vec3 fragWorld;
in vec2 fragTexCoord;
in vec3 fragNormal;
in vec4 fragTangent;

uniform sampler2D texture0;    // base colour, sRGB (MATERIAL_MAP_ALBEDO)
uniform sampler2D normalMap;   // MATERIAL_MAP_NORMAL
uniform sampler2D emissiveMap; // MATERIAL_MAP_EMISSION, sRGB

uniform vec4 uBaseColor;   // linear factor
uniform vec2 uMetalRough;  // metallic, roughness
uniform vec3 uEmissive;    // linear, strength applied
uniform vec4 uMaterial;    // x normal scale (0: no map), y alpha mode, z cutoff, w emissive map

uniform vec3 uViewPos;
uniform vec3 uSunDir;      // the way the light travels
uniform vec3 uSunColor;    // linear, intensity applied
uniform vec3 uSky;         // ambient radiance, linear, intensity applied
uniform vec3 uHorizon;
uniform vec3 uGround;
uniform float uExposure;

uniform mat4 uLightVP;
uniform sampler2D uShadowMap;
uniform float uShadowRes;
uniform int uHasShadow;

#define MAXP 4
uniform int uPointCount;
uniform vec3 uPointPos[MAXP];
uniform vec3 uPointColor[MAXP];
uniform float uPointRange[MAXP];

out vec4 finalColor;

const float PI = 3.14159265;

vec3 sky(vec3 d)
{
    float up = clamp(d.y, -1.0, 1.0);
    vec3 above = mix(uHorizon, uSky, pow(max(up, 0.0), 0.45));
    vec3 below = mix(uHorizon, uGround, pow(max(-up, 0.0), 0.35));
    return (up >= 0.0) ? above : below;
}

float sunShadow(vec3 N, vec3 L)
{
    vec4 p = uLightVP*vec4(fragWorld, 1.0);
    vec3 q = p.xyz/p.w*0.5 + 0.5;
    if (q.z > 1.0 || min(q.x, q.y) < 0.0 || max(q.x, q.y) > 1.0) return 1.0;
    float bias = max(0.0025*(1.0 - dot(N, L)), 0.0006);
    float lit = 0.0;
    float t = 1.0/uShadowRes;
    for (int x = -1; x <= 1; x++)
        for (int y = -1; y <= 1; y++)
            lit += (q.z - bias > texture(uShadowMap, q.xy + vec2(x, y)*t).r) ? 0.0 : 1.0;
    return lit/9.0;
}

float ggx(float NoH, float a)
{
    float a2 = a*a;
    float d = NoH*NoH*(a2 - 1.0) + 1.0;
    return a2/(PI*d*d);
}

float smithVisibility(float NoV, float NoL, float a)
{
    float a2 = a*a;
    float v = NoL*sqrt(NoV*NoV*(1.0 - a2) + a2);
    float l = NoV*sqrt(NoL*NoL*(1.0 - a2) + a2);
    return 0.5/max(v + l, 1e-5);
}

vec3 schlick(float VoH, vec3 f0)
{
    return f0 + (1.0 - f0)*pow(1.0 - VoH, 5.0);
}

vec2 environmentBRDF(float NoV, float roughness)
{
    vec4 r = roughness*vec4(-1.0, -0.0275, -0.572, 0.022) + vec4(1.0, 0.0425, 1.04, -0.04);
    float a = min(r.x*r.x, exp2(-9.28*NoV))*r.x + r.y;
    return vec2(-1.04, 1.04)*a + r.zw;
}

vec3 brdf(vec3 N, vec3 V, vec3 L, vec3 diffuse, vec3 f0, float a)
{
    vec3 H = normalize(L + V);
    float NoL = max(dot(N, L), 0.0);
    float NoV = max(dot(N, V), 1e-4);
    vec3 F = schlick(max(dot(H, V), 0.0), f0);
    vec3 spec = ggx(max(dot(N, H), 0.0), a)*smithVisibility(NoV, NoL, a)*F;
    return ((1.0 - F)*diffuse/PI + spec)*NoL;
}

// AgX in Benjamin Wrensch's minimal polynomial fit, with the editor's look
// (power 1.2, saturation 1.1). Returns linear display light.
vec3 agx(vec3 color)
{
    mat3 inset = mat3(
        vec3(0.842479062253094, 0.0423282422610123, 0.0423756549057051),
        vec3(0.0784335999999992, 0.878468636469772, 0.0784336),
        vec3(0.0792237451477643, 0.0791661274605434, 0.879142973793104));
    mat3 outset = mat3(
        vec3(1.19687900512017, -0.0528968517574562, -0.0529716355144438),
        vec3(-0.0980208811401368, 1.15190312990417, -0.0980434501171241),
        vec3(-0.0990297440797205, -0.0989611768448433, 1.15107367264116));
    float minEv = -12.47393;
    float maxEv = 4.026069;
    vec3 x = inset*max(color, vec3(1e-10));
    x = clamp((log2(x) - minEv)/(maxEv - minEv), 0.0, 1.0);
    vec3 x2 = x*x;
    vec3 x4 = x2*x2;
    x = 15.5*x4*x2 - 40.14*x4*x + 31.96*x4 - 6.868*x2*x + 0.4298*x2 + 0.1191*x - 0.00232;
    x = pow(max(x, vec3(0.0)), vec3(1.2));
    float luma = dot(x, vec3(0.2126, 0.7152, 0.0722));
    x = outset*(luma + 1.1*(x - luma));
    return pow(max(x, vec3(0.0)), vec3(2.2));
}

// dp1/dp2 and duv1/duv2 are the screen derivatives of the position and UV,
// taken in uniform control flow by the caller.
vec3 mapNormal(vec3 N, vec3 tn, vec3 dp1, vec3 dp2, vec2 duv1, vec2 duv2)
{
    vec3 T;
    vec3 B;
    if (dot(fragTangent.xyz, fragTangent.xyz) > 1e-8) {
        T = normalize(fragTangent.xyz - N*dot(N, fragTangent.xyz));
        B = cross(N, T)*((fragTangent.w < 0.0) ? -1.0 : 1.0);
    } else {
        // No TANGENT attribute: a cotangent frame from screen derivatives.
        vec3 dp2perp = cross(dp2, N);
        vec3 dp1perp = cross(N, dp1);
        T = dp2perp*duv1.x + dp1perp*duv2.x;
        B = -(dp2perp*duv1.y + dp1perp*duv2.y); // +Y of the map is -V
        float s = inversesqrt(max(max(dot(T, T), dot(B, B)), 1e-20));
        T *= s;
        B *= s;
    }
    return normalize(T*tn.x + B*tn.y + N*tn.z);
}

void main()
{
    // Everything that needs derivatives first, before any branch or discard.
    vec4 texel = texture(texture0, fragTexCoord);
    vec3 tn = texture(normalMap, fragTexCoord).xyz*2.0 - 1.0;
    vec3 glow = texture(emissiveMap, fragTexCoord).rgb;
    vec3 dp1 = dFdx(fragWorld);
    vec3 dp2 = dFdy(fragWorld);
    vec2 duv1 = dFdx(fragTexCoord);
    vec2 duv2 = dFdy(fragTexCoord);

    vec4 base = vec4(pow(texel.rgb, vec3(2.2)), texel.a)*uBaseColor;
    if (uMaterial.y > 0.5 && uMaterial.y < 1.5 && base.a < uMaterial.z) discard;

    vec3 N = normalize(fragNormal);
    if (!gl_FrontFacing) N = -N; // double-sided materials draw their back faces
    vec3 geometric = N;
    if (uMaterial.x > 0.0)
        N = mapNormal(N, normalize(vec3(tn.xy*uMaterial.x, tn.z)), dp1, dp2, duv1, duv2);

    float metallic = clamp(uMetalRough.x, 0.0, 1.0);
    float roughness = clamp(uMetalRough.y, 0.05, 1.0);
    float a = roughness*roughness;
    vec3 diffuse = base.rgb*(1.0 - metallic);
    vec3 f0 = mix(vec3(0.04), base.rgb, metallic);
    vec3 V = normalize(uViewPos - fragWorld);
    vec3 L = normalize(-uSunDir);

    float shadow = (uHasShadow == 1) ? sunShadow(geometric, L) : 1.0;
    vec3 color = brdf(N, V, L, diffuse, f0, a)*uSunColor*shadow;

    for (int i = 0; i < MAXP; i++) {
        if (i >= uPointCount) break;
        vec3 d = uPointPos[i] - fragWorld;
        float dist = length(d);
        float fall = clamp(1.0 - dist/uPointRange[i], 0.0, 1.0);
        color += brdf(N, V, d/max(dist, 1e-3), diffuse, f0, a)*uPointColor[i]*fall*fall;
    }

    vec2 env = environmentBRDF(max(dot(N, V), 1e-4), roughness);
    vec3 R = reflect(-V, N);
    color += diffuse*sky(N) + mix(sky(R), sky(N), roughness)*(f0*env.x + env.y);

    vec3 emissive = uEmissive;
    if (uMaterial.w > 0.5) emissive *= pow(glow, vec3(2.2));
    color += emissive;

    float alpha = (uMaterial.y > 1.5) ? base.a : 1.0;
    finalColor = vec4(pow(agx(color*uExposure*1.8), vec3(1.0/2.2)), alpha);
}
