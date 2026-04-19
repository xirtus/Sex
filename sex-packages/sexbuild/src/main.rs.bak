use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{Read, Write};
use anyhow::{Context, Result, anyhow};
use petgraph::graph::DiGraph;
use petgraph::algo::toposort;
use sha2::{Sha256, Digest};
use sex_pdx::{pdx_call, MessageType, StoreProtocol};

const MAGIC: &[u8; 8] = b"SEXPAC01";

pub struct PdxClient {
    pub slot: u32,
}

impl PdxClient {
    pub fn new() -> Self {
        Self { slot: 4 } // sexshop is Slot 4
    }

    pub fn put_object(&self, hash: [u8; 32], data: &[u8]) -> Result<()> {
        #[cfg(target_os = "none")]
        {
            // In SASOS, we need physical memory for PDX. 
            let paddr = self.alloc_phys(data.len() as u64)?;
            unsafe {
                core::ptr::copy_nonoverlapping(data.as_ptr(), paddr as *mut u8, data.len());
            }

            let msg = MessageType::Store(StoreProtocol::ObjectPut {
                hash,
                data_paddr: paddr,
                data_len: data.len() as u64,
            });

            let res = pdx_call(self.slot, 0, &msg as *const _ as u64, 0);
            if res != 0 {
                return Err(anyhow!("PDX ObjectPut failed: {}", res));
            }
        }
        Ok(())
    }

    pub fn get_object(&self, hash: [u8; 32]) -> Result<Vec<u8>> {
        #[cfg(target_os = "none")]
        {
            let msg = MessageType::Store(StoreProtocol::ObjectGet { hash });
            let res = pdx_call(self.slot, 0, &msg as *const _ as u64, 0);
            
            if res == 0 || res == u64::MAX {
                return Err(anyhow!("PDX ObjectGet failed or not found"));
            }

            // res is PFN in SASOS
            let paddr = res; // Simplified
            let mut data = vec![0u8; 4096]; // Prototype size
            unsafe {
                core::ptr::copy_nonoverlapping(paddr as *const u8, data.as_mut_ptr(), 4096);
            }
            Ok(data)
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = hash;
            Err(anyhow!("PDX not supported on host"))
        }
    }

    pub fn exists(&self, hash: [u8; 32]) -> bool {
        #[cfg(target_os = "none")]
        {
            let msg = MessageType::Store(StoreProtocol::ObjectExists { hash });
            let res = pdx_call(self.slot, 0, &msg as *const _ as u64, 0);
            res == 1
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = hash;
            false
        }
    }

