use limine::request::MpResponse;
use limine::mp::MpInfo;
use core::sync::atomic::Ordering;

pub fn boot_aps(smp: &MpResponse) {
    let bsp_id = smp.bsp_lapic_id;
    for cpu in smp.cpus() {
        if cpu.lapic_id == bsp_id {
            continue;
        }
        cpu.bootstrap(limine_ap_entry, 0);
    }
}

extern "C" fn limine_ap_entry(info: &MpInfo) -> ! {
    let _lapic_id = info.lapic_id;
    // Kernel initialization per core goes here
    loop { x86_64::instructions::hlt(); }
}
