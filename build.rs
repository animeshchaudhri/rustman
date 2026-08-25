//! Build script.
//!
//! Embeds the Windows application icon into the executable. `iced`'s
//! `window::Settings::icon` only sets the *runtime* window icon; the icon shown
//! by Explorer, the taskbar's pinned entry, Alt-Tab and the shortcut/installer
//! comes from a Win32 `RT_GROUP_ICON` resource compiled into the binary. With
//! no resource, `rustman.exe` shipped with the generic default icon.
//!
//! This is a no-op on every other platform.

fn main() {
    println!("cargo:rerun-if-changed=public/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        let icon = std::path::Path::new("public/icon.ico");
        if !icon.exists() {
            println!(
                "cargo:warning=public/icon.ico missing; the executable will have no icon"
            );
            return;
        }
        let mut res = winresource::WindowsResource::new();
        res.set_icon("public/icon.ico");
        res.set("ProductName", "Rustman");
        res.set("FileDescription", "Rustman — native API testing tool");
        res.set("CompanyName", "animeshchaudhri");
        res.set("LegalCopyright", "MIT licensed");
        if let Err(err) = res.compile() {
            // A missing resource compiler must not fail the build; the binary is
            // still perfectly usable, just without an embedded icon.
            println!("cargo:warning=failed to embed Windows icon: {err}");
        }
    }
}
