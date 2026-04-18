use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Fix Cargo.toml files (no_std / serde)
    for entry in walkdir::WalkDir::new(".") {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().map_or(false, |n| n == "Cargo.toml") && !path.to_str().unwrap().contains("target") {
            if path.to_str().unwrap().contains("sex-ld") { continue; }
            let content = fs::read_to_string(&path)?;
            let mut new_content = content.replace(
                "serde = \"1.0\"", 
                "serde = { version = \"1.0\", default-features = false, features = [\"derive\"] }"
            );
            // Ensure default-features = false for specific crates
            for dep in &["serde", "futures-core"] {
                let search = format!("{} = {{", dep);
                if new_content.contains(&search) && !new_content.contains("default-features") {
                    new_content = new_content.replace(&search, &format!("{} = {{ default-features = false, ", dep));
                }
            }
            fs::write(&path, new_content.replace(",,", ","))?;
        }
    }

    // 2. Fix pdx.rs (Surgical Patch)
    let pdx_path = Path::new("servers/sex-ld/src/pdx.rs");
    if pdx_path.exists() {
        let content = fs::read_to_string(pdx_path)?;
        let new_content = content
            .replace("ResolveObject { name }", "ResolveObject { name: _ }")
            .replace("MapLibrary { hash, base_addr }", "MapLibrary { hash, base_addr: _ }")
            .replace("MessageType, ", "");
        // NOTE: We do NOT touch GetEntry { hash } because hash is used on line 18!
        fs::write(pdx_path, new_content)?;
    }

    // 3. Fix main.rs (macOS Host Shims)
    let main_path = Path::new("servers/sex-ld/src/main.rs");
    if main_path.exists() {
        let content = fs::read_to_string(main_path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        for line in lines.iter_mut() {
            if line.trim() == "#![no_std]" || line.trim() == "#![no_main]" {
                *line = format!("// {}", line);
            }
        }

        let content = lines.join("\n");
        let mut final_content = content;
        if !final_content.contains("not(target_os = \"macos\")") {
            final_content = final_content.replace("fn panic", "#[cfg(not(target_os = \"macos\"))]\nfn panic");
        }
        if !final_content.contains("fn main()") {
            final_content.push_str("\n#[cfg(target_os = \"macos\")]\nfn main() { println!(\"Sex Linker (Host Mode) Online\"); }\n");
        }
        fs::write(main_path, final_content)?;
    }

    Ok(())
}

// Simple WalkDir implementation since we can't use external crates easily in a one-off script
mod walkdir {
    use std::fs;
    use std::path::{Path, PathBuf};
    pub struct WalkDir { stack: Vec<PathBuf> }
    impl WalkDir { pub fn new<P: AsRef<Path>>(path: P) -> Self { Self { stack: vec![path.as_ref().to_path_buf()] } } }
    impl Iterator for WalkDir {
        type Item = Result<fs::DirEntry, std::io::Error>;
        fn next(&mut self) -> Option<Self::Item> {
            while let Some(path) = self.stack.pop() {
                if path.is_dir() {
                    if let Ok(entries) = fs::read_dir(path) {
                        for entry in entries.flatten() { self.stack.push(entry.path()); }
                    }
                } else if let Ok(parent) = fs::read_dir(path.parent().unwrap()) {
                    for entry in parent.flatten() {
                        if entry.path() == path { return Some(Ok(entry)); }
                    }
                }
            }
            None
        }
    }
}
