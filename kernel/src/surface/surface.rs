#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceNode {
    pub surface_id: u32,
    pub capability_id: u32,
    pub z_index: i32,
    pub bounds: [i32; 4], // x, y, w, h
}
