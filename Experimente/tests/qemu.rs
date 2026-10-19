use std::process::Command;

#[test]
#[ignore = "requires qemu-system-x86_64 and GNU binutils; run explicitly"]
fn real_guest_learns_and_verifies_via_x86_ports() {
    let output = Command::new(env!("CARGO_BIN_EXE_nerva-serial"))
        .arg("demo")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("NERVA_VM_PASS"));
}