    fn alloc_phys(&self, size: u64) -> Result<u64> {
        #[cfg(target_os = "none")]
        {
            // Call Slot 1 (Kernel/MM) for physical allocation
            let res = pdx_call(1, 12 /* ALLOC_PHYS */, (size + 4095) / 4096, 0);
            if res == 0 || res == u64::MAX {
                return Err(anyhow!("Physical allocation failed"));
            }
            Ok(res)
        }
        #[cfg(not(target_os = "none"))]
        {
            let _ = size;
            Ok(0)
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Recipe {
    #[serde(default)]
    pub package: Option<Package>,
    #[serde(default)]
    pub source: Option<Source>,
    pub build: Option<Build>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Package {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String { "0.1.0".to_string() }

#[derive(Debug, Deserialize, Clone)]
pub struct Source {
    pub git: Option<String>,
    pub tar: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Build {
    pub template: Option<String>,
    pub depends: Option<Vec<String>>,
    pub manifest: Option<String>,
}

pub struct SexBuilder {
    pub recipes_dir: PathBuf,
    pub templates_dir: PathBuf,
    pub build_dir: PathBuf,
    pub binpkgs_dir: PathBuf,
    pub sasroot: PathBuf,
    pub pdx: PdxClient,
}

impl SexBuilder {
    pub fn new() -> Self {
        Self {
            recipes_dir: PathBuf::from("recipes"),
            templates_dir: PathBuf::from("srcpkgs"),
            build_dir: PathBuf::from("build_dir"),
            binpkgs_dir: PathBuf::from("../binpkgs"),
            sasroot: PathBuf::from("../sasroot"),
            pdx: PdxClient::new(),
        }
    }

    pub fn sync_cookbook(&self) -> Result<()> {
        let redox_dir = self.recipes_dir.join("redox-os");
        if !redox_dir.exists() {
            println!("sexbuild: Cloning Redox Cookbook...");
            Command::new("git")
                .args(["clone", "--depth", "1", "https://gitlab.redox-os.org/redox-os/cookbook.git", redox_dir.to_str().unwrap()])
                .status()?;
        } else {
            println!("sexbuild: Syncing Redox Cookbook...");
            Command::new("git")
                .args(["-C", redox_dir.to_str().unwrap(), "pull"])
                .status()?;
        }
        Ok(())
    }

    fn fetch_source(&self, name: &str, recipe: &Recipe) -> Result<PathBuf> {
        let source = recipe.source.as_ref().ok_or(anyhow!("No source section"))?;
        let pkg_build_dir = self.build_dir.join(name);
        fs::create_dir_all(&pkg_build_dir)?;
        
        if let Some(url) = &source.tar {
            let tarball_path = pkg_build_dir.join("source.tar.gz");
            
            // Phase 23: Check sexshop object store first
            if let Some(expected_hash) = &source.sha256 {
                let mut hash_bytes = [0u8; 32];
                if hex::decode_to_slice(expected_hash, &mut hash_bytes).is_ok() {
                    if self.pdx.exists(hash_bytes) {
                        println!("sexbuild: Source for {} found in sexshop object store", name);
                        let data = self.pdx.get_object(hash_bytes)?;
                        File::create(&tarball_path)?.write_all(&data)?;
                        return Ok(tarball_path);
                    }
                }
            }

            if !tarball_path.exists() {
                println!("sexbuild: Fetching {}...", url);
                let mut response = reqwest::blocking::get(url)?;
                let mut file = File::create(&tarball_path)?;
                response.copy_to(&mut file)?;
            }

            if let Some(expected_hash) = &source.sha256 {
                let mut file = File::open(&tarball_path)?;
                let mut hasher = Sha256::new();
                let mut buffer = [0; 8192];
                while let Ok(n) = file.read(&mut buffer) {
                    if n == 0 { break; }
                    hasher.update(&buffer[..n]);
                }
                let hash = hasher.finalize();
                let actual_hash = hex::encode(hash);
                if actual_hash != *expected_hash {
                    return Err(anyhow!("Hash mismatch! Expected {}, got {}", expected_hash, actual_hash));
                }

                // Cache in sexshop
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(&hash);
                let mut data = Vec::new();
                File::open(&tarball_path)?.read_to_end(&mut data)?;
                self.pdx.put_object(hash_bytes, &data)?;
            }
            Ok(tarball_path)
        } else if let Some(git_url) = &source.git {
            let git_path = pkg_build_dir.join("source");
            if !git_path.exists() {
                println!("sexbuild: Cloning {}...", git_url);
                Command::new("git").args(["clone", "--depth", "1", git_url, git_path.to_str().unwrap()]).status()?;
            }
            Ok(git_path)
        } else {
            Err(anyhow!("No tar or git source"))
        }
    }

    fn pack_spd(&self, name: &str, recipe: &Recipe, bin_path: &Path) -> Result<()> {
        let name_bytes = name.as_bytes();
        let mut name_fixed = [0u8; 32];
        let len = name_bytes.len().min(32);
        name_fixed[..len].copy_from_slice(&name_bytes[..len]);

        let mut data = Vec::new();
        File::open(bin_path)?.read_to_end(&mut data)?;
        
        let size = data.len() as u64;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = hasher.finalize();
        let hash_bytes: [u8; 32] = hash.into();

        // PDX Deduplication Check
        if self.pdx.exists(hash_bytes) {
            println!("sexbuild: Artifact {} already in sexshop object store (dedup)", name);
        } else {
            println!("sexbuild: Uploading {} to sexshop object store...", name);
            self.pdx.put_object(hash_bytes, &data)?;
        }

        fs::create_dir_all(&self.binpkgs_dir)?;
        let version = recipe.package.as_ref().map(|p| p.version.clone()).unwrap_or_else(default_version);
        let out_path = self.binpkgs_dir.join(format!("{}-{}.spd", name, version));
        let mut f = File::create(out_path)?;
        
        f.write_all(MAGIC)?;
        f.write_all(&name_fixed)?;
        f.write_all(&size.to_le_bytes())?;
        f.write_all(&hash)?;
        f.write_all(&data)?;
        
        let pos = f.metadata()?.len();
        let padding = (4096 - (pos % 4096)) % 4096;
        f.write_all(&vec![0u8; padding as usize])?;

        println!("sexbuild: Generated SPD for {}", name);
        Ok(())
    }

    fn cook(&self, name: &str) -> Result<()> {
        let recipe = self.load_recipe(name)?;
        let version = recipe.package.as_ref().map(|p| p.version.clone()).unwrap_or_else(default_version);
        println!("sexbuild: Cooking {} v{}...", name, version);

        if let Some(build) = &recipe.build {
            if build.template.as_deref() == Some("shell") {
                return self.run_legacy_shell(name);
            }
        }

        if recipe.source.is_some() {
            self.fetch_source(name, &recipe)?;
        }
        self.run_cargo_build(name, &recipe)
    }

    fn run_cargo_build(&self, name: &str, recipe: &Recipe) -> Result<()> {
        println!("sexbuild: Running real cargo build for {}...", name);
        
        // Execute cargo build for the specific package
        // Phase 23: Use sex-ld for linking
        let status = Command::new("cargo")
            .args(["build", "--release", "--target", "x86_64-unknown-none", "-p", name])
            .env("RUSTFLAGS", "-C linker=sex-ld")
            .status()?;
        
        if !status.success() {
            return Err(anyhow!("Cargo build failed for {}", name));
        }

        // Find binary. Mapping: crates/NAME -> target/.../NAME
        let bin_name = name;
        let bin_path = PathBuf::from("target/x86_64-unknown-none/release").join(bin_name);
        
        if !bin_path.exists() {
             return Err(anyhow!("Binary not found at {:?}", bin_path));
        }
        
        self.pack_spd(name, recipe, &bin_path)
    }

    fn run_legacy_shell(&self, name: &str) -> Result<()> {
        println!("sexbuild: Running legacy shell builder for {}...", name);
        let status = Command::new("./bin/sex-src.sh")
            .args(["pkg", name])
            .current_dir("../sex-src")
            .status()?;

        if status.success() { Ok(()) } else { Err(anyhow!("Legacy build failed")) }
    }

    pub fn load_recipe(&self, name: &str) -> Result<Recipe> {
        let toml_path = self.recipes_dir.join(name).join("recipe.toml");
        if toml_path.exists() {
            let content = fs::read_to_string(&toml_path)?;
            Ok(toml::from_str(&content)?)
        } else {
            let redox_root = self.recipes_dir.join("redox-os/recipes");
            if redox_root.exists() {
                for entry in walkdir::WalkDir::new(&redox_root)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_dir() && e.file_name().to_str() == Some(name))
                {
                    let r_path = entry.path().join("recipe.toml");
                    if r_path.exists() {
                        let content = fs::read_to_string(&r_path)?;
                        let mut recipe: Recipe = toml::from_str(&content)?;
                        if recipe.build.is_none() {
                            recipe.build = Some(Build { template: Some("cargo".to_string()), depends: None, manifest: None });
                        }
                        return Ok(recipe);
                    }
                }
            }
            
            let shell_path = self.templates_dir.join(name).join("template");
            if shell_path.exists() {
                Ok(Recipe {
                    package: Some(Package { name: name.to_string(), version: "legacy".to_string() }),
                    source: None,
                    build: Some(Build { template: Some("shell".to_string()), depends: None, manifest: None }),
                })
            } else {
                Err(anyhow!("Recipe {} not found", name))
            }
        }
    }

    pub fn build_recursive(&self, name: &str) -> Result<()> {
        let mut graph = DiGraph::<String, ()>::new();
        let mut nodes = HashMap::new();
        let mut pending = vec![name.to_string()];
        let mut visited = HashSet::new();

        while let Some(current) = pending.pop() {
            if visited.contains(&current) { continue; }
            visited.insert(current.clone());

            let recipe = self.load_recipe(&current)?;
            let u = *nodes.entry(current.clone()).or_insert_with(|| graph.add_node(current.clone()));

            if let Some(build) = recipe.build {
                if let Some(deps) = build.depends {
                    for dep in deps {
                        let v = *nodes.entry(dep.clone()).or_insert_with(|| graph.add_node(dep.clone()));
                        graph.add_edge(v, u, ());
                        pending.push(dep);
                    }
                }
            }
        }

        let sorted = toposort(&graph, None).map_err(|_| anyhow!("Dependency cycle"))?;
        for node in sorted {
            self.cook(&graph[node])?;
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    std::env::set_var("CARGO_BUILD_TARGET", "x86_64-unknown-none");

    if std::env::consts::ARCH == "aarch64" {
        panic!("aarch64 host detected! Phase 28 requires strict x86_64 cross-compilation. Force linux/amd64 Docker or use x86_64-unknown-none targets explicitly.");
    }

    let args: Vec<String> = std::env::args().collect();
    let builder = SexBuilder::new();
    if args.len() < 2 { return Ok(()); }
    
    match args[1].as_str() {
        "sync" => builder.sync_cookbook(),
        "build" => builder.build_recursive(&args[2]),
        "rebuild-kernel" => {
            println!("sexbuild: Rebuilding SASOS kernel (x17r1 x86_64)...");
            let status = Command::new("cargo")
                .args(["build", "--release", "--target", "x86_64-sex.json", "-p", "kernel"])
                .env("RUSTFLAGS", "-C linker=sex-ld -C target-cpu=skylake -C link-arg=--script=kernel/linker.ld")
                .env("CARGO_BUILD_TARGET", "x86_64-unknown-none")
                .status()?;
            if !status.success() {
                return Err(anyhow!("Kernel rebuild failed"));
            }
            Ok(())
        },
        pkg => builder.build_recursive(pkg),
    }
}
