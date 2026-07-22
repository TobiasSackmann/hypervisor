use anyhow::{Context, Result};
use kvm_bindings::{kvm_regs, kvm_sregs};
use kvm_ioctls::VcpuFd;

pub fn setup_real_mode_registers(vcpu: &mut VcpuFd, guest_start: u64) -> Result<()> {
    let mut sregs: kvm_sregs = vcpu.get_sregs().context("get special registers")?;

    sregs.cs.base = 0;
    sregs.cs.selector = 0;

    vcpu.set_sregs(&sregs).context("set special registers")?;

    let regs = kvm_regs {
        rip: guest_start,
        rflags: 0x2,
        rsp: guest_start + 0x800,
        rbp: guest_start + 0x800,
        ..Default::default()
    };

    vcpu.set_regs(&regs).context("set general registers")?;

    Ok(())
}
