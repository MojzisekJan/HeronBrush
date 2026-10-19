//! Building engine-ready levels out of a map: geometry, the optimisations, and one
//! writer per engine.
//!
//! Originally brought in from a separate project by the same author for
//! Godot and a handful of other engines, and is MIT licensed. The code is kept close to
//! its source rather than rewritten in this editor's style, so that a fix on either side
//! can be carried to the other by reading a diff. That is also why `missing_docs` is
//! allowed here and nowhere else in this workspace: documenting three hundred items to
//! this project's standard would make the two copies impossible to compare.
//!
//! The editor reaches it through `.map` text -- the format both sides already speak --
//! rather than by sharing types: the document is written out as a map and parsed here.
//! One format to keep in step instead of two sets of structures.

#![allow(
    missing_docs,
    reason = "vendored export code; see CREDITS.md and the module comment"
)]

/// The map type the builder's class table is keyed with, so a caller can name it without
/// depending on the same version of the crate.
pub use indexmap;

pub mod builder;
pub mod classdb;
pub mod fgd;
pub mod gameconfig;
pub mod geometry;
pub mod hidden;
pub mod level;
pub mod instancing;
pub mod map;
pub mod materials;
pub mod math;
pub mod merge;
pub mod overlap;
pub mod scene;
pub mod settings;
pub mod wad;
pub mod water;
pub mod writer;

/// Encode export assets without data-encoding's approximately 8 MiB wasm32
/// single-input limit. Complete Base64 groups may be concatenated directly.
pub fn encode_base64(bytes: &[u8]) -> String {
    const CHUNK: usize = 3 * 1024 * 1024;
    let capacity = bytes.len().div_ceil(3).checked_mul(4).unwrap_or(0);
    let mut output = String::with_capacity(capacity);
    for chunk in bytes.chunks(CHUNK) {
        data_encoding::BASE64.encode_append(chunk, &mut output);
    }
    output
}

#[cfg(test)]
mod encoding_tests {
    #[test]
    fn large_assets_preserve_base64_groups_and_final_padding() {
        let bytes: Vec<_> = (0..9 * 1024 * 1024 + 2).map(|n| (n % 251) as u8).collect();
        for length in [
            0,
            1,
            2,
            3 * 1024 * 1024 - 1,
            3 * 1024 * 1024,
            3 * 1024 * 1024 + 1,
            8 * 1024 * 1024 + 1,
            bytes.len(),
        ] {
            let encoded = super::encode_base64(&bytes[..length]);
            assert_eq!(encoded.len(), length.div_ceil(3) * 4);
            assert_eq!(
                data_encoding::BASE64.decode(encoded.as_bytes()).unwrap(),
                bytes[..length]
            );
        }
    }
}
