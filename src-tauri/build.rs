use std::env;
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    // tauri-build links its Windows resource, which carries the application manifest,
    // into binaries only. Test executables also load Common Controls v6 through Tauri
    // and fail with STATUS_ENTRYPOINT_NOT_FOUND without it, so the same manifest is
    // embedded through linker arguments for every target instead.
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))?;

    if env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|target_env| target_env == "msvc") {
        let manifest = Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed=windows-app-manifest.xml");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
    Ok(())
}
