#[cfg(target_os = "windows")]
fn embed_icon() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    let _ = res.compile();
}

#[cfg(not(target_os = "windows"))]
fn embed_icon() {}

fn main() {
    embed_icon();

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=framework=CoreGraphics");

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=framework=ApplicationServices");
}
