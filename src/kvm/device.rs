use anyhow::{Context, Result, anyhow};
use kvm_ioctls::Kvm;

/// Opens `/dev/kvm` and verifies that the kernel exposes the expected KVM API.
pub fn open() -> Result<Kvm> {
    let kvm = Kvm::new().context("open /dev/kvm")?;

    let api_version = kvm.get_api_version();
    if api_version != 12 {
        return Err(anyhow!("unexpected KVM API version: {api_version}"));
    }

    Ok(kvm)
}
