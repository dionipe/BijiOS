fn main() {
    println!("cargo:rustc-link-lib=vte-2.91-gtk4");
    println!("cargo:rustc-link-lib=gtk-4");
    println!("cargo:rustc-link-lib=cairo");
    println!("cargo:rustc-link-lib=pango-1.0");
    println!("cargo:rustc-link-lib=gio-2.0");
    println!("cargo:rustc-link-lib=gobject-2.0");
    println!("cargo:rustc-link-lib=glib-2.0");
    println!("cargo:rustc-link-lib=X11");
}
