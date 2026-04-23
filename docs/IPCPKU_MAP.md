ince Intel MPK physically limits you to 16 Protection Keys (0-15) per CPU thread, your core system servers must be meticulously mapped. PKEY 0 is the Kernel's default space, leaving you 15 hardware-locked domains for your SASOS userland.Here is the proposed Phase 25 Architectural Map for your Protection Domains and Capability Slots. You should drop this directly into your ARCHITECTURE.md.1. The Hardware Topology (PKU/PDX Map)This defines the physical memory boundary. Every process gets a strictly assigned Protection Domain (PD) ID and a corresponding hardware PKEY.PD IDIntel PKEYSubsystem / Server NameRole & Responsibility0PKEY 0sexos-kernelRing-0 Supervisor. Handles interrupts, PDX routing, and scheduling.1PKEY 1sexdisplayCompositor. Owns the framebuffer. Paints windows, routes pixels.2PKEY 2linenFile Manager. Manages user files. (May require bump allocator later).3PKEY 3silk-shellDesktop Shell. Renders the panel (silkbar), launcher, and desktop bg.4PKEY 4sexinputHID Server. Reads PS/2 or USB mouse/keyboard and routes events.5PKEY 5sexdriveStorage / VFS. Owns the NVMe driver and manages the file system tree.6PKEY 6sexnetNetwork Stack. Owns the NIC. TCP/IP stack.7PKEY 7sexaudioAudio Server. Routes PCM streams to the sound card.8-15PKEY 8-15Dynamic AppsRotating keys assigned to short-lived or untrusted user applications.2. The IPC Capability Routing Table (The "Well-Known" Slots)Instead of relying on random capability indexes, standardize your slot numbers across the entire OS. When the kernel spawns a new PD, it should populate the local capability table so that these slots always point to the correct destination PD's ring buffer.Slot IDEndpoint TargetExpected MessageType Opcodes (Hex)Slot 1sexdrive (Storage)0x10 (OPEN), 0x11 (READ), 0x12 (WRITE)Slot 2sexnet (Network)0x20 (CONNECT), 0x21 (SEND), 0x22 (RECV)Slot 3sexinput (HID)0x30 (SUBSCRIBE_MOUSE), 0x31 (SUBSCRIBE_KEY)Slot 4sexaudio (Audio)0x40 (PLAY_STREAM), 0x41 (SET_VOL)Slot 5sexdisplay (Video)0xDE (WINDOW_CREATE), 0xDF (WINDOW_DESTROY)Slot 6silk-shell (Shell)0x100 (SET_BG), 0x101 (RENDER_BAR)Slot 7-15App-to-AppDynamically negotiated peer-to-peer capability slots.3. The Implementation StrategyTo stop writing unsafe { pdx_call(5, ...) } in every single main.rs, you should update your sex_pdx crate to act as the single source of truth.Add this to crates/sex-pdx/src/lib.rs:Rust// Standardized Capability Slots
pub const SLOT_STORAGE: u64 = 1;
pub const SLOT_NETWORK: u64 = 2;
pub const SLOT_INPUT: u64   = 3;
pub const SLOT_AUDIO: u64   = 4;
pub const SLOT_DISPLAY: u64 = 5;
pub const SLOT_SHELL: u64   = 6;

// Compositor Opcodes (Slot 5)
pub const OP_WINDOW_CREATE: u64 = 0xDE;
pub const OP_WINDOW_PAINT: u64  = 0xDF;

// Shell Opcodes (Slot 6)
pub const OP_SET_BG: u64        = 0x100;
pub const OP_RENDER_BAR: u64    = 0x101;
Then, inside kernel/src/init.rs, when you spawn a new PD, you dynamically map its capability array exactly to this map:Rust// In kernel/src/init.rs
pd.grant_capability(SLOT_DISPLAY, CapabilityData::Domain(sexdisplay_pd_id));
pd.grant_capability(SLOT_SHELL, CapabilityData::Domain(silkshell_pd_id));
// ... etc
Next StepsWith this map in place, your linen binary can safely use SLOT_DISPLAY without guessing. Do you want to standardize these constants in sex_pdx now, or do you want to write the silk-shell stub to get the desktop panel rendering first?
