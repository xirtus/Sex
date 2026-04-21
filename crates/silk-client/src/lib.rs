// crates/silk-client/src/main.rs

#![no_std]

pub const SILK_MAGIC: u32 = 0x53454C4B; // 'SILK' - Correction from 'SILK' in log to match typical magic number representation.

#[repr(C)]
pub struct SilkWindow {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub virt_addr: u64, // Virtual address of the window's framebuffer for client drawing
    pub pfn_base: u64, // Physical frame number base of the window's framebuffer
    pub tag_mask: u64, // Tags associated with this window
}

use sex_pdx::{pdx_call, pdx_allocate_memory, pdx_map_memory, pdx_get_framebuffer_info, pdx_move_window, pdx_resize_window, pdx_spawn_pd, pdx_set_window_tags, pdx_get_window_tags, pdx_set_view_tags, pdx_get_view_tags, pdx_commit_window_frame, pdx_set_window_roundness, pdx_set_window_blur, pdx_set_window_animation, SexWindowCreateParams};

impl SilkWindow {
    pub fn create(_title: &str, w: u32, h: u32, initial_tag_mask: u64) -> Result<Self, ()> {
        let buffer_size = (w * h * 4) as u64; // Assuming 4 bytes per pixel

        let pfn_base = pdx_allocate_memory(buffer_size)
            .map_err(|_| ())?;

        let virt_addr = pdx_map_memory(pfn_base, buffer_size)
            .map_err(|_| ())?;

        let create_params = SexWindowCreateParams {
            x: 0, // Initial position, can be changed later
            y: 0,
            width: w,
            height: h,
            pfn_base,
        };

        let window_id = unsafe {
            pdx_call(
                1, // Target PD ID for sexdisplay (assuming PID 1)
                PDX_SEX_WINDOW_CREATE,
                &create_params as *const _ as u64,
                0,
            )
        };

        if window_id == 0 {
            Err(()) // Window creation failed
        } else {
            // Set the initial tag mask for the newly created window
            pdx_set_window_tags(window_id, initial_tag_mask)?;
            Ok(SilkWindow { id: window_id, width: w, height: h, virt_addr, pfn_base, tag_mask: initial_tag_mask })
        }
    }

    pub fn commit(&self, pfn_list: &[u64]) -> Result<(), ()> {
        pdx_commit_window_frame(self.id, pfn_list)
    }

    /// Sets the tag mask for this window.
    pub fn set_tags(&self, tag_mask: u64) -> Result<(), ()> {
        pdx_set_window_tags(self.id, tag_mask)
    }

    /// Gets the current tag mask for this window.
    pub fn get_tags(&self) -> Result<u64, ()> {
        pdx_get_window_tags(self.id)
    }

    /// Moves the window to the specified coordinates.
    pub fn move_to(&self, x: u32, y: u32) -> Result<(), ()> {
        pdx_move_window(self.id, x, y)
    }

    /// Resizes the window to the specified dimensions.
    pub fn resize(&self, width: u32, height: u32) -> Result<(), ()> {
        pdx_resize_window(self.id, width, height)
    }

    /// Requests the compositor to focus this window.
    pub fn focus(&self) -> Result<(), ()> {
        // Needs PDX_FOCUS_WINDOW to be called correctly
        // For now, assume it's like other pdx_calls
        let res = unsafe {
            pdx_call(
                1, // Target PD ID for sexdisplay (assuming PID 1)
                PDX_FOCUS_WINDOW,
                self.id, // arg0 is window ID
                0,
            )
        };
        if res == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Requests the compositor to minimize this window.
    pub fn minimize(&self) -> Result<(), ()> {
        let res = unsafe {
            pdx_call(
                1, // Target PD ID for sexdisplay (assuming PID 1)
                PDX_MINIMIZE_WINDOW,
                self.id, // arg0 is window ID
                0,
            )
        };
        if res == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Requests the compositor to maximize this window.
    pub fn maximize(&self) -> Result<(), ()> {
        let res = unsafe {
            pdx_call(
                1, // Target PD ID for sexdisplay (assuming PID 1)
                PDX_MAXIMIZE_WINDOW,
                self.id, // arg0 is window ID
                0,
            )
        };
        if res == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Requests the compositor to close this window.
    pub fn close(&self) -> Result<(), ()> {
        let res = unsafe {
            pdx_call(
                1, // Target PD ID for sexdisplay (assuming PID 1)
                PDX_CLOSE_WINDOW,
                self.id, // arg0 is window ID
                0,
            )
        };
        if res == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Sets the corner radius of the window.
    pub fn set_roundness(&self, radius: u32) -> Result<(), ()> {
        pdx_set_window_roundness(self.id, radius)
    }

    /// Sets the blur strength for the window.
    pub fn set_blur(&self, strength: u32) -> Result<(), ()> {
        pdx_set_window_blur(self.id, strength)
    }

    /// Sets the animation state for the window.
    pub fn set_animation(&self, is_animating: bool) -> Result<(), ()> {
        pdx_set_window_animation(self.id, is_animating)
    }
}

/// Sets the current view's tag mask for the compositor.
pub fn set_view_tags(tag_mask: u64) -> Result<(), ()> {
    pdx_set_view_tags(tag_mask)
}

/// Gets the current view's tag mask from the compositor.
pub fn get_view_tags() -> Result<u64, ()> {
    pdx_get_view_tags()
}
