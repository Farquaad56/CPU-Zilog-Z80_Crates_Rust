//! Test machines (CP/M, ZX Spectrum) and simulation session for Z80 emulator.
//!
//! This crate provides reusable machine configurations and a simulation
//! session that can be shared between the CLI and web server frontends.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod cpm;
pub mod session;
pub mod zx;

/// Crate version (from Cargo.toml).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn e01_01a_version() {
        assert!(!VERSION.is_empty(), "VERSION must be non-empty");
    }
}
