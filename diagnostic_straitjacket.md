# Diagnostic Straitjacket Plan (Revised v2)

## Objective
Fix the hang during scheduler-led handoff by ensuring the LAPIC timer is correctly initialized and firing.

## Key Files & Context
- `kernel/src/lib.rs`: Add debug prints for RSDP.
- `kernel/src/hal/x86_64.rs`: Add debug prints for `init_advanced`.
- `kernel/src/interrupts.rs`: Refine heartbeat print.

## Implementation Steps

### 1. Debug Prints in `kernel/src/lib.rs`
Check if `RSDP_REQUEST` is successful.

```rust
    // 3. Advanced Hardware (APIC + Timer)
    let rsdp_res = RSDP_REQUEST.response();
    serial_println!("kernel: RSDP Response present: {}", rsdp_res.is_some());
    let rsdp_addr = rsdp_res.map(|r| r.address() as usize).unwrap_or(0);
    serial_println!("kernel: RSDP Address: {:#x}", rsdp_addr);
    hal::init_advanced(rsdp_addr as u64, hhdm.offset);
```

### 2. Debug Prints in `kernel/src/hal/x86_64.rs`
Verify `init_advanced` is executing.

```rust
    fn init_advanced(&self, rsdp_addr: u64, hhdm_offset: u64) {
        serial_println!("X86Hal: init_advanced(rsdp={:#x}, hhdm={:#x})", rsdp_addr, hhdm_offset);
        if rsdp_addr != 0 {
            // ...
        }
    }
```

### 3. Refine Heartbeat in `kernel/src/interrupts.rs`
Use a more robust raw serial write.

```rust
extern "x86-interrupt" fn timer_interrupt_handler(stack_frame: InterruptStackFrame) {
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port = Port::new(0x3f8);
        for &c in b"TICK\n" {
            port.write(c);
        }
    }
    // ...
}
```

## Verification & Testing
1. Rebuild and boot.
2. Check if "kernel: RSDP Response present: true" appears.
3. Check if "TICK" appears.
