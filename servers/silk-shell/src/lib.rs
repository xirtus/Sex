//! silk-shell — Execution Orchestration Layer
//!
//! silk-shell is NOT a service. It is part of the SexOS execution topology:
//! - Capsule lifecycle manager: spawn, suspend, resume, destroy
//! - Runtime orchestration layer
//! - Execution composition system
//! - Domain execution entry point for interactive sessions
//!
//! silk-shell is a node in the capability graph, not a peripheral utility.
//! All capsule execution flows through silk-shell's orchestration context.
//! Authority for capsule operations is granted via sex-pdx capability — not ambient.

#![no_std]

use core::slice;
use sex_pdx::{pdx_call, SLOT_DISPLAY};

pub const PANEL_HEIGHT: u32 = 48;
pub const LAUNCHER_WIDTH: u32 = 320;
pub const SCREEN_WIDTH: u32 = 1280;
pub const SCREEN_HEIGHT: u32 = 720;
pub const BG_COLOR: u32 = 0xFF1E1E2E;

// Local Opcodes
pub const OP_WINDOW_CREATE: u64 = 0xE4; // Legacy stub for lib.rs compilation
pub const OP_APP_SURFACE_REQ: u64 = 0xFA; // App surface request contract
pub const APP_RUNTIME_ABI_VERSION: u8 = 1;
pub const OP_SET_BG: u64 = 0x100;
pub const OP_RENDER_BAR: u64 = 0x101;

// ── App Manifest Capability Contract V1 ─────────────────────────────────────
// App-like PDs declare identity and requested capabilities via a packed manifest
// in OP_APP_SURFACE_REQ arg2. The shell validates the manifest and rejects
// unknown or denied capabilities. No PD ever gets raw framebuffer access or
// shell policy ownership through this contract.

/// Capability bits an app-like PD may declare for its surface.
/// Each bit represents a discrete authority. Unknown bits are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppCapabilityBits(u8);

impl AppCapabilityBits {
    /// No extra capabilities requested.
    pub const NONE: u8 = 0x00;
    /// May send events to Bell notification service (SLOT_BELL).
    pub const BELL: u8 = 0x01;
    /// May access SexFiles VFS storage (SLOT_STORAGE).
    pub const SEXFILES: u8 = 0x02;

    /// Bitmask of all known/valid capability bits.
    /// Any bit outside KNOWN is rejected as unknown.
    /// DENIED is implicit: display/framebuffer ownership and shell policy
    /// ownership are NOT representable as capability bits. Any request
    /// for them would be rejected as unknown.
    const KNOWN: u8 = Self::BELL | Self::SEXFILES;

    /// Validate that all bits are known. Returns Err(()) if unknown bits present.
    pub fn validate(bits: u8) -> Result<Self, ()> {
        if bits & !Self::KNOWN != 0 {
            return Err(());
        }
        Ok(Self(bits))
    }

    pub fn bits(&self) -> u8 { self.0 }
    pub fn has_bell(&self) -> bool { self.0 & Self::BELL != 0 }
    pub fn has_sexfiles(&self) -> bool { self.0 & Self::SEXFILES != 0 }

    /// Human-readable summary of requested capabilities for logging.
    pub fn describe(&self) -> &'static str {
        match self.0 {
            0 => "none",
            _ if self.0 == Self::BELL => "bell",
            _ if self.0 == Self::SEXFILES => "sexfiles",
            _ if self.0 == (Self::BELL | Self::SEXFILES) => "bell+sexfiles",
            _ => "unknown",
        }
    }
}

/// App surface manifest: bounded identity and capability declaration.
///
/// Packed into the three u64 args of OP_APP_SURFACE_REQ (0xFA):
///   arg0 = surface_id (must be >= 200)
///   arg1 = title_id   (must be non-zero)
///   arg2 = packed manifest metadata:
///     bits 0-7    = capability_bits (AppCapabilityBits)
///     bits 8-23   = app_id (16-bit bounded discriminator)
///     bits 24-55  = reserved (must be 0)
///     bits 56-63  = version (must be 0 for V1)
///
/// No PD may request display/framebuffer ownership or shell policy ownership
/// through this manifest. Any attempt to encode such authority is rejected.
#[derive(Debug, Clone, Copy)]
pub struct AppManifest {
    /// Requested surface ID (>= 200 for user surfaces).
    pub surface_id: u64,
    /// Opaque title identifier (non-zero).
    pub title_id: u64,
    /// Bounded app discriminator (16 bits). Zero is reserved/invalid.
    pub app_id: u16,
    /// Declared capability request.
    pub capabilities: AppCapabilityBits,
}

