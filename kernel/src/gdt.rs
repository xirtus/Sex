use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use lazy_static::lazy_static;
use crate::serial_println;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(unsafe { &raw const STACK as *const u8 });
            stack_start + (STACK_SIZE as u64)
        };
        tss
    };
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let kernel_cs = gdt.append(Descriptor::kernel_code_segment());
        let kernel_ss = gdt.append(Descriptor::kernel_data_segment());
        let _compat = gdt.append(Descriptor::user_data_segment());
        let user_ss = gdt.append(Descriptor::user_data_segment());
        let user_cs = gdt.append(Descriptor::user_code_segment());
        let tss = gdt.append(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { kernel_cs, kernel_ss, user_cs, user_ss, tss })
    };
}

pub struct Selectors {
    pub kernel_cs: SegmentSelector,
    pub kernel_ss: SegmentSelector,
    pub user_cs:   SegmentSelector,
    pub user_ss:   SegmentSelector,
    pub tss:       SegmentSelector,
}

pub fn init() {
    GDT.0.load();
    unsafe {
        use x86_64::registers::segmentation::{CS, Segment};
        CS::set_reg(GDT.1.kernel_cs);
        x86_64::instructions::tables::load_tss(GDT.1.tss);
    }
}

pub fn get_selectors() -> Selectors {
    Selectors {
        kernel_cs: GDT.1.kernel_cs,
        kernel_ss: GDT.1.kernel_ss,
        user_cs:   GDT.1.user_cs,
        user_ss:   GDT.1.user_ss,
        tss:       GDT.1.tss,
    }
}

pub fn update_tss_rsp0(new_rsp0: VirtAddr) {
    unsafe {
        let tss_ptr = &TSS as *const _ as *mut TaskStateSegment;
        (*tss_ptr).privilege_stack_table[0] = new_rsp0;
    }
}
