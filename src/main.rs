mod guest;
mod kvm;
mod qemu;

use anyhow::{Result, bail};
use std::path::Path;

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let command = args.next();

    match command.as_deref().and_then(|arg| arg.to_str()) {
        Some("qemu") => {
            let iso = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: hypervisor qemu <ISO>"))?;
            if args.next().is_some() {
                bail!("usage: hypervisor qemu <ISO>");
            }
            qemu::run(Path::new(&iso))
        }
        Some("kvm-demo") => {
            let message = args
                .next()
                .map(|arg| arg.into_string())
                .transpose()
                .map_err(|_| anyhow::anyhow!("message must be valid UTF-8"))?
                .unwrap_or_else(|| "Hallo aus meiner VM!".to_string());
            if args.next().is_some() {
                bail!("usage: hypervisor kvm-demo [MESSAGE]");
            }
            kvm::vm::run_hello_vm(message)
        }
        None | Some("--help" | "-h") => {
            println!("Usage:\n  hypervisor qemu <ISO>\n  hypervisor kvm-demo [MESSAGE]");
            Ok(())
        }
        _ => bail!("unknown command; use --help"),
    }
}
