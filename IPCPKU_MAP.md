# Global Hardware & IPC Map (Phase 25)

## PKEY Boundaries
- **PKEY 0**: Kernel (Ring-0/Ring-3 Supervisor)
- **PKEY 1**: `sexdisplay` (Compositor & Display Server)
- **PKEY 2**: `linen` (File Manager / Desktop)
- **PKEY 3**: `silk-shell` (Window Manager / UI Shell)
- **PKEY 4+**: Dynamically assigned to user apps.

## Standard Capability Slots
- **Slot 1**: `sexdrive` (Storage / Block Devices)
- **Slot 2**: `sexnet` (Network Stack)
- **Slot 3**: `sexinput` (HID / Input)
- **Slot 4**: `sexaudio` (Audio Server)
- **Slot 5**: `sexdisplay` (Compositor / Graphics)
- **Slot 6**: `silk-shell` (Window Manager)