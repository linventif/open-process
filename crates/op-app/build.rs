//! Embed the Explorer icon without adding a Rust build dependency.

use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/open-process.ico");
    println!("cargo:rerun-if-changed=assets/windows.rc");
    println!("cargo:rerun-if-env-changed=RC");
    println!("cargo:rerun-if-env-changed=WINDRES");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let resource = out.join(if msvc { "app-icon.res" } else { "app-icon.o" });
    let mut command = if msvc {
        let mut command = Command::new(msvc_resource_compiler());
        command.arg("/nologo").arg("/fo").arg(&resource).arg("assets/windows.rc");
        command
    } else {
        let compiler = env::var_os("WINDRES").unwrap_or_else(|| "windres".into());
        let mut command = Command::new(compiler);
        command.args(["-i", "assets/windows.rc", "-O", "coff", "-o"]).arg(&resource);
        command
    };
    command.current_dir(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let status = command
        .status()
        .expect("Windows icon compiler unavailable: install the Windows SDK (rc.exe), or set RC / WINDRES");
    assert!(status.success(), "Windows icon resource compilation failed: {status}");
    println!("cargo:rustc-link-arg-bin=open-process={}", resource.display());
}

fn msvc_resource_compiler() -> PathBuf {
    if let Some(compiler) = env::var_os("RC") {
        return compiler.into();
    }
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            let compiler = directory.join("rc.exe");
            if compiler.is_file() {
                return compiler;
            }
        }
    }
    // Cargo can find MSVC without the developer shell; find the SDK likewise.
    let arch = match env::var("HOST").unwrap_or_default().split('-').next().unwrap_or("") {
        "aarch64" => "arm64",
        "i686" => "x86",
        _ => "x64",
    };
    let programs = env::var_os("ProgramFiles(x86)").unwrap_or_else(|| r"C:\Program Files (x86)".into());
    let sdk_bin = PathBuf::from(programs).join("Windows Kits/10/bin");
    let mut versions: Vec<_> = fs::read_dir(sdk_bin)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    versions.sort();
    for version in versions.into_iter().rev() {
        let compiler = version.join(arch).join("rc.exe");
        if compiler.is_file() {
            return compiler;
        }
    }
    "rc.exe".into()
}
