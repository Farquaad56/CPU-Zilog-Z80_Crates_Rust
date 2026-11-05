//! SingleStepTests/z80 loader (format: docs/specs/33_SINGLESTEP_FORMAT.md).
//!
//! Reads the JSON test files and builds a [`Cpu`] plus a sparse memory from the `initial` state.
//! The comparison with the `final` state and the cycles lives elsewhere (E13-08 and later).

use puce_z80::{Bus, Cpu};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Value read from an address that the test does not list in `ram`.
pub const MISSING_BYTE: u8 = 0xFF;

/// One bus cycle sample: address, data (`None` when the bus is disconnected), pins `rwmi`.
pub type CycleRow = (u16, Option<u8>, String);

/// One I/O transaction: port address, value, `"r"` or `"w"`.
pub type PortRow = (u16, u8, String);

/// CPU and memory state (`initial` or `final`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct State {
    /// Register A.
    pub a: u8,
    /// Register B.
    pub b: u8,
    /// Register C.
    pub c: u8,
    /// Register D.
    pub d: u8,
    /// Register E.
    pub e: u8,
    /// Register F.
    pub f: u8,
    /// Register H.
    pub h: u8,
    /// Register L.
    pub l: u8,
    /// Interrupt vector register.
    pub i: u8,
    /// Refresh register.
    pub r: u8,
    /// Alternate AF'.
    pub af_: u16,
    /// Alternate BC'.
    pub bc_: u16,
    /// Alternate DE'.
    pub de_: u16,
    /// Alternate HL'.
    pub hl_: u16,
    /// Index register IX.
    pub ix: u16,
    /// Index register IY.
    pub iy: u16,
    /// Program counter.
    pub pc: u16,
    /// Stack pointer.
    pub sp: u16,
    /// Internal MEMPTR register.
    pub wz: u16,
    /// Interrupt flip-flop 1 (0 or 1).
    pub iff1: u8,
    /// Interrupt flip-flop 2 (0 or 1).
    pub iff2: u8,
    /// Interrupt mode (0, 1 or 2).
    pub im: u8,
    /// 1 when the previous instruction was EI.
    pub ei: u8,
    /// 1 when the previous instruction was LD A,I or LD A,R.
    pub p: u8,
    /// Last F written by the previous instruction (0 if none).
    pub q: u8,
    /// Memory cells `[address, value]` that the test touches.
    pub ram: Vec<(u16, u8)>,
}

impl State {
    /// Copy every register of the state into `cpu`.
    pub fn apply(&self, cpu: &mut Cpu) {
        let r = &mut cpu.regs;
        r.a = self.a;
        r.b = self.b;
        r.c = self.c;
        r.d = self.d;
        r.e = self.e;
        r.f = self.f;
        r.h = self.h;
        r.l = self.l;
        r.i = self.i;
        r.r = self.r;
        r.set_af2(self.af_);
        r.set_bc2(self.bc_);
        r.set_de2(self.de_);
        r.set_hl2(self.hl_);
        r.ix = self.ix;
        r.iy = self.iy;
        r.pc = self.pc;
        r.sp = self.sp;
        r.wz = self.wz;
        r.iff1 = self.iff1 != 0;
        r.iff2 = self.iff2 != 0;
        r.im = self.im;
        r.q = self.q;
        r.halted = false;
        cpu.set_ei_pending(self.ei != 0);
        cpu.set_ld_air_pending(self.p != 0);
    }
}

/// One test case.
#[derive(Clone, Debug, Deserialize)]
pub struct Case {
    /// Human-readable name, for example `"3A 0000"`.
    pub name: String,
    /// State before the instruction.
    pub initial: State,
    /// State after the instruction.
    #[serde(rename = "final")]
    pub final_: State,
    /// One bus sample per T-state.
    pub cycles: Vec<CycleRow>,
    /// I/O transaction (IN / OUT instructions only).
    #[serde(default)]
    pub ports: Vec<PortRow>,
}

/// Sparse 64 KiB memory: absent addresses read as [`MISSING_BYTE`].
#[derive(Clone, Debug, Default)]
pub struct SparseBus {
    /// Cells present in the map.
    pub mem: HashMap<u16, u8>,
}

impl SparseBus {
    /// Build the memory from `[address, value]` pairs.
    pub fn from_pairs(pairs: &[(u16, u8)]) -> Self {
        SparseBus {
            mem: pairs.iter().copied().collect(),
        }
    }
}

impl Bus for SparseBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem.get(&addr).copied().unwrap_or(MISSING_BYTE)
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.mem.insert(addr, data);
    }
}

/// Parse the content of a test file.
pub fn parse_cases(json: &str) -> Result<Vec<Case>, String> {
    serde_json::from_str(json).map_err(|e| format!("invalid test file: {}", e))
}

/// Load a test file (an array of cases).
pub fn load_file(path: &Path) -> Result<Vec<Case>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    parse_cases(&text).map_err(|e| format!("{}: {}", path.display(), e))
}

