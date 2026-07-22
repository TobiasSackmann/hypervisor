use anyhow::{Context, Result};
use kvm_ioctls::VcpuExit;

use crate::guest::code::{
    DEFAULT_MESSAGE, GUEST_MEM_SIZE, GUEST_START, SERIAL_PORT_COM1, build_serial_message_guest,
};
use crate::kvm::{device, memory::GuestRam, serial::SerialConsole, vcpu};

/// Runs the current learning MVP: one KVM VM, one vCPU, one tiny serial guest.
pub fn run_hello_vm() -> Result<()> {
    let kvm = device::open()?;

    let vm: kvm_ioctls::VmFd = kvm.create_vm().context("create KVM VM")?;

    let code = build_serial_message_guest(DEFAULT_MESSAGE);
    let guest_ram = GuestRam::new(GUEST_START, GUEST_MEM_SIZE)?;
    guest_ram.write_at_start(&code)?;
    guest_ram.register_with_kvm(&vm, 0)?;

    let mut vcpu = vm.create_vcpu(0).context("create vCPU")?;

    vcpu::setup_real_mode_registers(&mut vcpu, GUEST_START)?;

    let mut serial = SerialConsole::new(SERIAL_PORT_COM1);

    loop {
        match vcpu.run().context("KVM_RUN failed")? {
            VcpuExit::IoOut(port, data) => {
                println!("guest wrote to I/O port 0x{:x}: {:?}", port, data);
                serial.record_io_out(port, data);
            }

            VcpuExit::Hlt => {
                if !serial.is_empty() {
                    print!("serial output: {}", serial.output_lossy());
                }

                println!("guest executed HLT; VM stopped successfully");
                break;
            }

            exit => {
                println!("unexpected VM exit: {:?}", exit);
                break;
            }
        }
    }

    Ok(())
}
