use std::{
    fs,
    path::{Path, PathBuf},
};

fn collect(dir: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, paths);
        } else if path.extension().is_some_and(|x| x == "rs") {
            paths.push(path);
        }
    }
}
fn main() {
    println!("cargo:rerun-if-changed=../../NERVA/src");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=../Modell/src");
    println!("cargo:rerun-if-changed=../Modell/Cargo.toml");
    println!("cargo:rerun-if-changed=../nerva-qemu/src");
    println!("cargo:rerun-if-changed=../nerva-qemu/Cargo.toml");
    let mut paths = vec![
        PathBuf::from("Cargo.toml"),
        PathBuf::from("Cargo.lock"),
        PathBuf::from("../../NERVA/Cargo.toml"),
        PathBuf::from("build.rs"),
        PathBuf::from("../Modell/Cargo.toml"),
        PathBuf::from("../nerva-qemu/Cargo.toml"),
    ];
    collect(Path::new("../../NERVA/src"), &mut paths);
    collect(Path::new("src"), &mut paths);
    collect(Path::new("../Modell/src"), &mut paths);
    collect(Path::new("../nerva-qemu/src"), &mut paths);
    paths.sort();
    let mut hash = 0xcbf29ce484222325_u64;
    for path in paths {
        for byte in path
            .to_string_lossy()
            .bytes()
            .chain([0])
            .chain(fs::read(&path).unwrap())
        {
            hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
        }
    }
    println!("cargo:rustc-env=NERVA_RESEARCH_BUILD={hash:016x}");
}
