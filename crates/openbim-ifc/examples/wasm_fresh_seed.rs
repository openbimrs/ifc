//! `fresh_seed` exported to JavaScript, for `scripts/check-wasm-seed.sh`
//! (#573): the script binds it with `wasm-bindgen --target nodejs` and checks
//! under Node that two processes draw different seeds (built with
//! `js-random`) and that a build without it refuses.
//!
//! On other targets this example is an empty library.

/// The seed as 32 lowercase hex digits, or the error text.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = freshSeed)]
pub fn fresh_seed() -> Result<String, String> {
    ifc::fresh_seed()
        .map(|seed| format!("{seed:032x}"))
        .map_err(|error| error.to_string())
}
