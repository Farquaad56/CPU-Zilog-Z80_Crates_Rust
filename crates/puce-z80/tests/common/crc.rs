//! CRC-32 (IEEE 802.3) computed at runtime; no external dependencies.

/// Incremental CRC-32 accumulator.
pub struct Crc32(u32);

impl Crc32 {
    /// Start a new CRC-32 computation.
    pub fn new() -> Self {
        Crc32(0xFFFFFFFF)
    }

    /// Feed a single byte into the computation.
    pub fn update(&mut self, b: u8) {
        let mut table = [0u32; 256];
        let poly = 0xEDB88320u32;
        for n in 0..256 {
            let mut c = n as u32;
            for _ in 0..8 {
                if c & 1 != 0 {
                    c = (c >> 1) ^ poly;
                } else {
                    c >>= 1;
                }
            }
            table[n as usize] = c;
        }
        let idx = ((self.0 ^ (b as u32)) & 0xFF) as usize;
        self.0 = table[idx] ^ (self.0 >> 8);
    }

    /// Finalize and return the CRC-32 value.
    pub fn finish(self) -> u32 {
        self.0 ^ 0xFFFFFFFF
    }
}

/// Compute CRC-32 over a byte slice in one call.
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = Crc32::new();
    for &b in data {
        c.update(b);
    }
    c.finish()
}
