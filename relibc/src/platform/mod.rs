pub mod sex;
pub use self::sex::*;

pub trait Pal {
    fn read(fd: i32, buf: &mut [u8]) -> isize;
    fn write(fd: i32, buf: &[u8]) -> isize;
    fn exit(status: i32) -> !;
}

pub trait PalSignal {}
pub trait PalSocket {}

pub mod types {
    #[allow(non_camel_case_types)]
    pub type c_int = i32;
    #[allow(non_camel_case_types)]
    pub type ssize_t = isize;
}
