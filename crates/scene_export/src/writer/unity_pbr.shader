// Heron Brush built-in pipeline PBR variant with explicit face culling.
Shader "Heron Brush/Standard Two Sided"
{
 Properties
 {
  _Color("Color", Color) = (1,1,1,1)
  _MainTex("Albedo", 2D) = "white" {}
  _Cutoff("Alpha cutoff", Float) = 0.5
  _Metallic("Metallic", Range(0,1)) = 0
  _Glossiness("Smoothness", Range(0,1)) = 0.5
  _GlossMapScale("Smoothness map scale", Range(0,1)) = 1
  _MetallicGlossMap("Metallic and smoothness", 2D) = "white" {}
  _BumpMap("Normal", 2D) = "bump" {}
  _BumpScale("Normal scale", Float) = 1
  _OcclusionMap("Occlusion", 2D) = "white" {}
  _OcclusionStrength("Occlusion strength", Range(0,1)) = 1
  _EmissionMap("Emission", 2D) = "white" {}
  _EmissionColor("Emission color", Color) = (0,0,0,1)
  [HideInInspector] _Mode("Mode", Float) = 0
  [HideInInspector] _Cull("Cull", Float) = 0
  [HideInInspector] _SrcBlend("Source blend", Float) = 1
  [HideInInspector] _DstBlend("Destination blend", Float) = 0
  [HideInInspector] _ZWrite("Depth write", Float) = 1
 }
 SubShader
 {
  Tags { "RenderType"="Opaque" }
  Cull [_Cull]
  Blend [_SrcBlend] [_DstBlend]
  ZWrite [_ZWrite]
  CGPROGRAM
  #pragma target 3.0
  #pragma surface surf Standard fullforwardshadows addshadow keepalpha
  #pragma shader_feature_local _NORMALMAP
  #pragma shader_feature_local _METALLICGLOSSMAP
  #pragma shader_feature_local _EMISSION
  #pragma shader_feature_local _ALPHATEST_ON
  #pragma shader_feature_local _ALPHABLEND_ON
  #include "UnityStandardUtils.cginc"
  sampler2D _MainTex, _MetallicGlossMap, _BumpMap, _OcclusionMap, _EmissionMap;
  fixed4 _Color;
  half _Cutoff, _Metallic, _Glossiness, _GlossMapScale, _BumpScale, _OcclusionStrength;
  half4 _EmissionColor;
  struct Input { float2 uv_MainTex; float facing : VFACE; };
  void surf(Input input, inout SurfaceOutputStandard output)
  {
   fixed4 albedo = tex2D(_MainTex, input.uv_MainTex) * _Color;
   output.Albedo = albedo.rgb;
   output.Alpha = albedo.a;
   #ifdef _ALPHATEST_ON
    clip(albedo.a - _Cutoff);
   #endif
   #ifdef _METALLICGLOSSMAP
    half4 mask = tex2D(_MetallicGlossMap, input.uv_MainTex);
    output.Metallic = mask.r * _Metallic;
    output.Smoothness = mask.a * _GlossMapScale;
   #else
    output.Metallic = _Metallic;
    output.Smoothness = _Glossiness;
   #endif
   output.Normal = half3(0,0,1);
   #ifdef _NORMALMAP
    output.Normal = UnpackScaleNormal(tex2D(_BumpMap, input.uv_MainTex), _BumpScale);
   #endif
   // glTF reverses the shading normal on back faces of double-sided surfaces.
   output.Normal *= input.facing < 0 ? -1 : 1;
   output.Occlusion = lerp(1, tex2D(_OcclusionMap, input.uv_MainTex).g, _OcclusionStrength);
   #ifdef _EMISSION
    output.Emission = tex2D(_EmissionMap, input.uv_MainTex).rgb * _EmissionColor.rgb;
   #endif
  }
  ENDCG
 }
 Fallback Off
}
