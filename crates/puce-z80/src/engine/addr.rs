//! Address resolution for bus cycles.
//!
//! Converts an `Addr` token into a 16-bit bus address, applying side effects
//! on registers (PC increment, SP push/pop, WZ update) as specified by the
//! Z80 execution model. Called once at tick t0 of each bus cycle token.

use crate::cpu::Cpu;
use crate::engine::decode::Prefix;
use crate::engine::plan::Addr;

/// Resolve an address token to a 16-bit bus address.
///
/// Applies register side effects (PC++, SP push/pop, WZ update) as required
/// by the Z80 execution model. All arithmetic is 16-bit wrapping.
///
/// # Arguments
/// * `cpu` - CPU state (mutated for PC/SP/WZ side effects)
/// * `a` - Address token from the current plan
/// * `port` - True if this is a port address cycle (IN/OUT instructions)
pub(crate) fn resolve(cpu: &mut Cpu, a: Addr, port: bool) -> u16 {
    match a {
        Addr::None => cpu.eng.xy_addr,

        Addr::PcInc => {
            let addr = cpu.regs.pc;
            cpu.regs.pc = cpu.regs.pc.wrapping_add(1);
            addr
        }

        Addr::Sp => cpu.regs.sp,

        Addr::SpInc => {
            let addr = cpu.regs.sp;
            cpu.regs.sp = cpu.regs.sp.wrapping_add(1);
            addr
        }

        Addr::SpPlus1 => cpu.regs.sp.wrapping_add(1),

        Addr::DecSp => {
            cpu.regs.sp = cpu.regs.sp.wrapping_sub(1);
            cpu.regs.sp
        }

        Addr::Hl => cpu.regs.hl(),
        Addr::Bc => cpu.regs.bc(),
        Addr::De => cpu.regs.de(),

        Addr::Nn => cpu.regs.wz,
        Addr::NnPlus1 => cpu.regs.wz.wrapping_add(1),

        Addr::Xy => {
            let base = match cpu.eng.prefix {
                Prefix::Dd => cpu.regs.ix,
                Prefix::Fd => cpu.regs.iy,
                Prefix::None => 0, // Should not happen in valid plans
            };
            let disp: i16 = cpu.eng.disp as i8 as i16;
            let addr = (base as i16 + disp) as u16;
            cpu.eng.xy_addr = addr;
            cpu.regs.wz = addr;
            addr
        }

        Addr::An => {
            if port {
                ((cpu.regs.a as u16) << 8) | (cpu.regs.wz & 0x00FF)
            } else {
                // AN without port context: use WZ low byte as fallback
                cpu.regs.wz
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cpu() -> Cpu {
        Cpu::new()
    }

    #[test]
    fn e03_05_pc_inc() {
        let mut cpu = make_cpu();
        cpu.regs.pc = 0x1000;
        let addr = resolve(&mut cpu, Addr::PcInc, false);
        assert_eq!(addr, 0x1000);
        assert_eq!(cpu.regs.pc, 0x1001);

        // Wrapping at 0xFFFF
        cpu.regs.pc = 0xFFFF;
        let addr2 = resolve(&mut cpu, Addr::PcInc, false);
        assert_eq!(addr2, 0xFFFF);
        assert_eq!(cpu.regs.pc, 0x0000);
    }

    #[test]
    fn e03_05_sp_inc() {
        let mut cpu = make_cpu();
        cpu.regs.sp = 0x2000;
        let addr = resolve(&mut cpu, Addr::SpInc, false);
        assert_eq!(addr, 0x2000);
        assert_eq!(cpu.regs.sp, 0x2001);
    }

    #[test]
    fn e03_05_dec_sp() {
        let mut cpu = make_cpu();
        cpu.regs.sp = 0x2000;
        let addr = resolve(&mut cpu, Addr::DecSp, false);
        assert_eq!(addr, 0x1FFF);
        assert_eq!(cpu.regs.sp, 0x1FFF);

        // Wrapping at 0
        cpu.regs.sp = 0x0000;
        let addr2 = resolve(&mut cpu, Addr::DecSp, false);
        assert_eq!(addr2, 0xFFFF);
        assert_eq!(cpu.regs.sp, 0xFFFF);
    }

    #[test]
    fn e03_05_sp_plus1() {
        let mut cpu = make_cpu();
        cpu.regs.sp = 0x2000;
        let addr = resolve(&mut cpu, Addr::SpPlus1, false);
        assert_eq!(addr, 0x2001);
        assert_eq!(cpu.regs.sp, 0x2000); // SP unchanged
    }

    #[test]
    fn e03_05_nn() {
        let mut cpu = make_cpu();
        cpu.regs.wz = 0xFFFF;

        let addr = resolve(&mut cpu, Addr::Nn, false);
        assert_eq!(addr, 0xFFFF);
        assert_eq!(cpu.regs.wz, 0xFFFF); // WZ unchanged

        let addr2 = resolve(&mut cpu, Addr::NnPlus1, false);
        assert_eq!(addr2, 0x0000); // Wrapping
        assert_eq!(cpu.regs.wz, 0xFFFF); // WZ still unchanged
    }

    #[test]
    fn e03_05_xy_ix() {
        let mut cpu = make_cpu();
        cpu.regs.ix = 0x1000;
        cpu.eng.prefix = Prefix::Dd;
        cpu.eng.disp = 0xFE; // -2 signed

        let addr = resolve(&mut cpu, Addr::Xy, false);
        assert_eq!(addr, 0x0FFE);
        assert_eq!(cpu.regs.wz, 0x0FFE);
        assert_eq!(cpu.eng.xy_addr, 0x0FFE);
    }

    #[test]
    fn e03_05_xy_iy_pos() {
        let mut cpu = make_cpu();
        cpu.regs.iy = 0xFFFF;
        cpu.eng.prefix = Prefix::Fd;
        cpu.eng.disp = 2; // +2 signed

        let addr = resolve(&mut cpu, Addr::Xy, false);
        assert_eq!(addr, 0x0001); // Wrapping
        assert_eq!(cpu.regs.wz, 0x0001);
    }

    #[test]
    fn e03_05_an_port() {
        let mut cpu = make_cpu();
        cpu.regs.a = 0x12;
        cpu.regs.wz = 0x0034; // port number n was read into Z

        let addr = resolve(&mut cpu, Addr::An, true);
        assert_eq!(addr, 0x1234);
    }

    #[test]
    fn e03_05_bc_port() {
        let mut cpu = make_cpu();
        cpu.regs.b = 0xAB;
        cpu.regs.c = 0xCD;

        let addr = resolve(&mut cpu, Addr::Bc, true);
        assert_eq!(addr, 0xABCD);
    }
}
