//! Memory and internal cycle pin outputs (R3/R4/R5, W3/W5, I<n>).
//!
//! Pure functions that compute the output pins for each T-state of memory
//! read/write cycles and internal cycles. Does not modify CPU state.

use crate::pins::Pins;

/// Compute the output pins for a given T-state of an M1 cycle.
///
/// # Arguments
/// * `pc` - Program counter value (address being fetched)
/// * `i` - I register value (high byte of refresh address)
/// * `r` - R register value AFTER increment (low 7 bits used for refresh)
/// * `t` - T-state index within the cycle (0..len-1)
/// * `len` - Total length of the M1 cycle in T-states (4, 5, or 6)
/// * `halt` - If true, add HALT signal to every T-state
///
/// # Returns
/// Pin state for this T-state.
pub fn fetch_pins(pc: u16, i: u8, r: u8, t: u8, len: u8, halt: bool) -> Pins {
    let mut pins = Pins(0);

    match t {
        0 => {
            // T0: A=PC, M1 only
            pins = pins.set_addr(pc).with(Pins::M1);
        }
        1 => {
            // T1: A=PC, M1|MREQ|RD (opcode read)
            pins = pins.set_addr(pc).with(Pins::M1 | Pins::MREQ | Pins::RD);
        }
        2 | 3 => {
            // T2/T3: refresh cycle - A=(I<<8)|(R&0x7F), MREQ|RFSH
            let refresh_addr = ((i as u16) << 8) | ((r & 0x7F) as u16);
            pins = pins.set_addr(refresh_addr).with(Pins::MREQ | Pins::RFSH);
        }
        _ => {
            // Extended M1 (t>=4): A=(I<<8)|(R&0x7F), no control signals
            let refresh_addr = ((i as u16) << 8) | ((r & 0x7F) as u16);
            pins = pins.set_addr(refresh_addr);
        }
    }

    if halt {
        pins = pins.with(Pins::HALT);
    }

    pins
}

/// Compute the output pins for a given T-state of a memory read cycle (R3/R4/R5).
///
/// # Arguments
/// * `addr` - Memory address being read
/// * `t` - T-state index within the cycle (0..len-1)
///
/// # Returns
/// Pin state for this T-state.
pub fn read_pins(addr: u16, t: u8) -> Pins {
    match t {
        0 => {
            // T0: A only, no control signals
            Pins(0).set_addr(addr)
        }
        1 => {
            // T1: A + MREQ + RD (data read from memory)
            Pins(0).set_addr(addr).with(Pins::MREQ | Pins::RD)
        }
        _ => {
            // T2 and beyond: A only, no control signals
            Pins(0).set_addr(addr)
        }
    }
}

/// Compute the output pins for a given T-state of a memory write cycle (W3/W5).
///
/// # Arguments
/// * `addr` - Memory address being written
/// * `data` - Data byte to write
/// * `t` - T-state index within the cycle (0..len-1)
///
/// # Returns
/// Pin state for this T-state.
pub fn write_pins(addr: u16, data: u8, t: u8) -> Pins {
    match t {
        0 => {
            // T0: A + D, no WR yet
            Pins(0).set_addr(addr).set_data(data)
        }
        1 => {
            // T1: A + D + MREQ + WR (data written to memory)
            Pins(0).set_addr(addr).set_data(data).with(Pins::MREQ | Pins::WR)
        }
        _ => {
            // T2 and beyond: A + D, no control signals
            Pins(0).set_addr(addr).set_data(data)
        }
    }
}

