//! Portland's core as the seed carries it (ADR 0093): the files under
//! `core/`, baked in at build by `build.rs`.

include!(concat!(env!("OUT_DIR"), "/core_files.rs"));

/// The entry file: the load order, one `require_relative` per core file.
pub const ENTRY: &str = "core.pdx";

/// One core file's text, by its file name.
pub fn text_of(name: &str) -> Option<&'static str> {
    CORE_FILES
        .iter()
        .find(|(file, _)| *file == name)
        .map(|(_, text)| *text)
}
