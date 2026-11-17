fn main() {
    // Declaring the app commands turns on access control for them: each window can only call
    // the commands its capability grants (see capabilities/).
    let manifest = tauri_build::AppManifest::new().commands(&[
        "set_unread_count",
        "get_settings",
        "save_settings",
        "check_for_updates",
        "get_update_state",
        "install_update",
        "get_app_version",
        "open_about_link",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to run tauri-build");
}
