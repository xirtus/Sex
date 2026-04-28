# SexOS Aider Workflow Agents

This file defines reusable prompt roles (agents) for the Aider AI coding assistant
when working on the SexOS kernel, servers, and capability‑layer crates.

Agent names are tool/task based, not model based. Aider may use Qwen, DeepSeek,
Claude, or another backend.

---

## Global rules (apply to every agent)

- Use the smallest useful file set.
- Never `/add` the whole repo.
- Prefer 3–8 files max.
- Do not redesign architecture.
- Do not perform broad refactors.
- Prefer smallest patch first.
- Report changed files, test command, result, risks, and next action.
- Use `grep`/`ripgrep` externally before adding files when possible.
- Clear/drop context between unrelated tasks.
- Do **not** modify `kernel/src/syscalls/mod.rs`, `servers/sexdisplay/src/main.rs`,
  or `crates/sex-pdx/src/lib.rs` unless explicitly instructed in a later task.

---

## aider-build-fixer

- **Purpose**  
  Diagnose and fix build errors in the SexOS workspace (kernel, servers, crates).
  Understands the `#![no_std]` environment, custom linker scripts, and the
  `x86_64` target.

- **When to use**  
  After a `cargo build` or `cargo check` fails with a compilation error,
  linker error, or missing symbol.

- **Files to add to Aider**  
  - `Cargo.toml` (workspace root)  
  - `kernel/Cargo.toml`  
  - `kernel/src/**/*.rs` (only the files that appear in the error)  
  - `servers/sexdisplay/Cargo.toml`  
  - `servers/sexdisplay/src/**/*.rs` (only the files that appear in the error)  
  - `crates/sex-pdx/Cargo.toml`  
  - `crates/sex-pdx/src/**/*.rs` (only the files that appear in the error)  
  - `rust-toolchain.toml` (if present)  
  - `linker.ld` or any `.ld` files

- **Exact launch prompt**  
  ```
  /chat You are aider-build-fixer. The workspace is a no_std x86_64 kernel.
  The build just failed. Show me the exact error and propose a minimal fix.
  Do not change any file unless I explicitly approve the change.
  ```

- **Token-saving routine**  
  - Only add the files that contain the error location.  
  - Use `grep` to find the relevant lines before adding files.  
  - Do not add the entire `kernel/src/` directory.

- **Max-efficiency workflow**  
  1. Run `cargo check 2>&1 | head -50` and paste the output.  
  2. Use `grep -rn 'error[E0'` to locate the exact file and line.  
  3. `/add` only that file (plus `Cargo.toml` if needed).  
  4. Ask for the fix.  
  5. Apply the diff, run `cargo check` again, report result.

- **Rules / forbidden behavior**  
  - Must not introduce `std` dependencies.  
  - Must not change linker scripts without asking.  
  - Must not remove `#![no_std]` or `#![no_main]` attributes.  
  - Must explain the root cause before suggesting a fix.

---

## aider-framebuffer-tracer

- **Purpose**  
  Trace framebuffer operations inside `sexdisplay` and the kernel’s display
  pipeline. Helps debug missing flips, wrong pixel data, or VBlank issues.

- **When to use**  
  When the screen shows garbage, no updates, or the compositor reports
  unexpected frame states.

- **Files to add to Aider**  
  - `servers/sexdisplay/src/main.rs`  
  - `kernel/src/syscalls/mod.rs` (only the display‑related sections)  
  - `crates/sex-pdx/src/lib.rs` (the `Window`, `FrameState`, `OP_WINDOW_*` constants)

- **Exact launch prompt**  
  ```
  /chat You are aider-framebuffer-tracer. I need to understand why the
  framebuffer is not flipping. Walk me through the code path from
  OP_WINDOW_SUBMIT to the actual hardware flip. Point out any missing
  steps or incorrect state transitions.
  ```

- **Token-saving routine**  
  - Only add the three files listed above.  
  - Use `grep -n 'OP_WINDOW_SUBMIT\|op_window_submit\|FrameState'` to focus on
    relevant lines.

- **Max-efficiency workflow**  
  1. `/add` the three files.  
  2. Ask the agent to trace the path.  
  3. If a bug is identified, ask for a one‑line fix.  
  4. Apply, rebuild, test.

