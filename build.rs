// Purpose: Embed the Windows application icon resource into the `ffm` binary.

#[cfg(not(test))]
fn main() {
    println!("cargo:rerun-if-changed=assets/ffm.rc");
    println!("cargo:rerun-if-changed=assets/ffm.ico");
    embed_windows_icon();
}

#[cfg(all(not(target_os = "windows"), not(test)))]
fn embed_windows_icon() {}

#[cfg(all(target_os = "windows", not(test)))]
fn embed_windows_icon() {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default());
    let res = out_dir.join("ffm.res");
    let compiled = compile_windows_icon(
        &["llvm-rc", "rc"],
        std::path::Path::new("assets"),
        &res,
        std::path::Path::new("assets/ffm.rc"),
    );
    emit_icon_directive(&res, compiled);
}

#[cfg(any(target_os = "windows", test))]
fn compile_windows_icon(
    tools: &[&str],
    assets: &std::path::Path,
    res: &std::path::Path,
    rc: &std::path::Path,
) -> bool {
    tools.iter().any(|tool| {
        std::process::Command::new(tool)
            .arg("/I")
            .arg(assets)
            .arg("/FO")
            .arg(res)
            .arg(rc)
            .status()
            .is_ok_and(|status| status.success())
    })
}

#[cfg(any(target_os = "windows", test))]
fn emit_icon_directive(res: &std::path::Path, compiled: bool) {
    if compiled {
        println!("cargo:rustc-link-arg-bin=ffm={}", res.display());
    }
}

#[cfg(test)]
#[path = "tests/build_script/tests.rs"]
mod tests;
