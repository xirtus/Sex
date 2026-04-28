pub mod shell;
pub mod fs;
pub mod cap_view;

pub struct LinenCore {
    pub shell: shell::Shell,
    pub fs: fs::FileSystem,
    pub cap_view: cap_view::CapabilityView,
}

impl LinenCore {
    pub fn new() -> Self {
        Self {
            shell: shell::Shell::new(),
            fs: fs::FileSystem::new(),
            cap_view: cap_view::CapabilityView::new(),
        }
    }
}
