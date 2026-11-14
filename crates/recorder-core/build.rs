use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, exit};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
        .join("../../native/macos");
    let source_dir = root.join("Sources/RecorderBridge");
    let mut sources: Vec<PathBuf> = match fs::read_dir(&source_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "swift")
            })
            .collect(),
        Err(error) => {
            eprintln!("cannot read the macOS Swift bridge sources: {error}");
            exit(1);
        }
    };
    sources.sort();
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap_or_default());
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        _ => {
            eprintln!("unsupported macOS target architecture");
            exit(1);
        }
    };
    println!("cargo:rerun-if-changed={}", source_dir.display());
    for source in &sources {
        println!("cargo:rerun-if-changed={}", source.display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        root.join("include/recorder-bridge.h").display()
    );

    let status = Command::new("swiftc")
        .args([
            "-emit-library",
            "-static",
            "-module-name",
            "RecorderBridge",
            "-target",
        ])
        .arg(format!("{arch}-apple-macosx14.2"))
        .arg("-module-cache-path")
        .arg(output.join("swift-module-cache"))
        .args(&sources)
        .arg("-o")
        .arg(output.join("libRecorderBridge.a"))
        .status();
    if !status.as_ref().is_ok_and(|status| status.success()) {
        eprintln!("failed to compile the macOS Swift bridge: {status:?}");
        exit(1);
    }
    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-lib=static=RecorderBridge");
}
