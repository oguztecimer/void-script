use std::process::Command;

fn main() {
    // Rebuild editor-ui when files change
    println!("cargo:rerun-if-changed=../../editor-ui/src");
    println!("cargo:rerun-if-changed=../../editor-ui/index.html");
    println!("cargo:rerun-if-changed=../../editor-ui/package.json");

    // Only build in release mode; in debug mode we use the dev server
    #[cfg(not(debug_assertions))]
    {
        let editor_ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../editor-ui");

        let status = Command::new("npm")
            .arg("run")
            .arg("build")
            .current_dir(&editor_ui_dir)
            .status()
            .expect("Failed to run npm build. Is Node.js installed?");

        if !status.success() {
            panic!("editor-ui build failed");
        }
    }

    // Suppress unused import warning in debug builds
    #[cfg(debug_assertions)]
    let _ = Command::new("true");
}
