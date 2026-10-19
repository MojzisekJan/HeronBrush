Shader "HeronBrush/Water" {
 Properties { _MainTex("Albedo",2D)="white" {} _Color("Tint",Color)=(.05,.38,.48,.55) _WaveHeight("Wave height",Float)=.035 _WaveSpeed("Wave speed",Float)=1.5 }
 SubShader {
  Tags { "Queue"="Transparent" "RenderType"="Transparent" }
  Blend SrcAlpha OneMinusSrcAlpha
  ZWrite Off
  Cull Off
  Pass {
   CGPROGRAM
   #pragma vertex vert
   #pragma fragment frag
   #pragma multi_compile_fog
   #include "UnityCG.cginc"
   sampler2D _MainTex;float4 _MainTex_ST,_Color;float _WaveHeight,_WaveSpeed;
   struct appdata {float4 vertex:POSITION;float3 normal:NORMAL;float2 uv:TEXCOORD0;};
   struct v2f {float4 pos:SV_POSITION;float2 uv:TEXCOORD0;UNITY_FOG_COORDS(1)};
   v2f vert(appdata v){v2f o;v.vertex.y+=smoothstep(.5,.9,v.normal.y)*_WaveHeight*sin(v.vertex.x*1.7+_Time.y*_WaveSpeed)*cos(v.vertex.z*1.3-_Time.y*_WaveSpeed*.8);o.pos=UnityObjectToClipPos(v.vertex);o.uv=TRANSFORM_TEX(v.uv,_MainTex);UNITY_TRANSFER_FOG(o,o.pos);return o;}
   fixed4 frag(v2f i):SV_Target {fixed4 c=tex2D(_MainTex,i.uv)*_Color;UNITY_APPLY_FOG(i.fogCoord,c);return c;}
   ENDCG
  }
 }
}
