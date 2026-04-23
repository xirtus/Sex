use clap::{Parser, ValueEnum};
use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
pub struct NewArgs {
    /// Name of the new application
    pub name: String,
    /// Base template to use
    #[arg(long, value_enum)]
    pub base: Option<BaseTemplate>,
    /// Architecture template (e.g. silk-pdx)
    #[arg(long)]
    pub template: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum BaseTemplate {
    CosmicFiles,
    CosmicPanel,
    CosmicAppletNetwork,
    CosmicSettings,
    CosmicApplets,
    Servo,
    RustMedia,
    CosmicEdit,
    RedoxCalc,
}

const DEFAULT_MAIN_RS: &str = include_str!("../templates/default/main.rs.tpl");
const DEFAULT_CARGO_TOML: &str = include_str!("../templates/default/Cargo.toml.tpl");

const COSMIC_FILES_MAIN_RS: &str = include_str!("../templates/cosmic_files/main.rs.tpl");
const COSMIC_FILES_CARGO_TOML: &str = include_str!("../templates/cosmic_files/Cargo.toml.tpl");

const COSMIC_EDIT_MAIN_RS: &str = include_str!("../templates/cosmic_edit/main.rs.tpl");
const COSMIC_EDIT_CARGO_TOML: &str = include_str!("../templates/cosmic_edit/Cargo.toml.tpl");

const REDOX_CALC_MAIN_RS: &str = include_str!("../templates/redox_calc/main.rs.tpl");
const REDOX_CALC_CARGO_TOML: &str = include_str!("../templates/redox_calc/Cargo.toml.tpl");

const COSMIC_PANEL_MAIN_RS: &str = include_str!("../templates/cosmic_panel/main.rs.tpl");
const COSMIC_PANEL_CARGO_TOML: &str = include_str!("../templates/cosmic_panel/Cargo.toml.tpl");

const COSMIC_APPLET_NETWORK_MAIN_RS: &str = include_str!("../templates/cosmic_applet_network/main.rs.tpl");
const COSMIC_APPLET_NETWORK_CARGO_TOML: &str = include_str!("../templates/cosmic_applet_network/Cargo.toml.tpl");

const COSMIC_SETTINGS_MAIN_RS: &str = include_str!("../templates/cosmic_settings/main.rs.tpl");
const COSMIC_SETTINGS_CARGO_TOML: &str = include_str!("../templates/cosmic_settings/Cargo.toml.tpl");

const COSMIC_APPLETS_MAIN_RS: &str = include_str!("../templates/cosmic_applets/main.rs.tpl");
const COSMIC_APPLETS_CARGO_TOML: &str = include_str!("../templates/cosmic_applets/Cargo.toml.tpl");

const SERVO_MAIN_RS: &str = include_str!("../templates/servo/main.rs.tpl");
const SERVO_CARGO_TOML: &str = include_str!("../templates/servo/Cargo.toml.tpl");

const RUST_MEDIA_MAIN_RS: &str = include_str!("../templates/rust_media/main.rs.tpl");
const RUST_MEDIA_CARGO_TOML: &str = include_str!("../templates/rust_media/Cargo.toml.tpl");

// ... similarly for other templates

pub fn handle_new(args: NewArgs) -> Result<()> {
    println!("Creating new app: {}", args.name);
    let app_path_str = format!("apps/{}", args.name);
    let app_path = Path::new(&app_path_str);

    if app_path.exists() {
        return Err(anyhow::anyhow!("Application '{}' already exists", args.name));
    }

    fs::create_dir_all(app_path.join("src"))?;

    let (main_rs, cargo_toml) = match args.base {
        Some(BaseTemplate::CosmicFiles) => (COSMIC_FILES_MAIN_RS, COSMIC_FILES_CARGO_TOML),
        Some(BaseTemplate::CosmicEdit) => (COSMIC_EDIT_MAIN_RS, COSMIC_EDIT_CARGO_TOML),
        Some(BaseTemplate::RedoxCalc) => (REDOX_CALC_MAIN_RS, REDOX_CALC_CARGO_TOML),
        Some(BaseTemplate::CosmicPanel) => (COSMIC_PANEL_MAIN_RS, COSMIC_PANEL_CARGO_TOML),
        Some(BaseTemplate::CosmicAppletNetwork) => (COSMIC_APPLET_NETWORK_MAIN_RS, COSMIC_APPLET_NETWORK_CARGO_TOML),
        Some(BaseTemplate::CosmicSettings) => (COSMIC_SETTINGS_MAIN_RS, COSMIC_SETTINGS_CARGO_TOML),
        Some(BaseTemplate::CosmicApplets) => (COSMIC_APPLETS_MAIN_RS, COSMIC_APPLETS_CARGO_TOML),
        Some(BaseTemplate::Servo) => (SERVO_MAIN_RS, SERVO_CARGO_TOML),
        Some(BaseTemplate::RustMedia) => (RUST_MEDIA_MAIN_RS, RUST_MEDIA_CARGO_TOML),
        None => (DEFAULT_MAIN_RS, DEFAULT_CARGO_TOML),
    };

    let main_rs_content = main_rs.replace("{{app_name}}", &args.name);
    let cargo_toml_content = cargo_toml.replace("{{app_name}}", &args.name);

    fs::write(app_path.join("src/main.rs"), main_rs_content)?;
    fs::write(app_path.join("Cargo.toml"), cargo_toml_content)?;
    
    println!("Successfully created application '{}' at '{}'", args.name, app_path.display());

    Ok(())
}
