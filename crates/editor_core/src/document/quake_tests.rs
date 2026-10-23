//! What a Quake map needs of worldspawn: its own keys, and the wads named in `wad`.
use super::*;

/// A one-texture WAD3 written to `path`; WAD3 carries its palette, so no game is needed.
pub(crate) fn write_wad(path: &Path, name: &str) {
    let (width, height) = (8u32, 8u32);
    let mut raw = [0u8; 16];
    raw[..name.len()].copy_from_slice(name.as_bytes());
    let mut mip: Vec<u8> = raw.to_vec();
    mip.extend_from_slice(&width.to_le_bytes());
    mip.extend_from_slice(&height.to_le_bytes());
    let mut at = 40u32;
    for level in 0..4u32 {
        mip.extend_from_slice(&at.to_le_bytes());
        at += (width >> level) * (height >> level);
    }
    for level in 0..4u32 {
        mip.extend(std::iter::repeat_n(
            1u8,
            ((width * height) >> (2 * level)) as usize,
        ));
    }
    mip.extend_from_slice(&256u16.to_le_bytes());
    mip.extend(std::iter::repeat_n(90u8, 768));

    let mut out = b"WAD3".to_vec();
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&((12 + mip.len()) as u32).to_le_bytes());
    out.extend_from_slice(&mip);
    out.extend_from_slice(&12u32.to_le_bytes());
    out.extend_from_slice(&(mip.len() as u32).to_le_bytes());
    out.extend_from_slice(&(mip.len() as u32).to_le_bytes());
    out.extend_from_slice(&[b'C', 0, 0, 0]);
    out.extend_from_slice(&raw);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("make the folder");
    std::fs::write(path, out).expect("write the wad");
}

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("heron-quake-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("scratch folder");
    directory
}

fn world_value(document: &Document, key: &str) -> Option<String> {
    document.worldspawn().property(key).map(str::to_string)
}

#[test]
fn worldspawn_keys_are_set_through_the_entity_calls_and_undone() {
    let mut document = Document::new(Default::default());
    let world = [Document::WORLDSPAWN];
    assert!(document.set_entity_property_on(&world, "message", "The lift"));
    assert!(document.set_entity_property_on(&world, "worldtype", "2"));
    assert_eq!(
        world_value(&document, "message").as_deref(),
        Some("The lift")
    );
    assert_eq!(
        document.worldspawn().classname(),
        Some("worldspawn"),
        "the classname comes first and is not one of the keys an edit can lose"
    );
    let text = document.to_map_text();
    assert!(text.contains("\"message\" \"The lift\""), "{text}");
    assert_eq!(text.matches("\"classname\" \"worldspawn\"").count(), 1);

    document.undo();
    assert_eq!(world_value(&document, "worldtype"), None);
    assert!(document.rename_entity_property_on(&world, "message", "_message"));
    assert_eq!(
        world_value(&document, "_message").as_deref(),
        Some("The lift")
    );
    assert!(document.remove_entity_property_on(&world, "_message"));
    assert_eq!(world_value(&document, "_message"), None);
    document.undo();
    document.undo();
    assert_eq!(
        world_value(&document, "message").as_deref(),
        Some("The lift")
    );
    // Setting what is already there is no edit at all.
    let undo = document.history.undo_len();
    assert!(!document.set_entity_property_on(&world, "message", "The lift"));
    assert_eq!(document.history.undo_len(), undo);
}

#[test]
fn a_read_maps_classname_stays_where_it_was() {
    let directory = scratch("classname");
    let path = directory.join("read.map");
    std::fs::write(
        &path,
        "{\n\"mapversion\" \"220\"\n\"classname\" \"worldspawn\"\n\"wad\" \"a.wad\"\n}\n",
    )
    .expect("write the map");
    let mut document = Document::open(&path, Default::default()).expect("open");
    document.set_entity_property_on(&[Document::WORLDSPAWN], "message", "Hi");
    let keys: Vec<&str> = document
        .tree
        .world
        .properties
        .iter()
        .map(|(key, _)| key.as_str())
        .collect();
    assert_eq!(keys, ["mapversion", "classname", "wad", "message"]);
}

#[test]
fn a_wad_added_to_a_saved_map_is_named_relative_to_it_once() {
    let directory = scratch("add-wad");
    let wad = directory.join("wads").join("bricks.wad");
    write_wad(&wad, "brick1");
    let map = directory.join("level.map");
    let mut document = Document::new(Default::default());
    document.save_as(&map).expect("save");

    let warnings = document.add_wad_to_map(&wad).expect("a wad");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(document.materials().get("brick1").is_some());
    assert_eq!(
        world_value(&document, "wad").as_deref(),
        Some("wads/bricks.wad"),
        "qbsp resolves a wad against the map's folder"
    );
    document
        .add_wad_to_map(&wad)
        .expect("the same wad again is fine");
    assert_eq!(
        world_value(&document, "wad").as_deref(),
        Some("wads/bricks.wad")
    );
    document.undo();
    assert_eq!(world_value(&document, "wad"), None);
}

#[test]
fn a_wad_named_relative_to_the_map_loads_once_when_it_opens() {
    let directory = scratch("open-wad");
    write_wad(&directory.join("wads").join("bricks.wad"), "brick1");
    let map = directory.join("maps").join("level.map");
    std::fs::create_dir_all(map.parent().expect("a folder")).expect("make the folder");
    std::fs::write(
        &map,
        "{\n\"classname\" \"worldspawn\"\n\"wad\" \"../wads/bricks.wad\"\n}\n",
    )
    .expect("write the map");
    let mut document = Document::open(&map, Default::default()).expect("open");
    let named = document.wads_named_by_the_map();
    assert_eq!(
        named.len(),
        1,
        "found beside the map, not in the working directory"
    );
    let loaded = document.load_named_wads();
    assert!(
        loaded.iter().all(|(_, result)| result.is_ok()),
        "{loaded:?}"
    );
    assert!(document.materials().get("brick1").is_some());
    assert!(
        document.load_named_wads().is_empty(),
        "a loaded wad is not read twice"
    );
    let packages = document
        .material_sources()
        .iter()
        .filter(|source| matches!(source, MaterialSource::Package(_)))
        .count();
    assert_eq!(packages, 1);
}
