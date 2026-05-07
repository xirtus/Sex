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
