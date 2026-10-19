//! Manual WASM allocation regression check. See scripts/tests/check_web_archive_sharing.cjs.
#![cfg(target_arch = "wasm32")]
use map_model::asset_fs::AssetFileSystem;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};
use wasm_bindgen::prelude::*;

thread_local! {
    static MOUNT: RefCell<Option<AssetFileSystem>> = const { RefCell::new(None) };
    static COPY: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
}

// Fixtures contain a small readable member and a large unused gap. The gap tests
// archive retention, not member inflation or many-resource indexing performance.
/// Stores a generated archive before the mount measurement starts.
#[wasm_bindgen]
pub fn prepare(zip: bool, mebibytes: u32) {
    assert!((16..=128).contains(&mebibytes));
    let size = mebibytes as usize * 1024 * 1024;
    let mut bytes = vec![0; size];
    if zip {
        bytes[..4].copy_from_slice(&0x04034b50u32.to_le_bytes());
        bytes[4..6].copy_from_slice(&20u16.to_le_bytes());
        let crc = crc32fast::hash(b"probe");
        bytes[14..18].copy_from_slice(&crc.to_le_bytes());
        bytes[18..22].copy_from_slice(&5u32.to_le_bytes());
        bytes[22..26].copy_from_slice(&5u32.to_le_bytes());
        bytes[26..28].copy_from_slice(&4u16.to_le_bytes());
        bytes[30..34].copy_from_slice(b"item");
        bytes[34..39].copy_from_slice(b"probe");
        let directory = size - 72;
        let d = &mut bytes[directory..];
        d[..4].copy_from_slice(&0x02014b50u32.to_le_bytes());
        d[4..6].copy_from_slice(&20u16.to_le_bytes());
        d[6..8].copy_from_slice(&20u16.to_le_bytes());
        d[16..20].copy_from_slice(&crc.to_le_bytes());
        d[20..24].copy_from_slice(&5u32.to_le_bytes());
        d[24..28].copy_from_slice(&5u32.to_le_bytes());
        d[28..30].copy_from_slice(&4u16.to_le_bytes());
        d[46..50].copy_from_slice(b"item");
        d[50..54].copy_from_slice(&0x06054b50u32.to_le_bytes());
        d[58..60].copy_from_slice(&1u16.to_le_bytes());
        d[60..62].copy_from_slice(&1u16.to_le_bytes());
        d[62..66].copy_from_slice(&50u32.to_le_bytes());
        d[66..70].copy_from_slice(&(directory as u32).to_le_bytes());
    } else {
        bytes[..4].copy_from_slice(b"PACK");
        bytes[4..8].copy_from_slice(&((size - 64) as u32).to_le_bytes());
        bytes[8..12].copy_from_slice(&64u32.to_le_bytes());
        bytes[12..17].copy_from_slice(b"probe");
        let d = &mut bytes[size - 64..];
        d[..4].copy_from_slice(b"item");
        d[56..60].copy_from_slice(&12u32.to_le_bytes());
        d[60..64].copy_from_slice(&5u32.to_le_bytes());
    }
    file_store::store(PathBuf::from(path(zip)), bytes);
}
fn path(zip: bool) -> &'static str {
    if zip {
        "/bench/archive.pk3"
    } else {
        "/bench/archive.pak"
    }
}

/// Retains the mounted index and optionally a deliberate full-copy control.
#[wasm_bindgen]
pub fn mount(zip: bool, copy_control: bool) {
    if copy_control {
        COPY.with(|slot| {
            *slot.borrow_mut() = Some(file_store::read(Path::new(path(zip))).unwrap())
        });
    }
    let fs = AssetFileSystem::mount(&[PathBuf::from("/bench")]);
    assert!(fs.warnings().is_empty(), "{:?}", fs.warnings());
    assert_eq!(fs.resolve("item").unwrap().read().unwrap(), b"probe");
    MOUNT.with(|slot| *slot.borrow_mut() = Some(fs));
}

/// Current committed WASM linear-memory size, not live allocation size.
#[wasm_bindgen]
pub fn memory_bytes() -> u32 {
    (core::arch::wasm32::memory_size::<0>() * 65536) as u32
}
