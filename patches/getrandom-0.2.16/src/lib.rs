//! Stub getrandom for wasm32-unknown-unknown without wasm-bindgen.
//! Uses a simple PRNG seeded from a counter (NOT cryptographically secure,
//! but sufficient for non-security-critical indexer use).

#![no_std]

use core::num::NonZeroU32;

/// Error type matching getrandom's public API.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Error(NonZeroU32);

impl Error {
    #[allow(dead_code)]
    pub const UNSUPPORTED: Error = Error(unsafe { NonZeroU32::new_unchecked(1) });

    pub fn code(self) -> NonZeroU32 {
        self.0
    }

    pub fn raw_os_error(self) -> Option<i32> {
        None
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "getrandom stub: code {}", self.0)
    }
}

impl core::error::Error for Error {}

impl From<Error> for core::num::NonZeroU32 {
    fn from(e: Error) -> Self {
        e.0
    }
}

/// Fill `dest` with pseudo-random bytes.
/// This is NOT cryptographically secure — it's a deterministic stub
/// for WASM indexer contexts where true randomness isn't needed.
pub fn getrandom(dest: &mut [u8]) -> Result<(), Error> {
    // Simple xorshift64 PRNG seeded from a counter.
    static mut SEED: u64 = 0x12345678_9abcdef0;
    unsafe {
        for byte in dest.iter_mut() {
            SEED ^= SEED << 13;
            SEED ^= SEED >> 7;
            SEED ^= SEED << 17;
            *byte = (SEED & 0xFF) as u8;
        }
    }
    Ok(())
}

/// Register a custom random provider (no-op in this stub).
pub fn register_custom_getrandom(_: fn(&mut [u8]) -> Result<(), Error>) {}
