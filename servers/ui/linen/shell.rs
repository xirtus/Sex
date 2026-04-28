use alloc::string::String;
use alloc::vec::Vec;
use crate::linen::fs::FileSystem;
use crate::linen::cap_view::CapabilityView;

pub struct Shell {
    pub fs: FileSystem,
    pub caps: CapabilityView,
    pub input_buffer: String,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            fs: FileSystem::new(),
            caps: CapabilityView::new(),
            input_buffer: String::new(),
        }
    }

    pub fn dispatch(&mut self, cmd: &str) -> String {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        match parts.as_slice() {
            ["ls"] => String::from("UCGM-Node(1): /root"),
            ["inspect"] => {
                let (svcs, active) = self.caps.query_system();
                alloc::format!("Services: {}, Active PDs: {}", svcs, active)
            },
            ["cd", dir] => {
                self.fs.navigate(dir);
                alloc::format!("Navigated to {}", dir)
            },
            _ => String::from("Unknown command"),
        }
    }
}
