mod qmp;

use std::{
    io::ErrorKind,
    io::{self, Write},
    os::unix::net::UnixStream,
    path::Path,
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};

/// Boots an x86-64 live ISO in the foreground, sharing the terminal with QEMU.
/// Performs an initial QMP status query; persistent management is not yet implemented.
pub fn run(iso: &Path) -> Result<()> {
    ensure!(
        iso.is_file(),
        "ISO is not a regular file: {}",
        iso.display()
    );
    let iso = iso.canonicalize().context("resolve ISO path")?;

    // A private, unique directory avoids collisions between VM instances.
    let runtime = tempfile::Builder::new()
        .prefix("hypervisor-")
        .tempdir_in("/tmp")
        .context("create VM runtime directory")?;
    let socket = runtime.path().join("qmp.sock");
    let endpoint = format!("unix:{},server=on,wait=off", socket.display());

    let console_socket = runtime.path().join("console.sock");
    let console_endpoint = format!("unix:{},server=on,wait=off", console_socket.display());
    eprintln!("Guest console socket: {}", console_socket.display());

    let mut child = Command::new("qemu-system-x86_64")
        .args(["-accel", "kvm", "-m", "256", "-smp", "1", "-cdrom"])
        .arg(&iso)
        .args(["-boot", "order=d", "-nic", "none"])
        .args(["-display", "none", "-monitor", "none"])
        .args(["-serial", &console_endpoint])
        .args(["-qmp", &endpoint])
        .spawn()
        .context("start QEMU")?;

    let result = (|| -> Result<()> {
        let stream = connect_qmp(&socket, &mut child)?;
        let mut qmp = qmp::Qmp::new(stream)?;

        query_version(&mut qmp)?;
        control_vm(&mut qmp, &mut child)?;

        Ok(())
    })();

    if let Err(error) = result {
        // Once spawned, every failing setup path must terminate and reap QEMU.
        // Retain the runtime directory if cleanup fails, for diagnosis/recovery.
        if let Err(cleanup) = terminate(&mut child) {
            let path = runtime.keep();
            return Err(error.context(format!(
                "QEMU cleanup failed: {cleanup:#}; runtime retained at {}",
                path.display()
            )));
        }
        return Err(error);
    }

    Ok(())
}

fn control_vm(qmp: &mut qmp::Qmp, child: &mut Child) -> Result<()> {
    loop {
        if let Some(exit_status) = child.try_wait().context("check QEMU process")? {
            ensure!(
                exit_status.success(),
                "QEMU exited unsuccessfully: {exit_status}"
            );
            return Ok(());
        }
        print!("hypervisor> ");
        io::stdout().flush()?;
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)?;
        ensure!(bytes_read > 0, "control input closed");

        let command_result: Result<()> = match input.trim() {
            "status" => query_status(qmp).map(|status| {
                eprintln!("QMP VM status: {status}");
            }),
            "help" => {
                eprintln!("Available commands: status, help, pause, resume");
                Ok(())
            }
            "pause" => pause_vm(qmp),
            "resume" => resume_vm(qmp),
            "" => Ok(()), // Ignore empty input,
            _ => {
                eprintln!("Unknown command. Available commands: status, help, pause, resume");
                Ok(())
            }
        };
        if let Err(error) = command_result {
            if error.is::<qmp::CommandRejected>() {
                eprintln!("Command rejected: {error}");
            } else {
                return Err(error);
            }
        }
    }
}

fn query_status(qmp: &mut qmp::Qmp) -> Result<String, anyhow::Error> {
    let status = qmp.execute("query-status")?;
    let status = status["status"]
        .as_str()
        .context("QMP status missing status string")?;
    Ok(status.to_owned())
}

fn pause_vm(qmp: &mut qmp::Qmp) -> Result<(), anyhow::Error> {
    qmp.execute("stop")?;
    Ok(())
}

fn resume_vm(qmp: &mut qmp::Qmp) -> Result<()> {
    qmp.execute("cont")?;
    Ok(())
}

fn query_version(qmp: &mut qmp::Qmp) -> Result<(), anyhow::Error> {
    let version = qmp.execute("query-version")?;
    let major = version["qemu"]["major"]
        .as_u64()
        .context("QMP version missing major number")?;
    let minor = version["qemu"]["minor"]
        .as_u64()
        .context("QMP version missing minor number")?;
    let micro = version["qemu"]["micro"]
        .as_u64()
        .context("QMP version missing micro number")?;
    eprintln!("QEMU version: {major}.{minor}.{micro}");
    Ok(())
}

fn connect_qmp(socket: &Path, child: &mut Child) -> Result<UnixStream> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().context("check QEMU startup")? {
            bail!("QEMU exited before QMP was ready: {status}");
        }
        match UnixStream::connect(socket) {
            Ok(stream) => return Ok(stream),
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::NotFound | ErrorKind::ConnectionRefused
                ) =>
            {
                ensure!(Instant::now() < deadline, "timed out connecting to QMP");
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error).context("connect to QMP"),
        }
    }
}

fn terminate(child: &mut Child) -> Result<()> {
    if child
        .try_wait()
        .context("check QEMU before cleanup")?
        .is_none()
    {
        child.kill().context("terminate QEMU after failure")?;
    }
    child.wait().context("reap QEMU after failure")?;
    Ok(())
}
