# Hypervisor

A lightweight, resource-efficient virtualization product written in Rust, built on
Linux, QEMU, and KVM. The goal is a complete hosted hypervisor solution that can
create, run, and manage multiple independent virtual machines with their own
kernels and operating systems.

## Product direction

QEMU is the primary VM execution backend. It supplies the virtual machine model,
device emulation, and guest boot support; KVM supplies hardware-accelerated CPU
virtualization on supported Linux hosts. A thin Rust layer manages configuration,
QEMU processes, VM lifecycle, and resource allocation. Building an entire VMM and
its device models independently would exceed the intended development effort.

The complete product combines these layers. The Rust component stays small while
QEMU and KVM provide the underlying virtualization capabilities. Efficiency is a
design goal to verify through measurements, not an existing performance claim.

The intended use cases are:

- Run independent operating systems, including Linux and eventually FreeBSD and
  Windows where the host architecture and QEMU guest support permit.
- Develop, test, and debug custom kernels and operating systems in VMs.
- Run untrusted workloads behind a guest-kernel isolation boundary. This requires
  deliberate host permissions, device exposure, and QEMU process isolation before
  treating the product as suitable for hostile workloads.

## Management architecture and scale

The number of VMs is configurable and limited by available host resources, not a
fixed count in the implementation. Two concurrent VMs are the first acceptance
scenario, not a product limit. The initial scope is one host; multi-host scheduling
is a separate future decision.

Design the management layer independently of its user interfaces from the start:

```text
Terminal CLI / future graphical UI / future Terraform provider
                         |
             Product management API
                         |
        Rust VM management service and domain logic
                         |
            QEMU backend: process control + QMP
                         |
          One QEMU process per VM, backed by KVM
```

Start with a terminal CLI. A graphical interface and a Terraform provider are
long-term product goals and must use the same management operations and state
model. Keep validation, resource accounting, and lifecycle decisions out of the
CLI. Introduce a local management service/API for persistent ownership of running
VMs; the exact transport and remote access design remain to be selected. Closing
a client must not terminate its managed VMs.

Use stable VM identifiers, persisted VM definitions, and separate desired
configuration and observed runtime state. Plan create/read/update/delete
operations with documented update/restart semantics, repeatable requests, and
structured errors. This lets a future Terraform provider reconcile declared
configuration with actual state. Model long-running operations explicitly and
serialize conflicting operations per VM while allowing independent VMs to make
progress concurrently. Recover and reconcile managed QEMU processes after a
management service restart.

Keep a dynamic VM registry and per-VM resources (disk, sockets, console, network
endpoints). Account for aggregate CPU, RAM, and storage allocations with an
explicit overcommit policy and host headroom. Increasing VM count should not
require changing the architecture; practical limits need workload measurements.

