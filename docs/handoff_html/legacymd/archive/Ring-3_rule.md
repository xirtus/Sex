1. GDT & TSS Array Limits (The 16-Byte Rule)
The x86_64 crate has a strict 8-slot GDT limit. A Task State Segment (TSS) in long mode is a "System Segment" requiring two contiguous 8-byte slots (16 bytes).

MANDATORY GDT ORDER: * Slot 0: Null

Slot 1: Kernel Code

Slot 2: Kernel Data

Slot 3 & 4: TSS (MUST be inserted here, before User segments, to prevent array overflow).

Slot 5: User Data (SS)

Slot 6: User Code (CS)

2. SYSRET Mathematical Offsets
The syscall instruction strictly calculates segments. x86_64::registers::model_specific::Star::write will throw a SysretOffset panic if indices violate this math:

Kernel SS Index MUST be Kernel CS + 1 (Index 2 = 1 + 1).

User CS Index MUST be User SS + 1 (Index 6 = 5 + 1).

Never pass user_data_selector as the Kernel SS parameter.

3. Ring-3 Context Switch (IRETQ)

The RPL Drop: When forging the interrupt stack frame in Task::new(), user selectors MUST explicitly be bitwise-OR'd with the Ring Privilege Level 3 (| 3).

User CS must evaluate to 0x33.

User SS must evaluate to 0x2B.

Failure to add the RPL causes an instant #GP(0x30).

The Stack Bomb: If using a custom stub (e.g., timer_interrupt_stub) before iretq, Task::new() must push exactly 15 dummy zeros onto the task stack on top of the hardware frame. Otherwise, the stub's pop r15 ... pop rdi sequence will literally eat the iretq frame, misaligning the stack.

*** ### Why this works for LLMs:

The "MANDATORY" phrasing: AI models are trained to follow explicit negative constraints ("Do not violate", "MANDATORY").

Pre-empting the Math: Explaining why Star::write panics prevents the LLM from trying to "hack" the GDT order in a way that breaks the sysret math.

Consolidated Fixes: It packages the RPL fix, the Stack alignment, the GDT limit, and the Syscall offsets into one token-light summary.
