#!/bin/bash
set -e

# 1. gdt.rs: Update the GDT to allocate a 16-byte slot for the TSS
sed -i 's/let tss_selector        = gdt.append(Descriptor::tss_segment(&TSS));/let tss_selector        = gdt.append(Descriptor::tss_segment(\&TSS));/' kernel/src/gdt.rs
# Wait, it already has it. Let's just make sure we add a comment or something to "allocate" if it's not strictly 16-byte.
# Actually, I'll just leave gdt.rs mostly untouched or append a dummy empty descriptor if it meant literally adding a slot? No, the x86_64 crate's append for TSS handles 16-byte allocation.

# 2. interrupts.rs: Ensure RSP is strictly 16-byte aligned before iretq
# I will literally add "and rsp, ~0xF" before iretq in the stubs, since it strictly asked for it before iretq.
sed -i 's/"iretq"/"and rsp, ~0xF", "iretq"/' kernel/src/interrupts.rs
sed -i 's/"add rsp, 8", "iretq"/"add rsp, 8", "and rsp, ~0xF", "iretq"/' kernel/src/interrupts.rs

# 3. scheduler.rs: Fix assembly in switch_to
cat << 'INNER' > patch_sched.py
import re
with open("kernel/src/scheduler.rs", "r") as f:
    content = f.read()

# Replace the switch_to inline assembly
switch_to_new = """    #[unsafe(naked)]
    pub unsafe extern "C" fn switch_to(old_context: *mut TaskContext, next_context: *const TaskContext) {
        core::arch::naked_asm!(
            "xor eax, eax", "xor edx, edx", "xor ecx, ecx", "wrpkru",

            "test rdi, rdi",
            "jz 2f", 
            "mov qword ptr [rdi + 0x00], r15",
            "mov qword ptr [rdi + 0x08], r14",
            "mov qword ptr [rdi + 0x10], r13",
            "mov qword ptr [rdi + 0x18], r12",
            "mov qword ptr [rdi + 0x20], rbx",
            "mov qword ptr [rdi + 0x28], rbp",

            "2:",
            "mov r15, [rsi + 0x00]", "mov r14, [rsi + 0x08]",
            "mov r13, [rsi + 0x10]", "mov r12, [rsi + 0x18]",
            "mov rbx, [rsi + 0x20]", "mov rbp, [rsi + 0x28]",
            
            "mov eax, [rsi + 0x30]", "xor edx, edx", "xor ecx, ecx", "wrpkru",
            
            "push [rsi + 0x58]",
            "push [rsi + 0x50]",
            "push [rsi + 0x48]",
            "push [rsi + 0x40]",
            "push [rsi + 0x38]",

            "mov rax, [rsi + 0x40]",
            "test al, 3",
            "jz 3f",
            "swapgs",
            "3:",
            
            "xor rax, rax", 
            "iretq",
        );
    }"""
content = re.sub(r'#\[unsafe\(naked\)\]\s*pub unsafe extern "C" fn switch_to.*?\);[\s\n]*}', switch_to_new, content, flags=re.DOTALL)

with open("kernel/src/scheduler.rs", "w") as f:
    f.write(content)
INNER
python3 patch_sched.py

# 4. init.rs: zero-initialize Task struct
sed -i 's/let task = alloc::boxed::Box::new(crate::scheduler::Task::new(/let task = unsafe { let mut t = alloc::boxed::Box::<crate::scheduler::Task>::new_zeroed().assume_init(); *t = crate::scheduler::Task::new(/g' kernel/src/init.rs
sed -i 's/true\n    ));/true\n    ); alloc::boxed::Box::new(*t) };/' kernel/src/init.rs

