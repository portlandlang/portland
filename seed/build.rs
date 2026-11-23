//! Bakes Portland's core into the seed (ADR 0093): every file under
//! `core/` becomes a `(name, text)` pair in `CORE_FILES`, so `pdx` is one
//! file that runs from any directory, and the seed and the compiler it
//! runs read the same core.

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let core = Path::new(&manifest).join("../core");
    println!("cargo:rerun-if-changed={}", core.display());
    let mut names: Vec<String> = std::fs::read_dir(&core)
        .expect("core/ should sit beside seed/")
        .map(|entry| entry.expect("core/ should be readable").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".pdx"))
        .collect();
    names.sort();
    let mut table = String::from("pub const CORE_FILES: &[(&str, &str)] = &[\n");
    for name in &names {
        let path = core.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        writeln!(
            table,
            "    ({name:?}, include_str!({:?})),",
            path.display().to_string()
        )
        .expect("writing to a String cannot fail");
    }
    table.push_str("];\n");
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    std::fs::write(Path::new(&out).join("core_files.rs"), table)
        .expect("the build directory should be writable");
}
