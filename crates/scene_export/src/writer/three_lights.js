// Native lights are children of the entity root, so placement, animation and
// project parenting use the same transform as meshes. Directions are local -Z.
export function createHeronLight(THREE, entity) {
  const spec = entity.light;
  if (!spec) return null;
  const color = new THREE.Color().setRGB(...spec.color.slice(0, 3), THREE.SRGBColorSpace);
  let light;
  if (spec.kind === 'Point') light = new THREE.PointLight(color, spec.energy, spec.range);
  else if (spec.kind === 'Spot') light = new THREE.SpotLight(color, spec.energy, spec.range, spec.half_angle * Math.PI / 180);
  else if (spec.kind === 'Directional') light = new THREE.DirectionalLight(color, spec.energy);
  else return null;
  light.name = 'HeronLight';
  light.position.set(0, 0, 0);
  light.castShadow = spec.shadows;
  if (light.target) {
    const target = new THREE.Object3D();
    target.position.set(0, 0, -1);
    light.add(target);
    light.target = target;
  }
  return light;
}