impl AppManifest {
    /// Current manifest version (0 for V1).
    const VERSION: u64 = 0;
    // Bit field masks for arg2 packing.
    const CAP_MASK:     u64 = 0x0000_0000_0000_00FF;
    const APP_ID_MASK:  u64 = 0x0000_0000_00FF_FF00;
    const RSVD_MASK:    u64 = 0x00FF_FFFF_FF00_0000;
    const VERSION_MASK: u64 = 0xFF00_0000_0000_0000;

    /// Pack the manifest into (arg0, arg1, arg2) for PDX message.
    pub fn pack(&self) -> (u64, u64, u64) {
        let arg2 = (Self::VERSION << 56)
                 | ((self.app_id as u64) << 8)
                 | (self.capabilities.bits() as u64);
        (self.surface_id, self.title_id, arg2)
    }

    /// Unpack a manifest from PDX message args. Validates version, reserved
    /// bits, and capability bits. Does NOT validate surface_id/title_id range
    /// (caller does that).
    pub fn unpack(surface_id: u64, title_id: u64, arg2: u64) -> Result<Self, ()> {
        // Version must match.
        let version = (arg2 & Self::VERSION_MASK) >> 56;
        if version != Self::VERSION {
            return Err(());
        }
        // Reserved bits must be zero.
        let reserved = arg2 & Self::RSVD_MASK;
        if reserved != 0 {
            return Err(());
        }
        // Capability bits must all be known.
        let cap_bits = (arg2 & Self::CAP_MASK) as u8;
        let capabilities = AppCapabilityBits::validate(cap_bits)?;
        // App ID is bounded 16-bit; zero is allowed (unidentified app).
        let app_id = ((arg2 & Self::APP_ID_MASK) >> 8) as u16;
        Ok(Self {
            surface_id,
            title_id,
            app_id,
            capabilities,
        })
    }

    /// Whether the app has identified itself with a non-zero app_id.
    pub fn has_identity(&self) -> bool {
        self.app_id != 0
    }
}

/// Orchestration state for silk-shell's active execution context.
/// Tracks the composition topology: active capsules, display geometry, input focus.
#[derive(Default)]
pub struct ShellState {
    pub panel_window_id: u32,
    pub launcher_window_id: u32,
    pub bg_color: u32,          // 0xFF1E1E2E (SexOS dark)
    pub is_launcher_open: bool,
    pub current_mouse_x: i32,
    pub current_mouse_y: i32,
}

/// Handle to an isolated execution capsule managed by silk-shell.
/// Capsules are the unit of execution composition in the orchestration layer.
/// A capsule is bound to a capability domain and has a defined execution lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapsuleHandle(pub u32);

pub struct Canvas {
    fb: &'static mut [u32],
    width: u32,
    height: u32,
}

impl Canvas {
    pub fn new(fb_ptr: *mut u32, w: u32, h: u32) -> Self {
        // Safe slice wrapper — eliminates raw pointer math disaster
        let fb = unsafe { slice::from_raw_parts_mut(fb_ptr, (w * h) as usize) };
        Self { fb, width: w, height: h }
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        let y_end = (y + h).min(self.height);
        let x_end = (x + w).min(self.width);
        for py in y..y_end {
            for px in x..x_end {
                let idx = (py * self.width + px) as usize;
                if idx < self.fb.len() {
                    self.fb[idx] = color;
                }
            }
        }
    }

    pub fn draw_panel(&mut self, state: &ShellState) {
        // silkbar at top
        self.fill_rect(0, 0, self.width, PANEL_HEIGHT, 0xFF0A0A14);
        // launcher area when open
        if state.is_launcher_open {
            self.fill_rect(0, PANEL_HEIGHT, LAUNCHER_WIDTH, 400, 0xFF1E1E2E);
        }
    }
}

/// PDX client for the SexDisplay compositor capability (SLOT_DISPLAY).
/// silk-shell uses this to submit display work on behalf of capsules it orchestrates.
pub struct PdxCompositorClient;

impl PdxCompositorClient {
    pub fn create_window(&self, x: i32, y: i32, w: u32, h: u32) -> u32 {
        pdx_call(SLOT_DISPLAY, OP_WINDOW_CREATE, x as u64, y as u64, w as u64).1 as u32
    }

    pub fn set_bg(&self, color: u32) {
        let _ = pdx_call(SLOT_DISPLAY, OP_SET_BG, color as u64, 0, 0);
    }

    pub fn render_bar(&self, window_id: u32) {
        let _ = pdx_call(SLOT_DISPLAY, OP_RENDER_BAR, window_id as u64, 0, 0);
    }
}
