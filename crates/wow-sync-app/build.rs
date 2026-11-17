// Embeds the brand icon into wow-sync.exe only. GPUI's Windows backend loads icon resource
// id 1 for the window class, so the same resource also gives the window/taskbar icon.
fn main() {
    println!("cargo:rerun-if-changed=resources/wow-sync.rc");
    println!("cargo:rerun-if-changed=../../assets/brand/wow_addon_sync.ico");
    #[cfg(windows)]
    embed_resource::compile_for("resources/wow-sync.rc", ["wow-sync"], embed_resource::NONE)
        .manifest_optional()
        .expect("embed icon resource");
}
