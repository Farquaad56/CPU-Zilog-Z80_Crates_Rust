//! Interrupt mode 2 with a make-believe Z80 CTC, arcade board style.
//!
//! - the "CTC" raises the INT line (`Bus::irq`) at regular intervals;
//! - during the interrupt acknowledge cycle it puts its vector on the data bus (`Bus::int_ack`);
//!   the CPU builds the address `(I << 8) | vector` and jumps through the word stored there;
//! - like the real Z80 peripherals in a daisy chain, it snoops the bus through `Bus::on_tick`
//!   for the byte pair `ED 4D` (RETI): only then is its interrupt "finished" and the next one
//!   allowed (the IEO line of the chain is released).
//!
//! Run it with `cargo run -q -p puce-z80 --example im2_ctc`.

use puce_z80::{Bus, Cpu, Pins};

/// Vector put on the bus by the CTC.
const VECTOR: u8 = 0x10;
/// The vector table lives at I = 0x80.
const TABLE: usize = 0x8010;
/// Interrupt handler address (stored in the table).
const HANDLER: u16 = 0x0120;
/// Counter incremented by the handler.
const COUNTER: usize = 0x9000;
/// T-states between two interrupt requests.
const PERIOD: u64 = 150;

/// IM 2 ; LD A,80h ; LD I,A ; EI ; JR $
const MAIN: [u8; 9] = [0xED, 0x5E, 0x3E, 0x80, 0xED, 0x47, 0xFB, 0x18, 0xFE];
/// LD HL,9000h ; INC (HL) ; EI ; RETI
const ISR: [u8; 7] = [0x21, 0x00, 0x90, 0x34, 0xFB, 0xED, 0x4D];

struct Board {
    ram: Box<[u8; 65536]>,
    // --- the CTC ---
    ticks: u64,
    irq_line: bool,
    in_service: bool,
    acks: u32,
    reti_seen: u32,
    // --- bus snooping ---
    prev_m1: bool,
    prev_opcode: u8,
    handler_fetches: u32,
}

impl Board {
    fn new() -> Board {
        let mut ram = Box::new([0u8; 65536]);
        ram[..MAIN.len()].copy_from_slice(&MAIN);
        ram[usize::from(HANDLER)..usize::from(HANDLER) + ISR.len()].copy_from_slice(&ISR);
        ram[TABLE] = HANDLER as u8;
        ram[TABLE + 1] = (HANDLER >> 8) as u8;
        Board {
            ram,
            ticks: 0,
            irq_line: false,
            in_service: false,
            acks: 0,
            reti_seen: 0,
            prev_m1: false,
            prev_opcode: 0,
            handler_fetches: 0,
        }
    }
}

impl Bus for Board {
    fn read(&mut self, addr: u16) -> u8 {
        self.ram[usize::from(addr)]
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.ram[usize::from(addr)] = data;
    }

    fn irq(&mut self) -> bool {
        self.irq_line
    }

    fn int_ack(&mut self) -> u8 {
        self.irq_line = false; // acknowledged
        self.in_service = true; // IEO low until RETI
        self.acks += 1;
        VECTOR
    }

    fn on_tick(&mut self, pins: Pins) {
        self.ticks += 1;
        // Snoop the opcode fetches (the first T of an M1 cycle shows the address).
        let m1 = pins.has(Pins::M1) && !pins.has(Pins::IORQ);
        if m1 && !self.prev_m1 {
            let addr = pins.addr();
            let opcode = self.ram[usize::from(addr)];
            if addr == HANDLER {
                self.handler_fetches += 1;
            }
            if self.prev_opcode == 0xED && opcode == 0x4D && self.in_service {
                self.in_service = false; // RETI seen: the chain is released
                self.reti_seen += 1;
            }
            self.prev_opcode = opcode;
        }
        self.prev_m1 = m1;
        // The timer fires periodically when the chain allows it.
        if self.ticks.is_multiple_of(PERIOD) && !self.in_service && !self.irq_line && self.acks < 10
        {
            self.irq_line = true;
        }
    }
}

fn main() {
    let mut board = Board::new();
    let mut cpu = Cpu::new();
    let mut steps = 0;
    while board.reti_seen < 10 && steps < 100_000 {
        cpu.step(&mut board);
        steps += 1;
    }
    println!(
        "{} interrupts acknowledged, {} RETI seen by the CTC, counter = {}, handler entered {} times",
        board.acks, board.reti_seen, board.ram[COUNTER], board.handler_fetches
    );
    println!(
        "vector {:02X} -> table[{:04X}] -> handler {:04X}",
        VECTOR, TABLE, HANDLER
    );
}
