use std::{path::Path, process::Command};

use anyhow::{Context, Result, ensure};

/// Boots an x86-64 live ISO in the foreground, sharing the terminal with QEMU.
/// This learning backend does not yet provide persistent VM management or QMP.
pub fn run(iso: &Path) -> Result<()> {
    ensure!(
        iso.is_file(),
        "ISO is not a regular file: {}",
        iso.display()
    );
    let iso = iso.canonicalize().context("resolve ISO path")?;

    let status = Command::new("qemu-system-x86_64")
        .args(["-accel", "kvm", "-m", "256", "-smp", "1", "-cdrom"])
        .arg(&iso)
        .args(["-boot", "order=d", "-nic", "none", "-nographic"])
        .status()
        .context("start QEMU; install qemu-system-x86_64 and check PATH")?;

    ensure!(status.success(), "QEMU exited unsuccessfully: {status}");
    Ok(())
}
