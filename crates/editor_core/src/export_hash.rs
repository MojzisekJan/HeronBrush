//! Normalize pointer-sized Hash inputs for shared desktop/wasm export asset names.
use std::hash::Hasher;

#[derive(Default)]
pub(crate) struct ExportHasher(std::collections::hash_map::DefaultHasher);
impl Hasher for ExportHasher {
    fn finish(&self) -> u64 { self.0.finish() }
    fn write(&mut self, bytes: &[u8]) { self.0.write(bytes); }
    fn write_usize(&mut self, value: usize) { self.write_u64(value as u64); }
    fn write_isize(&mut self, value: isize) { self.write_i64(value as i64); }
}
