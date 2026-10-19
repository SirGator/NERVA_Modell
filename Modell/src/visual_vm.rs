//! Lifecycle of the private, disposable VGA guest used by `demo` and `smoke`.

use nerva_qemu::{Result, qmp::Qmp};
use std::{
    env, fs,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    process::{Child, Command},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub struct DemoVm {
    child: Option<Child>,
    directory: PathBuf,
}

impl DemoVm {
    pub fn launch(cancelled: &AtomicBool) -> Result<(Self, Qmp)> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let directory =
            env::temp_dir().join(format!("nerva-visual-{}-{unique}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&directory)?;
        let mut vm = Self {
            child: None,
            directory,
        };
        let source = vm.directory.join("visual.S");
        let object = vm.directory.join("visual.o");
        let image = vm.directory.join("visual.img");
        let socket = vm.directory.join("qmp.sock");
        fs::write(&source, include_str!("../guest/visual.S"))?;
        checked(
            Command::new("as")
                .args(["--32"])
                .arg(&source)
                .arg("-o")
                .arg(&object),
        )?;
        checked(
            Command::new("ld")
                .args([
                    "-m",
                    "elf_i386",
                    "--oformat",
                    "binary",
                    "-Ttext",
                    "0x7c00",
                    "-e",
                    "_start",
                ])
                .arg(&object)
                .arg("-o")
                .arg(&image),
        )?;
        let bytes = fs::read(&image)?;
        if bytes.len() != 512 || bytes[510..] != [0x55, 0xaa] {
            return Err("invalid visual boot sector".into());
        }
        let mut command = Command::new(
            env::var_os("QEMU_SYSTEM_X86_64").unwrap_or_else(|| "qemu-system-x86_64".into()),
        );
        // The full keyboard can request a guest reboot (Ctrl+Alt+Delete).
        // Keep QEMU alive across resets so the visual loop can observe them.
        command
            .args([
                "-machine",
                "pc",
                "-accel",
                "tcg",
                "-m",
                "32M",
                "-display",
                "none",
                "-vga",
                "std",
                "-monitor",
                "none",
                "-serial",
                "none",
                "-parallel",
                "none",
                "-nic",
                "none",
                "-boot",
                "a",
            ])
            .arg("-drive")
            .arg(format!(
                "file={},format=raw,if=floppy,readonly=on",
                image.display()
            ))
            .arg("-qmp")
            .arg(format!("unix:{},server=on,wait=off", socket.display()));
        if let Some(firmware) = env::var_os("QEMU_FIRMWARE_DIR") {
            command.arg("-L").arg(firmware);
        }
        if let Some(rom) = env::var_os("QEMU_VGA_ROM") {
            let rom = rom.to_str().ok_or("QEMU_VGA_ROM must be UTF-8")?;
            command.arg("-global").arg(format!("VGA.romfile={rom}"));
        }
        vm.child = Some(command.spawn()?);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err("interrupted during QEMU startup".into());
            }
            if let Some(status) = vm.child.as_mut().expect("owned process").try_wait()? {
                return Err(format!("QEMU exited during startup: {status}").into());
            }
            match Qmp::connect(&socket) {
                Ok(qmp) => return Ok((vm, qmp)),
                Err(error) if Instant::now() >= deadline => return Err(error),
                Err(_) => thread::sleep(Duration::from_millis(20)),
            }
        }
    }
}

impl Drop for DemoVm {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn checked(command: &mut Command) -> Result<()> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!("{command:?}: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(())
}
