use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest = Path::new(&out_dir).join("bundled_themes.rs");
    let mut out = fs::File::create(&dest).expect("failed to create bundled_themes.rs");

    writeln!(out, "pub const BUNDLED: &[(&str, &str)] = &[").unwrap();

    let themes_dir = Path::new("themes");
    if let Ok(entries) = fs::read_dir(themes_dir) {
        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(std::ffi::OsStr::to_str) == Some("toml"))
            .collect();
        files.sort_by_key(|e| e.file_name());

        for entry in &files {
            let path = entry.path();
            let filename = path.file_name().unwrap().to_str().unwrap();
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            writeln!(out, "    ({filename:?}, {content:?}),").unwrap();
        }
    }

    writeln!(out, "];").unwrap();

    println!("cargo:rerun-if-changed=themes/");
}
