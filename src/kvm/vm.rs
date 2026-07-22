use anyhow::{Context, Result};
use kvm_ioctls::VcpuExit;

use crate::guest::code::{CODE, GUEST_MEM_SIZE, GUEST_START};
use crate::kvm::{device, memory::GuestRam, vcpu};

pub fn run_hello_vm() -> Result<()> {
    let kvm = device::open()?;

    let vm: kvm_ioctls::VmFd = kvm.create_vm().context("create KVM VM")?;

    let guest_ram = GuestRam::new(GUEST_START, GUEST_MEM_SIZE)?;
    guest_ram.write_at_start(CODE)?;
    guest_ram.register_with_kvm(&vm, 0)?;

    let mut vcpu = vm.create_vcpu(0).context("create vCPU")?;

    vcpu::setup_real_mode_registers(&mut vcpu, GUEST_START)?;

    let mut serial_output = Vec::new();

    loop {
        match vcpu.run().context("KVM_RUN failed")? {
            VcpuExit::IoOut(port, data) => {
                println!("guest wrote to I/O port 0x{:x}: {:?}", port, data);

                if port == 0x3f8 {
                    serial_output.extend_from_slice(data);
                }
            }

            VcpuExit::Hlt => {
                if !serial_output.is_empty() {
                    let output = String::from_utf8_lossy(&serial_output);
                    print!("serial output: {output}");
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