- **Rules / forbidden behavior**  
  - Must not propose changes to the kernel’s PCI/MMIO code.  
  - Must not suggest using `std::` or `alloc`.  
  - Must reference the exact line numbers in the files I added.

---

## aider-pdx-sync

- **Purpose**  
  Ensure that the PDX capability‑slot numbers, message type IDs, and
  syscall numbers are consistent across `crates/sex-pdx/src/lib.rs`,
  `kernel/src/syscalls/mod.rs`, and any server that uses them.

- **When to use**  
  After adding a new capability slot, a new message type, or a new syscall
  number. Also when a mismatch causes `ERR_CAP_INVALID` or `ERR_SERVICE_NOT_READY`.

- **Files to add to Aider**  
  - `crates/sex-pdx/src/lib.rs`  
  - `kernel/src/syscalls/mod.rs`  
  - `servers/sexdisplay/src/main.rs`  
  - Any other server that imports `sex_pdx`

- **Exact launch prompt**  
  ```
  /chat You are aider-pdx-sync. Compare the PDX constants in
  crates/sex-pdx/src/lib.rs with the syscall dispatch in
  kernel/src/syscalls/mod.rs and the usage in servers/sexdisplay/src/main.rs.
  List every mismatch and propose a single consistent set of values.
  ```

- **Token-saving routine**  
  - Only add the three files listed above.  
  - Use `grep -n 'const\|pub const\|SLOT_\|OP_\|ERR_'` to extract constants
    before adding files.

- **Max-efficiency workflow**  
  1. `/add` the three files.  
  2. Ask for the mismatch list.  
  3. Apply the smallest diff that fixes all mismatches.  
  4. Run `cargo check` and report.

- **Rules / forbidden behavior**  
  - Must not change the semantics of existing slots (only fix mismatches).  
  - Must not introduce new constants without asking.  
  - Must produce a diff that can be applied with `git apply`.

---

## aider-log-agent

- **Purpose**  
  Insert, remove, or adjust `serial_println!` calls to help debug a
  specific subsystem without affecting the rest of the codebase.

- **When to use**  
  When you need to trace a particular function or event but don’t want
  to pollute the log with unrelated messages.

- **Files to add to Aider**  
  Any `.rs` file that contains the code you want to trace.

- **Exact launch prompt**  
  ```
  /chat You are aider-log-agent. I need to trace the execution of
  [function name] in [file path]. Add a single serial_println! call
  at the entry and at each return point. Use the format
  "[module] [function] entered / returned". Do not change any other
  logic. After I confirm, remove the added lines.
  ```

- **Token-saving routine**  
  - Only add the single file that contains the function.  
  - Use `grep -n 'fn [function_name]'` to locate the exact line.

- **Max-efficiency workflow**  
  1. `/add` the file.  
  2. Ask for the log lines.  
  3. Apply, rebuild, test.  
  4. After debugging, ask the agent to remove the added lines.

- **Rules / forbidden behavior**  
  - Must not change any logic or data structures.  
  - Must not add `use` statements that aren’t already present.  
  - Must revert the added lines when asked.

---

## aider-doc-sync

- **Purpose**  
  Keep the documentation files (`ARCHITECTURE.md`, `README.md`, etc.)
  in sync with the actual code. Detects stale comments, outdated
  descriptions, and missing sections.

- **When to use**  
  After a significant refactor or before a release.

- **Files to add to Aider**  
  - `ARCHITECTURE.md`  
  - `README.md`  
  - `docs/*.md`  
  - Any `.rs` file that contains doc comments that should match the docs

- **Exact launch prompt**  
  ```
  /chat You are aider-doc-sync. Compare the doc comments in the source
  files I added with the content of ARCHITECTURE.md and README.md.
  List every discrepancy and propose a corrected version of the
  documentation. Do not change the source code.
  ```

- **Token-saving routine**  
  - Only add the documentation files and the `.rs` files that contain
    relevant doc comments.  
  - Use `grep -rn '///'` to find doc comments before adding files.

- **Max-efficiency workflow**  
  1. `/add` the documentation files and the `.rs` files.  
  2. Ask for the discrepancy list.  
  3. Apply the documentation changes.  
  4. Run `cargo doc` (if available) to verify.

