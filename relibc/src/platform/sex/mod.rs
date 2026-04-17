use super::types::*;
use super::{Pal, PalSignal, PalSocket};
use sex_rt::{sys_read, sys_write, sys_exit};

pub struct Sys;

impl Pal for Sys {
    fn read(fd: c_int, buf: &mut [u8]) -> ssize_t {
        sys_read(fd as usize, buf) as ssize_t
    }

    fn write(fd: c_int, buf: &[u8]) -> ssize_t {
        sys_write(fd as usize, buf) as ssize_t
    }

    fn exit(status: c_int) -> ! {
        sys_exit(status as usize)
    }
}

impl PalSignal for Sys {}
impl PalSocket for Sys {}