QMP means QEMU Machine Protocol: a JSON-based control connection to an already
running QEMU process. The backend uses it to query VM status, request shutdown,
and receive lifecycle events. Rust starts QEMU through operating-system process
APIs, then controls it through QMP. QMP is an internal backend interface; the
product management API also handles VM definitions, persistence, resource limits,
and operations across multiple VMs. See the
[QMP specification](https://www.qemu.org/docs/master/interop/qmp-spec.html).

## First product milestone: small VMs on Raspberry Pi

The first use case is running multiple minimal Alpine Linux VMs concurrently on a
Raspberry Pi. Use a supported 64-bit Arm board with a 64-bit Linux host, working
KVM support, and enough RAM for the host and all guests. The exact board, RAM size,
and host distribution remain to be selected and validated.

Use native AArch64 guests with `qemu-system-aarch64`, the generic `virt` machine,
and KVM acceleration. The guest sees QEMU's virtual platform rather than an
emulated Raspberry Pi board. Cross-architecture software emulation is outside the
first efficiency milestone; x86 guests on an Arm host do not gain native KVM
acceleration.

Start with headless guests, serial consoles, minimal virtual devices, and explicit
per-VM RAM and vCPU settings. Each VM gets its own writable disk and control socket.
Use Alpine's AArch64 Virtual installation media to prepare a reproducible guest
image. Firmware and the guest boot path must be selected and documented during
this first boot experiment.

The milestone is complete when the Rust CLI can start, inspect, and gracefully
stop at least two independent Alpine VMs on the selected Pi, provide console and
network access, and clean up child processes and runtime resources. Record the
host baseline, per-VM and total memory use, idle CPU consumption, boot time, and
behavior under a small repeatable workload. Set numerical resource targets after
measuring the selected hardware.

Architecture references:

- [QEMU system emulation and accelerators](https://www.qemu.org/docs/master/system/introduction.html)
- [QEMU Arm virt platform](https://www.qemu.org/docs/master/system/arm/virt.html)
- [Alpine Linux installation downloads](https://www.alpinelinux.org/downloads/)

## Current implementation: direct KVM learning prototype

The QEMU backend and multi-VM product described above are planned, not implemented.
The existing code is an x86-only direct-KVM learning prototype. It remains useful
for understanding guest memory, register setup, and VM exits, but does not yet
implement the Raspberry Pi milestone. The commands and code map below describe
this prototype.

## Current MVP

The current code starts one tiny x86 guest through `/dev/kvm`:

1. Open KVM and verify the KVM API version.
2. Create a VM.
3. Allocate 4 KiB of guest RAM in userspace, mapped at guest physical address `0x1000`.
4. Copy a small guest program into that RAM.
5. Register the memory region with KVM.
6. Create one vCPU.
7. Initialize the vCPU in a real-mode-like state.
8. Run the vCPU, log each `IoOut` exit, collect bytes for port `0x3f8`, and resume.
9. On `HLT`, print the collected message and stop the VM loop.

The guest is generated as raw x86 machine code from `Hello from guest\n`.
It selects COM1 once, emits one `MOV`/`OUT` pair per message byte, and halts once
at the end:

```asm
mov dx, 0x3f8
mov al, <byte>
out dx, al
; repeat mov al / out for the remaining bytes
hlt
```

The serial console collects a complete message, ignores other ports, and replaces
invalid UTF-8 when displaying output. It is a byte collector, not a full UART
device model.

## Code map

Start with [`run_hello_vm()`](src/kvm/vm.rs), which connects these components:

| File | Responsibility |
| --- | --- |
| [`src/main.rs`](src/main.rs) | Application entry point. |
| [`src/kvm/device.rs`](src/kvm/device.rs) | Open KVM and check API version 12. |
| [`src/kvm/memory.rs`](src/kvm/memory.rs) | Allocate guest RAM, copy code, and register the host memory with KVM. |
| [`src/kvm/vcpu.rs`](src/kvm/vcpu.rs) | Set the initial registers and entry address. |
| [`src/kvm/vm.rs`](src/kvm/vm.rs) | Create the VM and vCPU, then handle VM exits. |
| [`src/kvm/serial.rs`](src/kvm/serial.rs) | Collect and decode serial output. |
| [`src/guest/code.rs`](src/guest/code.rs) | Define guest constants and generate machine code. |

Guest addresses refer to the guest's physical address space. KVM maps these to
memory owned by the host process. `GuestRam` must stay alive while KVM uses that
mapping; the current run function keeps it alive throughout vCPU execution.

## Run

Use an x86-64 Linux host with KVM available, hardware virtualization enabled, and
read/write access to `/dev/kvm`. Building requires a Rust toolchain supporting
edition 2024 and Cargo. CI uses the stable Rust toolchain.

```sh
cargo run
```

Expected output (intermediate byte logs omitted):

```text
guest wrote to I/O port 0x3f8: [72]
...
serial output: Hello from guest
guest executed HLT; VM stopped successfully
```

The message, memory size, and entry address are constants in
[`src/guest/code.rs`](src/guest/code.rs); there are no CLI options yet.
For a first experiment, change `DEFAULT_MESSAGE` and run again.

## Tests and manual smoke test

Run the six unit tests without access to `/dev/kvm`:

```sh
cargo test --all-targets --all-features
```

They cover the generated COM1 setup, per-byte instructions, halting with an empty
message, serial port filtering, byte collection, and invalid UTF-8 handling.
They do not execute a VM or verify the KVM memory and register setup.

On a KVM host, use `cargo run` as a manual smoke test. Verify that the output
contains `serial output: Hello from guest` followed by
`guest executed HLT; VM stopped successfully`. A successful process exit alone
is insufficient: unexpected VM exits currently also end the loop with `Ok(())`.

## Current limitations

- One VM, one vCPU, one RAM region, and a generated real-mode-style guest.
- No external guest image loader, ELF/bzImage support, or Linux boot path yet.
- Only `IoOut` and `Hlt` have dedicated handling. Other exits are logged and stop
  execution without returning an error; writes to other ports are logged but
  are not collected by the serial console.
- No automated KVM integration test; CI checks builds and pure logic only.

## Roadmap

Prioritize the QEMU-backed product path:

1. Select the Raspberry Pi model, RAM budget, and 64-bit host OS. Verify access to
   `/dev/kvm` and an actual AArch64 QEMU boot with KVM; fail clearly if KVM is
   unavailable instead of silently falling back to software emulation.
2. Boot one Alpine AArch64 guest manually with a minimal QEMU command. Document
   QEMU/firmware versions, image provenance and checksum, installation steps,
   serial console, networking, and clean shutdown. Measure baseline resource use.
3. Define the interface-independent management/domain layer and API contract,
   including stable VM IDs, persisted definitions, and runtime state. Add a Rust
   QEMU backend alongside the current learning prototype. Define a
   small VM configuration (name, architecture, RAM, vCPUs, disk, and networking),
   validate it, and launch QEMU with structured process arguments.
4. Add QMP (QEMU Machine Protocol) control through a per-VM Unix socket for status,
   shutdown, and lifecycle events. Handle startup failures, process exit, timeouts,
   and explicit forced shutdown; reap children and clean up runtime files.
5. Add a dynamic VM registry and resource accounting, using two concurrent VMs as
   the first acceptance scenario. Give VMs independent writable disks, unique identifiers,
   sockets, and network endpoints. Start with user-mode networking and explicit
   SSH port forwarding; reserve host capacity when assigning guest resources.
6. Measure the two-VM milestone on the Pi. Tune virtual devices and guest services
   against recorded memory, CPU, boot-time, and workload results. Add tests for
   configuration and process/control failures, plus an opt-in QEMU/KVM smoke test.
7. Extend guest profiles and image/boot support toward other Linux distributions,
   custom kernels, FreeBSD, and Windows. Validate support per host architecture;
   broader OS support follows the first Alpine milestone.

The direct-KVM prototype can continue as a learning aid, but reimplementing QEMU's
VMM/device functionality is outside the primary product roadmap.

## CI

GitHub Actions runs the deterministic quality checks on every push to `main` or
`master` and on pull requests:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
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

In the current prototype, KVM keeps Linux responsible for device
drivers, scheduling, memory management, interrupts, and hardware setup while this
project focuses on VM construction, vCPU state, guest memory, and VM exits.
