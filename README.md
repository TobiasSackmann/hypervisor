# Hypervisor

Hands-on learning project for a small Linux/KVM based x86 hypervisor in Rust.

The project intentionally uses Linux as the host operating system and KVM as the
hardware virtualization interface. It is not a bare-metal VMX hypervisor and does
not try to replace the host OS.

## Current MVP

The current code starts one tiny x86 guest through `/dev/kvm`:

1. Open KVM and verify the KVM API version.
2. Create a VM.
3. Allocate one guest RAM region in userspace.
4. Copy a small guest program into that RAM.
5. Register the memory region with KVM.
6. Create one vCPU.
7. Initialize the vCPU in a real-mode-like state.
8. Run the vCPU until it exits.
9. Print serial-style output when the guest writes to I/O port `0x3f8`.

The guest currently writes a short string to the emulated serial port by
repeating this instruction pattern:

```asm
mov dx, 0x3f8
mov al, <byte>
out dx, al
hlt
```

## Run

```sh
cargo run
```

Expected output:

```text
guest wrote to I/O port 0x3f8: [72]
...
serial output: Hello from guest
guest executed HLT; VM stopped successfully
```

This requires Linux with KVM available, usually `/dev/kvm` permissions for the
current user, and hardware virtualization enabled in firmware.

## Roadmap

Near-term work should keep the feedback loop small:

1. Extend the serial output path so the guest can print strings, not only one byte.
2. Add a slightly larger guest program and keep all VM exits visible.
3. Move guest loading behind a small abstraction so raw bytes and later ELF/kernel
   images can use the same VM setup path.
4. Add basic CLI options for guest memory size and entry address.
5. Add tests for pure logic and a manual smoke test for `/dev/kvm` integration.
6. Later, boot a tiny Linux kernel or bzImage through KVM.

## CI

GitHub Actions runs the deterministic quality checks on every push to `main` or
`master` and on pull requests:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --all-targets --all-features
cargo build --release --all-targets --all-features
cargo rustc --release -- --emit=llvm-ir
```

Profiling tools such as flamegraph, heaptrack, Valgrind Massif, DHAT, allocator
tracking, and jemalloc stats are useful later, but should not be mandatory CI
gates yet. They need stable benchmarks and often extra host support. Until there
is a repeatable workload, CI should prove correctness and build quality while
profiling stays a manual or scheduled investigation.

## KVM vs. bare-metal VMX

In this project, Linux owns the hardware. KVM exposes virtualization through
`/dev/kvm`, and this Rust program asks Linux to create and run VMs. VM exits are
reported back to the process through KVM ioctls.

In a bare-metal VMX hypervisor, the hypervisor itself boots on the machine,
configures CPU virtualization instructions directly, owns physical memory and
interrupt setup, and usually must provide much more OS-like infrastructure.

Using KVM is the practical path here: it keeps Linux responsible for device
drivers, scheduling, memory management, interrupts, and hardware setup while this
project focuses on VM construction, vCPU state, guest memory, and VM exits.
