using System;
using System.IO;
using Stride.Core.Yaml;
using Stride.Assets.Materials;
using Stride.Assets.Textures;
using Stride.Rendering.Materials;
using Stride.Rendering.Materials.ComputeColors;
class Program {
 static void Require(bool value, string message) { if (!value) throw new Exception(message); }
 static void Main(string[] args) {
  if (args.Length != 1) throw new ArgumentException("Pass the arena folder written by HERON_STRIDE_MATERIAL_OUTPUT");
  var folder = Path.GetFullPath(args[0]);
  using var stream = File.OpenRead(Path.Combine(folder, "Materials", "pbr.sdmat"));
  var m = (MaterialAsset)AssetYamlSerializer.Default.Deserialize(stream);
  Require(m.Attributes.Surface is MaterialNormalMapFeature, "normal feature");
  var normal = (MaterialNormalMapFeature)m.Attributes.Surface;
  Require(normal.NormalMap is ComputeBinaryColor && !normal.IsXYNormal, "normal scale graph");
  Require(m.Attributes.Emissive is MaterialEmissiveMapFeature, "emissive feature");
  var emission = (MaterialEmissiveMapFeature)m.Attributes.Emissive;
  Require(((ComputeFloat)emission.Intensity).Value == 3, "emissive strength");
  Require(m.Attributes.Transparency is MaterialTransparencyCutoffFeature, "cutoff feature");
  Require(Math.Abs(((ComputeFloat)((MaterialTransparencyCutoffFeature)m.Attributes.Transparency).Alpha).Value - 0.7f) < 0.0001, "cutoff value");
  Require(m.Attributes.Occlusion is MaterialOcclusionMapFeature, "AO feature");
  Require(((MaterialGlossinessMapFeature)m.Attributes.MicroSurface).Invert, "roughness inversion");
  var rough = (MaterialGlossinessMapFeature)m.Attributes.MicroSurface;
  var metal = (MaterialMetalnessMapFeature)m.Attributes.Specular;
  Require(rough.GlossinessMap is ComputeBinaryScalar && metal.MetalnessMap is ComputeBinaryScalar, "MR factor graphs");
  Require(((MaterialOcclusionMapFeature)m.Attributes.Occlusion).AmbientOcclusionMap is ComputeBinaryScalar, "AO strength graph");
  var count = 0;
  foreach (var path in Directory.GetFiles(Path.Combine(folder, "Textures"), "*.sdtex")) {
   using var input = File.OpenRead(path);
   var tex = (TextureAsset)AssetYamlSerializer.Default.Deserialize(input);
   count++;
   if (path.EndsWith(".normal.sdtex")) {
    Require(tex.Type is NormapMapTextureType n && n.InvertY, "normal Y convention");
    Require(!tex.IsCompressed, "normal Z preservation");
   }
   else {
    Require(tex.Type is ColorTextureType c && !c.PremultiplyAlpha, "straight alpha");
    if (path.EndsWith(".mr.sdtex") || path.EndsWith(".ao.sdtex"))
     Require(!((ColorTextureType)tex.Type).UseSRgbSampling, "linear PBR channels");
   }
   Console.WriteLine($"Loaded {Path.GetFileName(path)}: {tex.Type.GetType().Name}");
  }
  Require(count == 5, "all five texture slots");
  Console.WriteLine("PASS: Stride 4.4.1 loads native PBR material, normal scale graph, emission, mask, MR, AO and texture assets.");
 }
}
