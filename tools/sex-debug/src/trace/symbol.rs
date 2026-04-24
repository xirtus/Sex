use std::collections::HashMap;
use std::process::Command;

pub struct SymbolResolver {
    binary_path: String,
    cache: HashMap<u64, String>,
}

impl SymbolResolver {
    pub fn new(binary_path: &str) -> Self {
        Self {
            binary_path: binary_path.to_string(),
            cache: HashMap::new(),
        }
    }

    pub fn resolve(&mut self, rip: u64) -> String {
        if let Some(sym) = self.cache.get(&rip) {
            return sym.clone();
        }

        let output = Command::new("addr2line")
            .arg("-f")
            .arg("-e")
            .arg(&self.binary_path)
            .arg(format!("{:x}", rip))
            .output();

        let sym = match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut lines = stdout.lines();
                let func_name = lines.next().unwrap_or("??").to_string();
                if func_name == "??" {
                    format!("0x{:x}", rip)
                } else {
                    func_name
                }
            }
            _ => format!("0x{:x}", rip),
        };

        self.cache.insert(rip, sym.clone());
        sym
    }
}