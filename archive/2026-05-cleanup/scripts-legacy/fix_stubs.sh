#!/bin/bash
for app_dir in apps/cosmic-*; do
    [ -d "$app_dir" ] || continue
    main_file="$app_dir/src/main.rs"
    if [ -f "$main_file" ]; then
        echo "Updating $main_file with correct SexWindowCreateParams fields"
        cat << 'SRC_EOF' > "$main_file"
#![no_std]
#![no_main]

extern crate alloc;

use sex_pdx::{pdx_call, PDX_SEX_WINDOW_CREATE, SexWindowCreateParams};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let title = b"NativeSexOS";
    let params = SexWindowCreateParams {
        x: 0,
        y: 0,
        w: 800,
        h: 600,
        title: title,
    };
    
    // Convert pointer to u64
    let arg0 = (&params as *const SexWindowCreateParams) as u64;
    
    let _ = unsafe { pdx_call(5, PDX_SEX_WINDOW_CREATE, arg0, 0, 0) };
    
    loop {}
}
SRC_EOF
    fi
done
