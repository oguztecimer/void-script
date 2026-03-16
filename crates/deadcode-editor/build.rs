use std::process::Command;

fn main() {
    let editor_ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../editor-ui");

    // Only rebuild if frontend sources changed
    println!("cargo::rerun-if-changed=../../editor-ui/src");
    println!("cargo::rerun-if-changed=../../editor-ui/index.html");
    println!("cargo::rerun-if-changed=../../editor-ui/package.json");

    // Install deps if node_modules missing
    if !editor_ui_dir.join("node_modules").exists() {
        let status = Command::new("npm")
            .arg("install")
            .current_dir(&editor_ui_dir)
            .status()
            .expect("failed to run npm install — is Node.js installed?");
        assert!(status.success(), "npm install failed");
    }

    // Build frontend
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(&editor_ui_dir)
        .status()
        .expect("failed to run npm run build — is Node.js installed?");
    assert!(status.success(), "npm run build failed");
}
