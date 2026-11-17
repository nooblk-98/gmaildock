fn main() {
    // Declaring the app commands turns on access control for them: a window can only call
    // the commands its capability grants (see capabilities/). Gmail only reports its unread count.
    let manifest = tauri_build::AppManifest::new().commands(&["set_unread_count"]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to run tauri-build");
}
