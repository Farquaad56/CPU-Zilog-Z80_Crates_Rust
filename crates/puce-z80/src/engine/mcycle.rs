//! M1 cycle pin outputs (F4/F5/F6).
//!
//! Pure function that computes the output pins for each T-state of an
//! instruction fetch cycle. Does not modify CPU state.

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
}