- **Rules / forbidden behavior**  
  - Must not modify any `.rs` file.  
  - Must not change the structure of the documentation (only content).  
  - Must preserve the existing Markdown formatting.

---

## aider-unsafe-auditor

- **Purpose**  
  Audit `unsafe` blocks in a given file or set of files for correctness,
  safety invariants, and adherence to the SexOS capability model.

- **When to use**  
  Before merging a pull request that touches `unsafe` code, or when a
  crash is suspected to originate from an `unsafe` block.

- **Files to add to Aider**  
  The `.rs` file(s) that contain the `unsafe` blocks to audit.

- **Exact launch prompt**  
  ```
  /ask You are aider-unsafe-auditor. Review every unsafe block in the
  files I added. For each block, state:
  - The safety invariant it relies on.
  - Whether the invariant is upheld by the surrounding code.
  - Any missing safety comments.
  - Any potential UB.
  Do not suggest changes unless the code is incorrect.
  ```

- **Token-saving routine**  
  - Only add the file(s) that contain the `unsafe` blocks.  
  - Use `grep -n 'unsafe'` to locate the blocks before adding files.

- **Max-efficiency workflow**  
  1. `/add` the file(s).  
  2. Ask for the audit.  
  3. If issues are found, ask for the smallest fix.  
  4. Apply, rebuild, test.

- **Rules / forbidden behavior**  
  - Must not propose changes (only audit).  
  - Must not assume the code is correct.  
  - Must flag any use of `unsafe` that is not justified by a comment.  
  - Must check that new constants do not conflict with existing ones.

---

## aider-test-writer

- **Purpose**  
  Write unit tests or integration tests for a given function or module,
  following the existing test patterns in the SexOS codebase.

- **When to use**  
  When adding a new function or fixing a bug that should be covered by a test.

- **Files to add to Aider**  
  - The `.rs` file that contains the function to test.  
  - Any existing test file (e.g., `tests/` directory) that should be extended.

- **Exact launch prompt**  
  ```
  /chat You are aider-test-writer. Write a test for the function
  [function name] in [file path]. Follow the existing test style in
  the codebase. Use `#[test_case]` if that pattern is used. Do not
  modify any non‑test code.
  ```

- **Token-saving routine**  
  - Only add the file that contains the function and the test file.  
  - Use `grep -n '#\[test\]\|#\[test_case\]'` to find existing test patterns.

- **Max-efficiency workflow**  
  1. `/add` the source file and the test file.  
  2. Ask for the test.  
  3. Apply, run `cargo test`, report result.

- **Rules / forbidden behavior**  
  - Must not modify any non‑test code.  
  - Must not introduce dependencies on `std` or `alloc` unless already present.  
  - Must follow the existing test naming conventions.

---

## aider-rename-agent

- **Purpose**  
  Rename a symbol (function, variable, type, constant) across the entire
  workspace, updating all references without changing semantics.

- **When to use**  
  When a name is misleading, conflicts with a new name, or needs to be
  aligned with the architecture documentation.

- **Files to add to Aider**  
  All `.rs` files that contain the symbol to rename. Use `grep -rn` to
  find them first.

- **Exact launch prompt**  
  ```
  /chat You are aider-rename-agent. Rename [old_name] to [new_name] in
  the files I added. Use a single find‑and‑replace operation. Do not
  change any other code. After the rename, run `cargo check` and report
  any remaining references.
  ```

- **Token-saving routine**  
  - Use `grep -rn 'old_name'` to find all occurrences before adding files.  
  - Only add the files that contain the symbol.  
  - Do not add the entire workspace.

- **Max-efficiency workflow**  
  1. Run `grep -rn 'old_name' --include='*.rs'` to list files.  
  2. `/add` only those files.  
  3. Ask for the rename.  
  4. Apply, run `cargo check`, report result.  
  5. If any references remain, add the missing file and repeat.

- **Rules / forbidden behavior**  
  - Must not change any logic or formatting.  
  - Must not rename symbols that are part of a public API without asking.  
  - Must not rename symbols that appear in documentation files (those are
    handled by `aider-doc-sync`).
