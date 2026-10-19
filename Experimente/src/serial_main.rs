#[cfg(not(unix))]
compile_error!("The QEMU Unix socket adapter currently requires Linux or another Unix host.");

use nerva_experiments::{Result, protocol, serial_model::VmModel};
use std::{
    env, fs,
    os::unix::{fs::DirBuilderExt, net::UnixStream},
    path::{Path, PathBuf},
    process::{Child, Command},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const TIMEOUT: Duration = Duration::from_secs(15);

fn main() {
    if let Err(error) = run() {
        eprintln!("nerva-serial: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut arguments = env::args().skip(1);
    let mode = arguments.next().unwrap_or_else(|| "help".into());
    if matches!(mode.as_str(), "help" | "--help" | "-h") {
        println!(
            "Usage:\n  nerva-serial demo [--weights FILE]\n  nerva-serial attach SOCKET [--weights FILE]\n\nEnvironment: QEMU_SYSTEM_X86_64, QEMU_FIRMWARE_DIR (optional -L directory)"
        );
        return Ok(());
    }
    let socket = match mode.as_str() {
        "demo" => None,
        "attach" => Some(PathBuf::from(
            arguments.next().ok_or("attach requires a socket path")?,
        )),
        _ => return Err(format!("unknown mode: {mode}; use --help").into()),
    };
    let mut weights = None;
    while let Some(argument) = arguments.next() {
        if argument != "--weights" || weights.is_some() {
            return Err(format!("unexpected argument: {argument}").into());
        }
        weights = Some(PathBuf::from(
            arguments.next().ok_or("--weights requires a path")?,
        ));
    }
    let mut model = VmModel::new()?;
    if let Some(path) = &weights {
        match fs::metadata(path) {
            Ok(_) => model.load(path)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let count = if let Some(socket) = socket {
        let mut stream = connect(&socket, None)?;
        protocol::serve(&mut stream, &mut model)?
    } else {
        demo(&mut model)?
    };
    if let Some(path) = weights {
        model.save(&path)?;
        println!("Learned weights saved to {}", path.display());
    }
    println!("Completed {count} neural trials.");
    Ok(())
}

/// Own only the child and private directory created by this invocation.
struct VmProcess {
    child: Option<Child>,
    directory: PathBuf,
}

impl Drop for VmProcess {
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

fn demo(model: &mut VmModel) -> Result<usize> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let directory = env::temp_dir().join(format!("nerva-vm-{}-{unique}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&directory)?;
    let mut vm = VmProcess {
        child: None,
        directory,
    };
    let object = vm.directory.join("guest.o");
    let image = vm.directory.join("guest.img");
    let socket = vm.directory.join("com1.sock");
    let guest_log = vm.directory.join("guest.log");
    // Assemble embedded source so the binary can be moved independently of the repo.
    let source = vm.directory.join("boot.S");
    fs::write(&source, include_str!("../guest/boot.S"))?;
    checked(
        Command::new("as")
            .arg("--32")
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
        return Err("guest boot sector must be exactly 512 bytes with a BIOS signature".into());
    }
    let executable =
        env::var_os("QEMU_SYSTEM_X86_64").unwrap_or_else(|| "qemu-system-x86_64".into());
    let mut command = Command::new(executable);
    command
        .args([
            "-machine",
            "pc",
            "-accel",
            "tcg",
            "-m",
            "16M",
            "-display",
            "none",
            "-monitor",
            "none",
            "-serial",
            "none",
            "-parallel",
            "none",
            "-nic",
            "none",
            "-vga",
            "none",
            "-no-reboot",
            "-boot",
            "a",
        ])
        .arg("-drive")
        .arg(format!(
            "file={},format=raw,if=floppy,readonly=on",
            image.display()
        ))
        .arg("-chardev")
        .arg(format!(
            "socket,id=nerva,path={},server=on,wait=on",
            socket.display()
        ))
        .args([
            "-device",
            "isa-serial,chardev=nerva,index=0,iobase=0x3f8,irq=4",
            "-device",
            "isa-debug-exit,iobase=0xf4,iosize=0x04",
        ])
        .arg("-debugcon")
        .arg(format!("file:{}", guest_log.display()));
    if let Some(firmware) = env::var_os("QEMU_FIRMWARE_DIR") {
        command.arg("-L").arg(firmware);
    }
    vm.child = Some(command.spawn().map_err(|error| {
        format!("could not start QEMU: {error}; install qemu-system-x86 or set QEMU_SYSTEM_X86_64")
    })?);
    let mut stream = connect(&socket, vm.child.as_mut())?;
    println!("QEMU connected: guest IN/OUT 0x3F8 ↔ NERVA (12 neurons, 16 plastic synapses)");
    let count = protocol::serve(&mut stream, model).map_err(|error| {
        let log = fs::read_to_string(&guest_log).unwrap_or_default();
        format!("VM communication failed: {error}; guest log: {log}")
    })?;
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = vm.child.as_mut().unwrap().try_wait()? {
            let log = fs::read_to_string(&guest_log).unwrap_or_default();
            if status.code() != Some(33) || !log.contains("NERVA_VM_PASS") {
                return Err(format!("guest verification failed ({status}): {log}").into());
            }
            print!("{log}");
            println!("Guest verified all 4 learned decisions through COM1 I/O ports.");
            return Ok(count);
        }
        if Instant::now() >= deadline {
            return Err("QEMU did not exit after FINISH".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn connect(socket: &Path, mut child: Option<&mut Child>) -> Result<UnixStream> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(child) = child.as_deref_mut()
            && let Some(status) = child.try_wait()?
        {
            return Err(format!("QEMU exited before connection: {status}").into());
        }
        match UnixStream::connect(socket) {
            Ok(stream) => {
                stream.set_read_timeout(Some(TIMEOUT))?;
                stream.set_write_timeout(Some(TIMEOUT))?;
                return Ok(stream);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                ) =>
            {
                if Instant::now() >= deadline {
                    return Err(error.into());
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.into()),
        }
    }
}
