#![no_std]
#![no_main]
#![feature(asm_const, asm_experimental_arch)]

use sex_pdx::{pdx_listen_raw, PdxRequest, MessageType, HIDEvent, pdx_call}; // Only keep necessary sex_pdx imports
use core::ptr;
use x86_64::instructions::hlt;

// --- Syscall IDs for SexCompositor (Duplicated from sexdisplay for now, should be in common header) ---
const PDX_SEX_WINDOW_CREATE: u64 = 0xDE;
const PDX_COMPOSITOR_COMMIT: u64 = 0xDD;
const PDX_SET_WINDOW_ROUNDNESS: u64 = 0xDF;
const PDX_SET_WINDOW_BLUR: u64 = 0xE0;
const PDX_SET_WINDOW_ANIMATION: u64 = 0xE1;

// --- Client-side abstraction for SexCompositor interaction ---
// In a full system, this would be part of a `silk-client` crate.
struct PdxCompositorClient {
    // Placeholder for compositor's PD ID or capability slot.
    // Assuming a fixed ID for the display server.
    display_server_pd_id: u32, 
}

impl PdxCompositorClient {
    fn new() -> Self {
        Self { display_server_pd_id: 5 } // Assuming display server is on PD ID 5
    }

    fn create_window(&mut self, x: u32, y: u32, width: u32, height: u32, pfn_base: u64) -> u64 {
        #[repr(C)] // Ensure layout is C-compatible for syscall
        struct SexWindowCreateParams {
            x: u32, y: u32, width: u32, height: u32, pfn_base: u64,
        }
        let params = SexWindowCreateParams { x, y, width, height, pfn_base };
        // Call PDX_SEX_WINDOW_CREATE on the display server PD.
        // arg0 is pointer to params, arg1/arg2/arg3 are unused.
        pdx_call(self.display_server_pd_id, PDX_SEX_WINDOW_CREATE, &params as *const _ as u64, 0, 0)
    }

    fn commit_frame(&mut self, window_id: u64, pfn_list_ptr: u64, num_pages: u64) -> u64 {
        // PDX_COMPOSITOR_COMMIT expects window_id, pfn_list_ptr, num_pages as args.
        // We use a custom calling convention where the actual parameters are bundled with the msg_type if needed.
        // For simplicity, directly mapping to arg0, arg1, arg2.
        pdx_call(self.display_server_pd_id, PDX_COMPOSITOR_COMMIT, window_id, pfn_list_ptr, 0) // arg2 for num_pages might be needed here
    }

    fn set_window_roundness(&mut self, window_id: u64, radius: u32) -> u64 {
        pdx_call(self.display_server_pd_id, PDX_SET_WINDOW_ROUNDNESS, window_id, radius as u64, 0)
    }

    fn set_window_blur(&mut self, window_id: u64, strength: u32) -> u64 {
        pdx_call(self.display_server_pd_id, PDX_SET_WINDOW_BLUR, window_id, strength as u64, 0)
    }

    fn set_window_animation(&mut self, window_id: u64, animating: bool) -> u64 {
        pdx_call(self.display_server_pd_id, PDX_SET_WINDOW_ANIMATION, window_id, animating as u64, 0)
    }
}


// Define event types that the shell might receive.
// These would typically be defined by the IPC/event service protocols.
#[derive(Debug, Clone, Copy)]
pub enum ShellEvent {
    NoEvent,        // When no events are pending
    Redraw(u64),    // Window ID to redraw
    Quit,           // Shell should quit
    ToggleLauncher, // Simulate a keyboard shortcut to open/close the launcher
    KeyPress(u16),  // A raw key press event
    // Add other events like Input(InputEvent), WindowMove, etc.
}

// Function to get the next event from the system via PDX.
fn get_next_event() -> ShellEvent {
    // In a real system, this would block until an event is received.
    // For this prototype, we'll poll and then process the request.
    let request = pdx_listen_raw(0); // Flags 0 for default listen behavior

    match request.num { // 'num' typically holds the message type or syscall ID
        _ if request.caller_pd == 0 => { // Kernel messages or special events
            match unsafe { ptr::read_volatile(request.arg0 as *const MessageType) } {
                MessageType::HIDEvent { ev_type, code, value } => {
                    // Assuming ev_type 1 is EV_KEY for key presses/releases
                    if ev_type == 1 && value == 1 { // Key press
                        ShellEvent::KeyPress(code)
                    } else {
                        ShellEvent::NoEvent
                    }
                },
                _ => ShellEvent::NoEvent,
            }
        },
        _ => ShellEvent::NoEvent, // Other PDX requests
    }
}


// Mock drawing function for main window content
fn draw_window_content(
    _compositor: &PdxCompositorClient, // Compositor client might be used for querying info
    _window_id: u64,
    _width: u32,
    _height: u32,
    _pfn_base: u64,
) {
    // Simulate drawing a background color for the main window.
}

// Mock drawing function for panel content
fn draw_panel_content(
    _compositor: &PdxCompositorClient,
    _panel_id: u64,
    _width: u32,
    _height: u32,
    _pfn_base: u64,
) {
    // Simulate drawing panel elements like a clock, workspace indicators.
    // For now, just a placeholder.
}

// Mock drawing function for launcher content
fn draw_launcher_content(
    _compositor: &PdxCompositorClient,
    _launcher_id: u64,
    _width: u32,
    _height: u32,
    _pfn_base: u64,
) {
    // Simulate drawing an input field and some app icons.
}

