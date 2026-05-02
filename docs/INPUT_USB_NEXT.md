# INPUT_USB_NEXT

## Current Known-Good Input Path
raw/synthetic report -> HID_POINTER_REPORT_NORMALIZER_V1 -> OP_HID_EVENT -> silk-shell pointer state -> click focus / drag

## Why USB Was Split
USB input is not a single patch. It spans:
- host controller discovery
- XHCI initialization
- device enumeration
- endpoint configuration
- interrupt transfers
- HID report fetching
- report normalization
- OP_HID_EVENT delivery
- shell policy consumption

## USB NO-GO List
Do not combine these into one phase:
- full USB subsystem rewrite
- XHCI + HID + gestures + compositor behavior in one patch
- Bluetooth
- PS/2 product path
- trackpad gestures
- drag/click policy changes
- surface protocol changes
- Linen/file browser work
- scheduler/PKRU/time changes
- backing buffer/shared memory work

## Next Phases
1. `USB_HOST_DISCOVERY_V1`
- inspect current PCI/MMIO/IRQ/DMA capability reality
- identify existing USB/XHCI code if any
- no implementation unless trivial diagnostic logging

2. `USB_XHCI_MINIMAL_ENUM_V1`
- minimal controller bring-up
- enumerate one device if feasible
- no HID policy yet

3. `USB_HID_BOOT_MOUSE_REPORT_V1`
- obtain fixed boot-protocol mouse-like reports
- feed bytes to existing normalizer

4. `USB_HID_POINTER_PRODUCER_V1`
- route real reports into OP_HID_EVENT
- prove click-focus/drag with real hardware or QEMU USB tablet/mouse

5. `TOUCHPAD_ABS_CONTACT_V1`
- later absolute/contact events
- no gestures

6. `TRACKPAD_GESTURES_V1`
- later policy: scroll/swipe/workspace gestures

## Phase Gate Rule
If a proposed USB patch touches kernel, sexinput, sex-pdx, silk-shell, sexdisplay, and build spec all at once, reject and split it.

## Success Criteria Before USB Producer
- Existing keyboard controls still work.
- Synthetic pointer producer still works.
- HID normalizer still converts fixed reports.
- click-focus and drag remain shell-only policy.
- no #GP/#PF/panic.
- no IPC storm.
