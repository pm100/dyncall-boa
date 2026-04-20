use std::{env, path::PathBuf, process::Command};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source = manifest.join("tools").join("struct_fixture.rs");
    let out_dir = manifest.join("target").join("struct-fixture");
    let output = out_dir.join("struct_fixture.dll");

    std::fs::create_dir_all(&out_dir).unwrap();

    if !output.exists() {
        let status = Command::new("rustc")
            .args([
                "--crate-type", "cdylib",
                "--edition", "2021",
                source.to_str().unwrap(),
                "-o", output.to_str().unwrap(),
            ])
            .status()
            .expect("rustc not found on PATH");
        assert!(status.success(), "Failed to compile struct_fixture.dll");
    }

    println!("cargo:rerun-if-changed=tools/struct_fixture.rs");
}