// Entry point for the Silk Shell
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize PDX client for interacting with SexCompositor
    let mut compositor = PdxCompositorClient::new();

    // Placeholder PFN bases for window buffers.
    let mock_main_window_pfn_base: u64 = 0x12345000;
    let mock_panel_pfn_base: u64 = 0x12346000;
    let mock_launcher_pfn_base: u64 = 0x12347000; // PFN for the launcher window

    // Create main application window
    let main_window_id = compositor.create_window(
        100, // x position
        100, // y position
        800, // width
        600, // height
        mock_main_window_pfn_base,
    );

    if main_window_id == 0 {
        loop {} // Halt if main window creation fails
    }
    let main_window_width = 800;
    let main_window_height = 600;

    // Create panel window
    const PANEL_HEIGHT: u32 = 30; // pixels
    let panel_id = compositor.create_window(
        0, // x position (top-left)
        0, // y position (top-left)
        1024, // width (assuming framebuffer width for now, needs to be dynamic)
        PANEL_HEIGHT, // height
        mock_panel_pfn_base,
    );

    if panel_id == 0 {
        loop {} // Halt if panel creation fails
    }
    let panel_width = 1024;
    let panel_height = PANEL_HEIGHT;

    // Apply UI aesthetics to panel
    compositor.set_window_roundness(panel_id, 8); // 8px rounded corners
    compositor.set_window_blur(panel_id, 10);    // Light blur
    compositor.set_window_animation(panel_id, true); // Enable animations for panel


    // Create launcher window (initially hidden)
    const LAUNCHER_WIDTH: u32 = 400;
    const LAUNCHER_HEIGHT: u32 = 300;
    // Centered horizontally, slightly below the top
    let launcher_x = (1024 / 2).saturating_sub(LAUNCHER_WIDTH / 2);
    let launcher_y = (768 / 2).saturating_sub(LAUNCHER_HEIGHT / 2); // Assuming 768px height for now

    let launcher_id = compositor.create_window(
        launcher_x,
        launcher_y,
        LAUNCHER_WIDTH,
        LAUNCHER_HEIGHT,
        mock_launcher_pfn_base,
    );

    if launcher_id == 0 {
        loop {} // Halt if launcher creation fails
    }
    let launcher_width = LAUNCHER_WIDTH;
    let launcher_height = LAUNCHER_HEIGHT;

    // Apply UI aesthetics to launcher
    compositor.set_window_roundness(launcher_id, 12); // More rounded corners
    compositor.set_window_blur(launcher_id, 25);    // Stronger blur
    compositor.set_window_animation(launcher_id, true); // Enable animations

    let mut is_launcher_visible = false; // Launcher starts hidden
    // let mut cycle_count = 0; // No longer needed for mock event cycling

    // Main event loop
    loop {
        unsafe { hlt() }; // Wait for event

        let event = get_next_event(); // No longer passing compositor or cycle_count

        match event {
            ShellEvent::Redraw(id) => {
                let (window_info_width, window_info_height, window_info_pfn_base) = if id == main_window_id {
                    (main_window_width, main_window_height, mock_main_window_pfn_base)
                } else if id == panel_id {
                    (panel_width, panel_height, mock_panel_pfn_base)
                } else if id == launcher_id && is_launcher_visible {
                    (launcher_width, launcher_height, mock_launcher_pfn_base)
                } else {
                    // Unknown window ID, or launcher is hidden, ignore redraw for now
                    continue;
                };

                // Perform drawing
                if id == main_window_id {
                    draw_window_content(&compositor, id, window_info_width, window_info_height, window_info_pfn_base);
                } else if id == panel_id {
                    draw_panel_content(&compositor, id, window_info_width, window_info_height, window_info_pfn_base);
                } else if id == launcher_id {
                    draw_launcher_content(&compositor, id, window_info_width, window_info_height, window_info_pfn_base);
                }

                // Commit the frame to display the changes
                let num_pages = (window_info_width * window_info_height * 4) / 4096 + 1;
                let mut pfn_list = [window_info_pfn_base];

                compositor.commit_frame(id, pfn_list.as_ptr() as u64, num_pages as u64);
            }
            ShellEvent::ToggleLauncher => {
                is_launcher_visible = !is_launcher_visible;
                if is_launcher_visible {
                    // When launcher becomes visible, trigger a redraw for it
                    // TODO: Need a way to explicitly tell compositor to redraw a window
                    // For now, next redraw cycle will pick it up.
                }
            }
            ShellEvent::KeyPress(code) => {
                // Example: Press 'L' to toggle launcher (assuming 'L' is scancode 26)
                if code == 0x26 { // Scancode for 'L' (this needs to be looked up)
                    is_launcher_visible = !is_launcher_visible;
                    // Trigger a redraw for the launcher if it becomes visible
                    if is_launcher_visible {
                        let num_pages = (launcher_width * launcher_height * 4) / 4096 + 1;
                        let mut pfn_list = [mock_launcher_pfn_base];
                        compositor.commit_frame(launcher_id, pfn_list.as_ptr() as u64, num_pages as u64);
                    }
                }
            }
            ShellEvent::Quit => {
                // Handle shell exit.
            }
            ShellEvent::NoEvent => {
                // No events, continue loop.
            }
        }
    }
}