/// Build the CPU and the memory from the `initial` state of a case.
pub fn apply_initial(case: &Case) -> (Cpu, SparseBus) {
    let mut cpu = Cpu::new();
    case.initial.apply(&mut cpu);
    (cpu, SparseBus::from_pairs(&case.initial.ram))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
      {"name": "00 0000",
       "initial": {"a":110,"b":1,"c":2,"d":3,"e":4,"f":250,"h":5,"l":6,"i":166,"r":16,
         "af_":27987,"bc_":37392,"de_":52402,"hl_":41834,"ix":29661,"iy":5464,"pc":19935,
         "sp":59438,"wz":62861,"iff1":1,"iff2":0,"im":2,"ei":1,"p":1,"q":7,"ram":[[19935,0]]},
       "final": {"a":110,"b":1,"c":2,"d":3,"e":4,"f":250,"h":5,"l":6,"i":166,"r":17,
         "af_":27987,"bc_":37392,"de_":52402,"hl_":41834,"ix":29661,"iy":5464,"pc":19936,
         "sp":59438,"wz":62861,"iff1":1,"iff2":0,"im":2,"ei":0,"p":0,"q":0,"ram":[[19935,0]]},
       "cycles": [[19935,null,"----"],[19935,null,"r-m-"],[42512,0,"----"],[42512,null,"----"]]},
      {"name": "D3 0000",
       "initial": {"a":102,"b":0,"c":0,"d":0,"e":0,"f":0,"h":0,"l":0,"i":0,"r":0,
         "af_":0,"bc_":0,"de_":0,"hl_":0,"ix":0,"iy":0,"pc":38371,
         "sp":0,"wz":0,"iff1":0,"iff2":0,"im":0,"ei":0,"p":0,"q":0,"ram":[[38371,211],[38372,159]]},
       "final": {"a":102,"b":0,"c":0,"d":0,"e":0,"f":0,"h":0,"l":0,"i":0,"r":2,
         "af_":0,"bc_":0,"de_":0,"hl_":0,"ix":0,"iy":0,"pc":38373,
         "sp":0,"wz":26271,"iff1":0,"iff2":0,"im":0,"ei":0,"p":0,"q":0,"ram":[[38371,211],[38372,159]]},
       "cycles": [[38371,null,"----"],[26271,102,"-w-i"]],
       "ports": [[26271,102,"w"]]}
    ]"#;

    #[test]
    fn e13_07_parse_sample() {
        let cases = parse_cases(SAMPLE).unwrap();
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].name, "00 0000");
        assert_eq!(cases[0].cycles.len(), 4);
        assert_eq!(cases[0].cycles[1], (19935, None, "r-m-".to_string()));
        assert_eq!(cases[0].final_.pc, 19936);
        assert!(cases[0].ports.is_empty());
        assert_eq!(cases[1].ports, vec![(26271, 102, "w".to_string())]);
    }

    #[test]
    fn e13_07_apply_regs() {
        let cases = parse_cases(SAMPLE).unwrap();
        let (cpu, _) = apply_initial(&cases[0]);
        let r = &cpu.regs;
        assert_eq!(
            (r.a, r.b, r.c, r.d, r.e, r.f, r.h, r.l),
            (110, 1, 2, 3, 4, 250, 5, 6)
        );
        assert_eq!((r.i, r.r, r.pc, r.sp, r.wz), (166, 16, 19935, 59438, 62861));
        assert_eq!((r.ix, r.iy), (29661, 5464));
        assert_eq!(
            (r.af2(), r.bc2(), r.de2(), r.hl2()),
            (27987, 37392, 52402, 41834)
        );
        assert!(r.iff1 && !r.iff2);
        assert_eq!((r.im, r.q), (2, 7));
        assert!(cpu.ei_pending() && cpu.ld_air_pending());
    }

    #[test]
    fn e13_07_apply_ram() {
        let cases = parse_cases(SAMPLE).unwrap();
        let (_, mut bus) = apply_initial(&cases[1]);
        assert_eq!(bus.mem.len(), 2);
        assert_eq!(bus.read(38371), 211);
        assert_eq!(bus.read(38372), 159);
        assert_eq!(bus.read(0x1234), MISSING_BYTE);
        bus.write(0x1234, 7);
        assert_eq!(bus.read(0x1234), 7);
    }

    #[test]
    fn e13_07_missing_field_error() {
        let bad = SAMPLE.replace("\"wz\":62861,", "");
        let err = parse_cases(&bad).unwrap_err();
        assert!(err.contains("missing field `wz`"), "{}", err);
    }

    #[test]
    fn e13_07_load_real_file() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/singlestep/v1/00.json");
        if !path.exists() {
            eprintln!("SKIP: run python docs/tools/fetch_roms.py --optional singlestep");
            return;
        }
        let cases = load_file(&path).unwrap();
        assert_eq!(cases.len(), 1000);
        assert_eq!(cases[0].cycles.len(), 4);
    }
}
