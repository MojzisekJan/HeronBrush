// Delta transport for the generated export. The full snapshot remains available for resync.
export function unpack(html) {
  const match=html.match(/(<script id="level-data" type="application\/json">)(.*?)(<\/script>)/s);
  if(!match) throw new Error('Missing playtest export data');
  const data=JSON.parse(match[2]);
  return {data, template:html.slice(0,match.index)+match[1]+'__HERON_DATA__'+match[3]+html.slice(match.index+match[0].length)};
}
export function pack(template,data) {return template.replace('__HERON_DATA__',JSON.stringify(data).replaceAll('<','\\u003c'));}
function changed(before,after) {return Object.fromEntries(Object.entries(after).filter(([key,value])=>before[key]!==value));}
export function delta(before,after) {
  const old=new Map(before.level.entities.map(entity=>[entity.name,entity]));
  const {entities,...level}=after.level;
  return {level,entities:entities.flatMap(entity=> {
      const previous=old.get(entity.name)||{};
      const fields=Object.fromEntries(Object.entries(entity).filter(([key,value])=>JSON.stringify(previous[key])!==JSON.stringify(value)));
      const remove=Object.keys(previous).filter(key=>!(key in entity));
      return Object.keys(fields).length || remove.length ? [{name:entity.name,fields,remove}] : [];
    }),entityOrder:entities.map(entity=>entity.name),
    meshes:changed(before.meshes,after.meshes), textures:changed(before.textures,after.textures),
    removeMeshes:Object.keys(before.meshes).filter(key=>!(key in after.meshes)),removeTextures:Object.keys(before.textures).filter(key=>!(key in after.textures))};
}
export function apply(before,patch) {
  const meshes={...before.meshes,...patch.meshes},textures={...before.textures,...patch.textures};
  for(const key of patch.removeMeshes) delete meshes[key];
  for(const key of patch.removeTextures) delete textures[key];
  const entities=new Map(before.level.entities.map(entity=>[entity.name,entity]));
  for(const change of patch.entities) {const entity={...entities.get(change.name),...change.fields};for(const key of change.remove) delete entity[key];entities.set(change.name,entity);}
  const ordered=patch.entityOrder.map(name=> {if(!entities.has(name)) throw new Error('Missing playtest entity: '+name);return entities.get(name);});
  return {level:{...patch.level,entities:ordered},meshes,textures};
}

// Compare metadata and entity order only. Embedded meshes and collision arrays
// must not be serialized just to discover that a patch changes nothing.
export function unchanged(before,patch) {
  if(patch.entities.length || Object.keys(patch.meshes).length || Object.keys(patch.textures).length
    || patch.removeMeshes.length || patch.removeTextures.length) return false;
  if(before.level.entities.length!==patch.entityOrder.length
    || before.level.entities.some((entity,index)=>entity.name!==patch.entityOrder[index])) return false;
  const {entities,...level}=before.level;
  return JSON.stringify(level)===JSON.stringify(patch.level);
}
