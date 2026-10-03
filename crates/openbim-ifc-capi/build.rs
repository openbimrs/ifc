//! Stamp a relocatable name onto the shared library (#41).
//!
//! Rust's `cdylib` carries no SONAME on ELF, and on Mach-O its install name
//! is the absolute path cargo linked it at. A consumer's loader entry then
//! records that path: on Linux a `DT_NEEDED` containing `/` is never searched
//! through RPATH/RUNPATH, and on macOS the dylib only loads from the cargo
//! target directory. Either way the library cannot be installed or shipped in
//! an archive.
//!
//! The bare file name (ELF) and `@rpath/<file>` (Mach-O) make the dependency
//! resolve through the consumer's normal search path, as Axiolid's C ABI does
//! (axiolid/kernel#70). `@rpath` in turn needs an `LC_RPATH` in the consumer;
//! the CMake package supplies it (axiolid/kernel#113, see `CMakeLists.txt`).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Only the cdylib is affected; the rlib and staticlib carry no name.
    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("linux" | "android" | "freebsd" | "netbsd" | "openbsd" | "dragonfly") => {
            println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libopenbim_ifc_capi.so");
        }
        Ok("macos" | "ios") => {
            println!(
                "cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libopenbim_ifc_capi.dylib"
            );
        }
        // Windows: the import library already names the DLL.
        _ => {}
    }
}
