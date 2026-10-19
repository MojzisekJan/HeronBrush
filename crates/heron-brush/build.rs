//! Embed the application icon in Windows executables.
use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=assets/app-icons/heron-brush.ico");
    println!("cargo:rerun-if-env-changed=RC");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") { return; }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let icon = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("assets/app-icons/heron-brush.ico");
    let source = out.join("app-icon.rc");
    std::fs::write(&source, format!("1 ICON \"{}\"\n", icon.display().to_string().replace('\\', "/"))).unwrap();
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let output = out.join(if msvc { "app-icon.res" } else { "app-icon.o" });
    let compiler = env::var("RC").unwrap_or_else(|_| if msvc { "rc.exe".into() } else { "windres".into() });
    let mut cmd = Command::new(&compiler);
    if msvc { cmd.arg("/nologo").arg("/fo").arg(&output).arg(&source); }
    else { cmd.arg("-i").arg(&source).arg("-o").arg(&output).args(["-O", "coff"]); }
    let status = cmd.status().unwrap_or_else(|e| panic!("Cannot run {compiler}: {e}. Install the Windows resource compiler or set RC to its path."));
    assert!(status.success(), "Windows icon resource compilation failed");
    println!("cargo:rustc-link-arg-bin=heron-brush={}", output.display());
}