/// Compute the output pins for an internal cycle (I<n>).
///
/// Internal cycles have no bus activity - only the address is held.
///
/// # Arguments
/// * `addr` - Last address on the bus (held during internal cycle)
///
/// # Returns
/// Pin state with only the address set, no control signals.
pub fn internal_pins(addr: u16) -> Pins {
    Pins(0).set_addr(addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e03_02_t0() {
        let p = fetch_pins(0x1000, 0x40, 0x50, 0, 4, false);
        assert_eq!(p.addr(), 0x1000);
        assert!(p.has(Pins::M1));
        assert!(!p.has(Pins::MREQ));
        assert!(!p.has(Pins::RD));
    }

    #[test]
    fn e03_02_t1() {
        let p = fetch_pins(0x1000, 0x40, 0x50, 1, 4, false);
        assert_eq!(p.addr(), 0x1000);
        assert!(p.has(Pins::M1));
        assert!(p.has(Pins::MREQ));
        assert!(p.has(Pins::RD));
    }

    #[test]
    fn e03_02_t2_refresh() {
        let p = fetch_pins(0x1000, 0x40, 0x50, 2, 4, false);
        assert_eq!(p.addr(), 0x4050);
        assert!(!p.has(Pins::M1));
        assert!(p.has(Pins::MREQ));
        assert!(p.has(Pins::RFSH));
        assert!(!p.has(Pins::RD));
    }

    #[test]
    fn e03_02_t3() {
        let p = fetch_pins(0x1000, 0x40, 0x50, 3, 4, false);
        assert_eq!(p.addr(), 0x4050);
        assert!(!p.has(Pins::M1));
        assert!(p.has(Pins::MREQ));
        assert!(p.has(Pins::RFSH));
    }

    #[test]
    fn e03_02_extended() {
        // F6: 6 T-states, check t4 and t5 have no control signals
        for t in [4u8, 5] {
            let p = fetch_pins(0x1000, 0x40, 0x50, t, 6, false);
            assert_eq!(p.addr(), 0x4050);
            assert!(!p.has(Pins::M1));
            assert!(!p.has(Pins::MREQ));
            assert!(!p.has(Pins::RFSH));
            assert!(!p.has(Pins::RD));
        }
    }

    #[test]
    fn e03_02_r_bit7_masked() {
        // R=0xFF should give low byte 0x7F in refresh address
        let p = fetch_pins(0x1000, 0x40, 0xFF, 2, 4, false);
        assert_eq!(p.addr(), 0x407F);
    }

    #[test]
    fn e03_02_halt_flag() {
        // HALT adds HALT signal to all T-states of the cycle
        for t in 0..4u8 {
            let p = fetch_pins(0x1000, 0x40, 0x50, t, 4, true);
            assert!(p.has(Pins::HALT));
        }
    }

    #[test]
    fn e03_03_read_t0() {
        // T0: address only, no control signals
        let p = read_pins(0x2000, 0);
        assert_eq!(p.addr(), 0x2000);
        assert!(!p.has(Pins::MREQ));
        assert!(!p.has(Pins::RD));
    }

    #[test]
    fn e03_03_read_t1() {
        // T1: address + MREQ + RD (data read)
        let p = read_pins(0x2000, 1);
        assert_eq!(p.addr(), 0x2000);
        assert!(p.has(Pins::MREQ));
        assert!(p.has(Pins::RD));
    }

    #[test]
    fn e03_03_read_t2_ext() {
        // T2..T4: address only, no control signals (extended read cycles)
        for t in [2u8, 3, 4] {
            let p = read_pins(0x2000, t);
            assert_eq!(p.addr(), 0x2000);
            assert!(!p.has(Pins::MREQ));
            assert!(!p.has(Pins::RD));
        }
    }

    #[test]
    fn e03_03_write_t0() {
        // T0: address + data, no WR yet
        let p = write_pins(0x3000, 0xAB, 0);
        assert_eq!(p.addr(), 0x3000);
        assert_eq!(p.data(), 0xAB);
        assert!(!p.has(Pins::WR));
    }

    #[test]
    fn e03_03_write_t1() {
        // T1: address + data + MREQ + WR (data written)
        let p = write_pins(0x3000, 0xAB, 1);
        assert_eq!(p.addr(), 0x3000);
        assert_eq!(p.data(), 0xAB);
        assert!(p.has(Pins::MREQ));
        assert!(p.has(Pins::WR));
    }

    #[test]
    fn e03_03_write_ext() {
        // T2..T4: address + data maintained, no control signals (extended write)
        for t in [2u8, 3, 4] {
            let p = write_pins(0x3000, 0xAB, t);
            assert_eq!(p.addr(), 0x3000);
            assert_eq!(p.data(), 0xAB);
            assert!(!p.has(Pins::MREQ));
            assert!(!p.has(Pins::WR));
        }
    }

    #[test]
    fn e03_03_internal() {
        // Internal cycle: address only, no control signals at all
        let p = internal_pins(0x4000);
        assert_eq!(p.addr(), 0x4000);
        assert!(!p.has(Pins::MREQ));
        assert!(!p.has(Pins::RD));
        assert!(!p.has(Pins::WR));
    }
}
