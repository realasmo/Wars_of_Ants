//! Wars of Ants — shared simulation core.
//!
//! Compiled natively for the server and to WASM for the browser client.
//! Phase 0: skeleton only — simulation starts in Phase 1.

use wasm_bindgen::prelude::*;

/// Core version string — sanity check for the WASM bindings.
#[wasm_bindgen]
pub fn core_version() -> String {
    format!("woa-core {}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn core_has_version() {
        assert!(super::core_version().starts_with("woa-core"));
    }
}
