# rCore OS Tutorial - Copilot Instructions

This is a **RISC-V 64-bit OS tutorial** implementing core OS concepts across 9 chapters. Copilot assists with kernel development, syscalls, memory management, and task scheduling.

## Quick Start

### Build & Run
```bash
cd 2026s-rcore-aptacc2421/os
make run              # Build + execute in QEMU
make build            # Compile only (no execution)
make debug            # Launch GDB debugging
```

### Testing
```bash
# Test specific chapter implementations
cd 2026s-rcore-aptacc2421/os
make test CHAPTER=5
```

### Project Location
- **Kernel**: `os/src/` (Cargo project)
- **User Apps**: `user/src/bin/` (test programs named `ch[N]_*.rs`)
- **Build Artifacts**: `os/target/riscv64gc-unknown-none-elf/release/`

## Architecture

| Component | Files | Purpose |
|-----------|-------|---------|
| **Memory** | `os/src/mm/` | Virtual memory (SV39 paging), frame allocation, page tables |
| **Task** | `os/src/task/` | Process scheduling, TCB management, context switching |
| **Trap** | `os/src/trap/` | Exception/interrupt dispatch and handling |
| **Syscall** | `os/src/syscall/` | System call implementations (process, filesystem, I/O) |
| **Config** | `os/src/config.rs` | Kernel constants (PAGE_SIZE, MEMORY_END, etc.) |

## Code Conventions

### Naming
- **Files**: snake_case (e.g., `frame_allocator.rs`)
- **Modules**: Folder name + `mod.rs` re-exports symbols
- **Constants**: UPPER_SNAKE_CASE for kernel parameters
- **Functions**: snake_case; syscalls use `sys_*()` prefix

### Syscall IDs
Use RISC-V ABI syscall numbers (in `os/src/syscall/mod.rs`):
- WRITE (64), EXIT (93), FORK (220), EXEC (221), etc.

### Memory Layout
- **Kernel base**: `0x80200000`
- **User apps**: Dynamically mapped via memory_set
- **Page size**: 4 KB (typical RISC-V)

### Task States
Tasks Progress: **Ready** → **Running** → **Blocked** → **Exit**

## Development Patterns

### Adding a Syscall
1. Define ID constant in `os/src/syscall/mod.rs`
2. Implement `sys_*()` function in appropriate module (`fs.rs`, `process.rs`)
3. Add dispatch case in `syscall()` function
4. Test with user app in `user/src/bin/ch[N]_yourtest.rs`

### Modifying Memory Management
- **Address translation**: `os/src/mm/page_table.rs`
- **Allocation policy**: `os/src/mm/frame_allocator.rs`
- **Memory layout**: Edit in `os/src/mm/memory_set.rs` and `config.rs`

### Debugging
- Use `make debug` for live GDB debugging
- Inspect memory: `x/10x $sp` (stack pointer content)
- Break on syscall: `set breakpoint syscall`

## Chapter Progression
- **ch1-2**: Bootloader, bare-metal execution
- **ch3**: Task scheduling with preemption
- **ch4**: Virtual memory and page tables
- **ch5**: Process creation (fork) and priority scheduling
- **ch6-9**: Advanced features (I/O, file systems, threads)

## File Locations Reference
```
os/
  src/
    batch.rs              # Initial task loading
    main.rs               # Entry point, module init
    entry.asm             # CPU entry, stack setup
    trap.S                # Trap handler assembly
    switch.S              # Context switch assembly
    syscall/
      mod.rs              # Syscall dispatcher
      process.rs          # Process syscalls (fork, exec, exit)
      fs.rs               # File I/O syscalls
    mm/
      memory_set.rs       # Virtual address space layout
      page_table.rs       # Page table operations
      address.rs          # Address conversion utilities
    task/
      manager.rs          # Global task pool
      processor.rs        # Current CPU state
      context.rs          # Task context (registers + stack)
  
user/src/bin/
  ch2b_hello_world.rs     # First test program
  ch3_sleep.rs            # Sleep syscall test
  ch5_spawn0.rs           # Process spawn test
  ch5_stride.rs           # Scheduler fairness test
```

## Common Tasks

### View task state
→ See `os/src/task/task.rs` (TaskStatus enum)

### Add a new test program
1. Create `user/src/bin/ch[N]_name.rs`
2. Run `make run` from `os/` (automatically linked)

### Check kernel constants
→ `os/src/config.rs` (PAGE_SIZE, MEMORY_END, USER_STACK_SIZE)

### Trace syscall flow
→ Entry: `trap.S` → `os/src/trap/mod.rs` → `os/src/syscall/mod.rs` → specific handler

## Links & Resources
- [rCore Tutorial Guide](https://LearningOS.github.io/rCore-Tutorial-Guide/)
- [Book (detailed)](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)
- [API Docs](https://learningos.github.io/rCore-Tutorial-Code/ch5/os/index.html) (chapter-specific)

---

**When working on this project:**
- Always verify you're working in `os/` or `user/` directories (not root)
- Use `make build` to check compilation without running QEMU
- Reference chapter-specific test apps in `user/src/bin/` for feature expectations
- Check git branch (`git branch -v`) to validate chapter context
