#![no_std]
#![no_main]

extern crate alloc;
extern crate sex_rt;

mod pdx_io;
mod termion_sex;
use pdx_io::StdStream;

use sex_pdx::pdx_call;

/// Ion SexShell Main Entry
/// Phase 1: Port basic loop and PDX I/O

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut stdout = StdStream::stdout();
    let stdin = StdStream::stdin();

    let welcome = "SexOS Ion Shell v0.1.0 (Zero-Copy PDX)\n$ ";
    let _ = stdout.file.write(welcome.as_bytes());

    let mut line_buf = [0u8; 1024];

    loop {
        // 1. Read input from sexinput via PDX (blocking)
        match stdin.file.read(&mut line_buf) {
            Ok(n) if n > 0 => {
                let cmd = core::str::from_utf8(&line_buf[..n]).unwrap_or("");
                handle_command(cmd, &mut stdout);
                let _ = stdout.file.write(b"\n$ ");
            },
            _ => {
                // Yield or wait for PDX message
            }
        }
    }
}

fn handle_command(cmd: &str, stdout: &mut StdStream) {
    let trimmed = cmd.trim();
    if trimmed.is_empty() { return; }

    // Basic Redirection Support
    if let Some((command, path)) = trimmed.split_once('>') {
        let command = command.trim();
        let path = path.trim();
        
        match pdx_io::PdxFile::open(path, 1 /* O_WRONLY | O_CREAT */) {
            Ok(file) => {
                let mut redirect_out = StdStream { file };
                execute_internal(command, &mut redirect_out);
            },
            Err(_) => {
                let _ = stdout.file.write(b"ion: failed to open file for redirection\n");
            }
        }
        return;
    }

    execute_internal(trimmed, stdout);
}

fn execute_internal(cmd: &str, stdout: &mut StdStream) {
    match cmd {
        "help" => {
            let _ = stdout.file.write(b"Builtins: help, exit, ls, cat\n");
        },
        "exit" => {
            // sys_exit
            let _ = pdx_call(0 /* kernel_cap */, 60 /* EXIT */, 0, 0);
        },
        _ if cmd.starts_with("echo ") => {
            let msg = &cmd[5..];
            let _ = stdout.file.write(msg.as_bytes());
            let _ = stdout.file.write(b"\n");
        },
        _ => {
            // Attempt to spawn process via PDX
            execute_external(cmd, stdout);
        }
    }
}

fn execute_external(cmd: &str, stdout: &mut StdStream) {
    // Phase 1: Simple spawn via kernel
    let mut path = [0u8; 512];
    let cmd_bytes = cmd.as_bytes();
    path[..cmd_bytes.len()].copy_from_slice(cmd_bytes);

    let res = pdx_call(0 /* kernel_cap */, 25 /* SPAWN_PD */, &path as *const _ as u64, 0);
    if (res as i64) < 0 {
        let _ = stdout.file.write(b"ion: command not found\n");
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
