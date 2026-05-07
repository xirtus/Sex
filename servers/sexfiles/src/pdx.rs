//! Thin wrappers over sex-pdx for sexfiles server.
//! Uses standard pdx_listen_raw(0) / pdx_reply pattern.

pub use sex_pdx::{
    pdx_listen_raw, pdx_reply, serial_println,
    SLOT_BLOCK,
    BLOCK_ERR_BAD_CMD, BLOCK_ERR_BAD_LEN, BLOCK_ERR_NO_DEVICE,
};
