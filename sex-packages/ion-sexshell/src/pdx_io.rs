use sex_pdx::pdx_call;
use sex_pdx::MessageType;
use sex_pdx::PageHandover;

/// PDX-based I/O for Ion Shell.
/// Uses VFS zero-copy handover for large buffers.

pub struct PdxFile {
    pub fd: u64,
    pub server_pd: u32,
}

impl PdxFile {
    pub fn open(path: &str, flags: u32) -> Result<Self, i64> {
        let mut path_buf = [0u8; 512];
        let bytes = path.as_bytes();
        let len = bytes.len().min(511);
        path_buf[..len].copy_from_slice(&bytes[..len]);

        let res = pdx_call(
            1, // Assuming sexfiles is PD 1 for now (mapped via name server later)
            1, // VfsOpen
            &path_buf as *const _ as u64,
            flags as u64,
        );

        if (res as i64) < 0 {
            Err(res as i64)
        } else {
            Ok(Self { fd: res, server_pd: 1 })
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, i64> {
        if buf.len() > 4096 {
            self.read_zero_copy(buf)
        } else {
            // Short read via direct PDX call
            let res = pdx_call(
                self.server_pd,
                2, // VfsRead
                self.fd,
                buf.as_mut_ptr() as u64 | ((buf.len() as u64) << 32),
            );
            if (res as i64) < 0 { Err(res as i64) } else { Ok(res as usize) }
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, i64> {
        if buf.len() > 4096 {
            self.write_zero_copy(buf)
        } else {
            let res = pdx_call(
                self.server_pd,
                3, // VfsWrite
                self.fd,
                buf.as_ptr() as u64 | ((buf.len() as u64) << 32),
            );
            if (res as i64) < 0 { Err(res as i64) } else { Ok(res as usize) }
        }
    }

    fn read_zero_copy(&self, buf: &mut [u8]) -> Result<usize, i64> {
        // Phase 18: HandoverRead
        let pfn = (buf.as_ptr() as u64) >> 12;
        let page = PageHandover { pfn, pku_key: 0 };

        let msg = MessageType::VfsRead { fd: self.fd, len: buf.len() as u64, offset: 0 };
        let res = pdx_call(
            self.server_pd,
            &msg as *const _ as u64,
            &page as *const _ as u64,
            buf.len() as u64,
        );
        if (res as i64) < 0 { Err(res as i64) } else { Ok(res as usize) }
    }

    fn write_zero_copy(&self, buf: &[u8]) -> Result<usize, i64> {
        // Phase 18: HandoverWrite
        let pfn = (buf.as_ptr() as u64) >> 12;
        let page = PageHandover { pfn, pku_key: 0 };

        let msg = MessageType::VfsWrite { fd: self.fd, len: buf.len() as u64, offset: 0 };
        let res = pdx_call(
            self.server_pd,
            &msg as *const _ as u64,
            &page as *const _ as u64,
            buf.len() as u64,
        );
        if (res as i64) < 0 { Err(res as i64) } else { Ok(res as usize) }
    }
}

pub struct StdStream {
    pub file: PdxFile,
}

impl StdStream {
    pub fn stdin() -> Self { Self { file: PdxFile { fd: 0, server_pd: 1 } } }
    pub fn stdout() -> Self { Self { file: PdxFile { fd: 1, server_pd: 1 } } }
    pub fn stderr() -> Self { Self { file: PdxFile { fd: 2, server_pd: 1 } } }
}
