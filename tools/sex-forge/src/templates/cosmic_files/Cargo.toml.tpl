[package]
name = "{{app_name}}"
version = "0.1.0"
edition = "2024"

[dependencies]
silkclient = { path = "../../crates/silk-client" }
sex-pdx = { path = "../../crates/sex-pdx" }
sex-graphics = { path = "../../crates/sex-graphics" }

# Dependencies from cosmic-files would be added here
# This is a stub and would require a full dependency analysis
# of the original cosmic-files crate.
#
# For example:
# cosmic-files = { git = "https://github.com/pop-os/cosmic-files" }
# iced = "0.10"
# libcosmic = { git = "https://github.com/pop-os/libcosmic" }
