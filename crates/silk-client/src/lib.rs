#![no_std]

pub const SILK_MAGIC: u32 = 0x53454C4B;

#[repr(C)]
pub struct SilkWindow {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub virt_addr: u64,
    pub pfn_base: u64,
    pub tag_mask: u64,
}

pub use sex_pdx::{
    pdx_call, pdx_allocate_memory, pdx_map_memory, pdx_get_framebuffer_info,
    pdx_move_window, pdx_resize_window, pdx_spawn_pd,
    pdx_set_window_tags, pdx_get_window_tags,
    pdx_set_view_tags, pdx_get_view_tags,
    pdx_commit_window_frame, pdx_set_window_roundness,
    pdx_set_window_blur, pdx_set_window_animation,
    SexWindowCreateParams,
    PDX_SEX_WINDOW_CREATE, PDX_ALLOCATE_MEMORY, PDX_MAP_MEMORY,
    PDX_FOCUS_WINDOW, PDX_MINIMIZE_WINDOW, PDX_MAXIMIZE_WINDOW, PDX_CLOSE_WINDOW,
};

pub trait SexApp {
    fn new(pdx: sex_pdx::Pdx) -> Self;
    fn run(&mut self, pdx: sex_pdx::Pdx) -> bool;
}

#[macro_export]
macro_rules! app_main {
    ($app:ty) => {
        #[no_mangle]
        pub extern "C" fn _start() -> ! {
            let mut app = <$app as $crate::SexApp>::new(0);
            loop {
                if !app.run(0) { break; }
            }
            unsafe { sex_pdx::pdx_call(0, 0xFF, 0, 0); }
            loop {}
        }
    };
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let s1 = core::slice::from_raw_parts(s1, n);
    let s2 = core::slice::from_raw_parts(s2, n);
    for i in 0..n {
        if s1[i] != s2[i] { return s1[i] as i32 - s2[i] as i32; }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let dest_s = core::slice::from_raw_parts_mut(dest, n);
    let src_s = core::slice::from_raw_parts(src, n);
    dest_s.copy_from_slice(src_s);
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let s_s = core::slice::from_raw_parts_mut(s, n);
    for i in 0..n { s_s[i] = c as u8; }
    s
}

impl SilkWindow {
    pub fn create(_title: &str, w: u32, h: u32, initial_tag_mask: u64) -> Result<Self, ()> {
        let buffer_size = (w * h * 4) as u64;

        let pfn_base = pdx_allocate_memory(PDX_ALLOCATE_MEMORY, buffer_size)
            .map_err(|_| ())?;
        let virt_addr = pdx_map_memory(PDX_MAP_MEMORY, pfn_base, buffer_size)
            .map_err(|_| ())?;

        let create_params = SexWindowCreateParams { x: 0, y: 0, width: w, height: h, pfn_base };

        let window_id = unsafe {
            pdx_call(1, PDX_SEX_WINDOW_CREATE, &create_params as *const _ as u64, 0)
        };

        if window_id == 0 {
            Err(())
        } else {
            pdx_set_window_tags(window_id, initial_tag_mask)?;
            Ok(SilkWindow { id: window_id, width: w, height: h, virt_addr, pfn_base, tag_mask: initial_tag_mask })
        }
    }

    pub fn commit(&self, pfn_list: &[u64]) -> Result<(), ()> { pdx_commit_window_frame(self.id, pfn_list) }
    pub fn set_tags(&self, mask: u64) -> Result<(), ()>       { pdx_set_window_tags(self.id, mask) }
    pub fn get_tags(&self) -> Result<u64, ()>                  { pdx_get_window_tags(self.id) }
    pub fn move_to(&self, x: u32, y: u32) -> Result<(), ()>   { pdx_move_window(self.id, x, y) }
    pub fn resize(&self, w: u32, h: u32) -> Result<(), ()>    { pdx_resize_window(self.id, w, h) }

    pub fn focus(&self) -> Result<(), ()> {
        if unsafe { pdx_call(1, PDX_FOCUS_WINDOW, self.id, 0) } == 0 { Ok(()) } else { Err(()) }
    }
    pub fn minimize(&self) -> Result<(), ()> {
        if unsafe { pdx_call(1, PDX_MINIMIZE_WINDOW, self.id, 0) } == 0 { Ok(()) } else { Err(()) }
    }
    pub fn maximize(&self) -> Result<(), ()> {
        if unsafe { pdx_call(1, PDX_MAXIMIZE_WINDOW, self.id, 0) } == 0 { Ok(()) } else { Err(()) }
    }
    pub fn close(&self) -> Result<(), ()> {
        if unsafe { pdx_call(1, PDX_CLOSE_WINDOW, self.id, 0) } == 0 { Ok(()) } else { Err(()) }
    }
    pub fn set_roundness(&self, r: u32) -> Result<(), ()>    { pdx_set_window_roundness(self.id, r) }
    pub fn set_blur(&self, s: u32) -> Result<(), ()>         { pdx_set_window_blur(self.id, s) }
    pub fn set_animation(&self, a: bool) -> Result<(), ()>   { pdx_set_window_animation(self.id, a) }
}

pub fn set_view_tags(mask: u64) -> Result<(), ()> { pdx_set_view_tags(mask) }
pub fn get_view_tags() -> Result<u64, ()>          { pdx_get_view_tags() }
