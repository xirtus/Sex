use crate::serial_println;
use crate::servers::sexc::sexc;

/// Sample POSIX Application running in its own Protection Domain.
/// This uses the sexc emulation layer to interact with the OS.

pub fn posix_app_main(pd_id: u32) {
    serial_println!("APP: [PD {}] Starting POSIX application...", pd_id);

    // 1. Initialize sexc for the current PD
    let libc = sexc::new(pd_id);

    // 2. Demonstrate POSIX write() to stdout (FD 1)
    let msg = "Hello from sexc! This is a POSIX-like environment.\n";
    libc.write(1, msg.as_ptr(), msg.len()).expect("APP: Write failed");

    // 3. Demonstrate POSIX open() and read() from the sexvfs
    serial_println!("APP: Opening /disk0/config.json...");
    match libc.open("/disk0/config.json", 0) {
        Ok(fd) => {
            serial_println!("APP: File opened with FD: {}", fd);

            // 4. Read from the file via the Node Capability (safe_pdx_call)
            let mut buf = [0u8; 32];
            match libc.read(fd, buf.as_mut_ptr(), buf.len()) {
                Ok(_) => {
                    serial_println!("APP: Successfully read from sexvfs via sexc.");
                },
                Err(e) => serial_println!("APP: Read error: {}", e),
            }

            libc.close(fd);
        },
        Err(e) => serial_println!("APP: Open error: {}", e),
    }

    serial_println!("APP: POSIX application exiting.");
}
