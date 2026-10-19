//! Project-owned class defaults and build rules. Authored map properties win.
use std::collections::BTreeMap;
use scene_export::{fgd::{EntityClass,ClassKind,Property},indexmap::IndexMap,map::MapFile};

#[derive(Debug,Clone,Default,PartialEq,serde::Serialize,serde::Deserialize)]
#[serde(default,deny_unknown_fields)]
pub struct Rule {
    pub build:BTreeMap<String,String>,
    pub defaults:BTreeMap<String,serde_json::Value>,
}

pub fn classes<'a>(map:&MapFile,classes:&'a IndexMap<String,EntityClass>,rules:&BTreeMap<String,Rule>)->Result<std::borrow::Cow<'a,IndexMap<String,EntityClass>>,String>{
    if rules.is_empty(){return Ok(std::borrow::Cow::Borrowed(classes))}
    let mut mapped=classes.clone();apply(map,&mut mapped,rules)?;Ok(std::borrow::Cow::Owned(mapped))
}
pub fn validate(rules:&BTreeMap<String,Rule>)->Result<(),String>{
    apply(&MapFile{entities:vec![],groups:vec![],warnings:vec![]},&mut IndexMap::new(),rules)
}

pub fn apply(map:&MapFile,classes:&mut IndexMap<String,EntityClass>,rules:&BTreeMap<String,Rule>)->Result<(),String>{
    for (name,rule) in rules {
        if name.is_empty(){return Err("Entity binding classname cannot be empty".into())}
        let solid=map.entities.iter().any(|entity|entity.get("classname")==Some(name.as_str()) && (!entity.brushes.is_empty() || !entity.patches.is_empty()));
        let class=classes.entry(name.clone()).or_insert_with(||EntityClass{kind:if solid{ClassKind::Solid}else{ClassKind::Point},classname:name.clone(),description:String::new(),bases:vec![],size:None,color:None,model:None,properties:Default::default(),directives:Default::default()});
        for (key,value) in &rule.build {
            if !matches!(key.as_str(),"node"|"scene"|"script"|"collision"|"origin"|"spawn"|"visuals"|"occluder"|"auto"|"name_property"|"groups"|"link_property"|"link_name_property"|"connect"|"scene_property") {
                return Err(format!("Entity binding {name}: unknown build rule {key}"))
            }
            use scene_export::settings::{CollisionKind,OriginKind,SpawnKind};
            let valid=match key.as_str(){
                "collision"=>CollisionKind::parse(value).is_some(),
                "origin"=>OriginKind::parse(value).is_some(),
                "spawn"=>SpawnKind::parse(value).is_some(),
                "visuals"|"occluder"|"auto"=>matches!(value.trim().to_ascii_lowercase().as_str(),"true"|"false"|"1"|"0"|"yes"|"no"|"on"|"off"),
                "connect"=>value.split(',').all(|pair|pair.split_once(':').is_some_and(|(signal,method)|!signal.trim().is_empty()&&!method.trim().is_empty())),
                _=>!value.trim().is_empty(),
            };
            if !valid{return Err(format!("Entity binding {name}: invalid {key} value {value:?}"))}
            let value=if matches!(key.as_str(),"visuals"|"occluder"|"auto"){value.trim().to_ascii_lowercase()}else{value.clone()};
            class.directives.entries.insert(key.clone(),value);
        }
        for (key,value) in &rule.defaults {
            if key.is_empty() || key=="classname" {return Err(format!("Entity binding {name}: invalid default property {key}"))}
            fn supported(value:&serde_json::Value)->bool {match value {serde_json::Value::Array(values)=>values.iter().all(supported),serde_json::Value::String(_)|serde_json::Value::Number(_)|serde_json::Value::Bool(_)=>true,_=>false}}
            if !supported(value){return Err(format!("Entity binding {name}.{key}: object/null defaults are not native property values; use a JSON string for project-owned structured data"))}
            let (ty,raw)=match value {
                serde_json::Value::String(text)=>("string",text.clone()),
                serde_json::Value::Bool(value)=>("boolean",value.to_string()),
                serde_json::Value::Number(value)=>(if value.is_i64(){"integer"}else{"float"},value.to_string()),
                serde_json::Value::Array(_)=>("json",value.to_string()),
                _=>return Err(format!("Entity binding {name}.{key}: default must be a string, number, boolean or array")),
            };
            let property=class.properties.entry(key.clone()).or_insert_with(||Property{name:key.clone(),ty:ty.into(),label:key.clone(),description:String::new(),default:None,choices:vec![],flags:vec![]});
            property.default=Some(raw);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_typos_and_non_native_defaults_are_reported() {
        for build in [serde_json::json!({"collision":"conevx"}),serde_json::json!({"origin":"unknown"}),serde_json::json!({"spawn":"unknown"}),serde_json::json!({"visuals":"maybe"}),serde_json::json!({"connect":"body_entered"})] {
            let rules=serde_json::from_value(serde_json::json!({"puzzle":{"build":build}})).unwrap();
            assert!(validate(&rules).is_err());
        }
        let rules=serde_json::from_value(serde_json::json!({"puzzle":{"defaults":{"data":{"x":1}}}})).unwrap();
        assert!(validate(&rules).unwrap_err().contains("object/null"));
    }
    #[test]
    fn typed_defaults_and_boolean_rules_reach_class_definitions() {
        let rules=serde_json::from_value(serde_json::json!({"puzzle":{"build":{"visuals":" FALSE ","collision":"convex"},"defaults":{"armed":true,"charge":9,"sequence":[2,3,5]}}})).unwrap();
        let mut classes=IndexMap::new();
        apply(&MapFile{entities:vec![],groups:vec![],warnings:vec![]},&mut classes,&rules).unwrap();
        let class=&classes["puzzle"];
        assert_eq!(class.directives.bool("visuals"),Some(false));
        assert_eq!(class.properties["armed"].ty,"boolean");
        assert_eq!(class.properties["charge"].ty,"integer");
        assert_eq!(class.properties["sequence"].default.as_deref(),Some("[2,3,5]"));
    }
}
