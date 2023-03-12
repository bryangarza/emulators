#[macro_use]
extern crate num_derive;

use num_traits::{FromPrimitive, ToPrimitive};

const PROGRAM_COUNTER_RESET_VALUE: u32 = 0xbfc00000;
const BIOS_METADATA: Range = Range {
    starting_addr: 0xbfc00000,
    last_addr: 0xbfc00000 + (512 * 1024),
    size: 512 * 1024,
};

pub struct Range {
    starting_addr: u32,
    last_addr: u32,
    size: u32,
}

fn main() {
    let mut cpu = Cpu::new();
    loop {
        cpu.run_single_cycle();
    }
}

struct Cpu {
    pc: u32,
    registers: [u32; 32],
    interconnect: Interconnect,
}

impl Cpu {
    pub fn new() -> Self {
        let mut registers = [0xdeadbeef; 32];
        registers[0] = 0;
        Cpu {
            pc: PROGRAM_COUNTER_RESET_VALUE,
            registers,
            interconnect: Interconnect::new(),
        }
    }

    pub fn load32(&self, addr: u32) -> Result<u32, String> {
        self.interconnect.load32(addr)
    }

    pub fn run_single_cycle(&mut self) {
        let instr = self
            .load32(self.pc)
            .expect("Unable to load next instruction");
        self.pc = self.pc.wrapping_add(4);
        self.execute_instr(instr);
    }

    pub fn execute_instr(&mut self, instr_: u32) {
        let instr = Instruction(instr_);
        if let Some(opcode) = instr.opcode() {
            match opcode {
                Opcode::LoadUpperImmediate => self.op_lui(instr),
                Opcode::OrImmediate => self.op_ori(instr),
            }
        } else {
            panic!(
                "If I could, I'd handle this instruction: {instr_:#x} {instr_:#b}
            registers = {:#x?}",
                self.registers
            );
        }
    }

    pub fn get_register(&self, register_index: u32) -> u32 {
        self.registers[register_index as usize]
    }

    pub fn set_register(&mut self, register_index: u32, value: u32) {
        self.registers[register_index as usize] = value;
        // Never overwrite $zero
        self.registers[0] = 0;
    }

    // Load Upper Immediate
    // Set imm as upper 16 bits of target register
    fn op_lui(&mut self, instr: Instruction) {
        let t = instr.target_register();
        let i = instr.immediate_value();

        self.set_register(t, i << 16);
    }

    // Or Immediate
    // Set target register to logical OR of target register and imm
    fn op_ori(&mut self, instr: Instruction) {
        let t = instr.target_register();
        let i = instr.immediate_value();

        let res = self.get_register(t) | i;

        self.set_register(t, res);
    }
}

struct Bios {
    data: Vec<u8>,
}

impl Bios {
    pub fn new() -> Self {
        // TODO: Move path to config
        let data = std::fs::read("./data/SCPH1001.BIN").expect("unable to load BIOS file!");
        Bios { data }
    }

    // little endian (LSB goes first, i.e., the left side)
    pub fn load32(&self, offset: u32) -> u32 {
        let offset = offset as usize;

        let msb = self.data[offset] as u32;
        let next_sb = self.data[offset + 1] as u32;
        let next_next_sb = self.data[offset + 2] as u32;
        let lsb = self.data[offset + 3] as u32;

        lsb << 24 | next_next_sb << 16 | next_sb << 8 | msb
    }
}

struct Interconnect {
    bios: Bios,
}

impl Interconnect {
    pub fn new() -> Self {
        Interconnect { bios: Bios::new() }
    }

    pub fn load32(&self, addr: u32) -> Result<u32, String> {
        if addr >= BIOS_METADATA.starting_addr || addr < BIOS_METADATA.last_addr {
            // The addr relative to BIOS' starting address
            let offset = addr - BIOS_METADATA.starting_addr;
            return Ok(self.bios.load32(offset));
        }

        Err(format!("Addr {addr} not in range for any peripheral").to_string())
    }
}

struct Instruction(u32);

impl Instruction {
    fn opcode(&self) -> Option<Opcode> {
        // 31..26 (6b)
        let op = self.0 >> 26;
        Opcode::from_u32(op)
    }

    fn target_register(&self) -> u32 {
        // 20..16 (5b)
        0b0001_1111 & (self.0 >> 16)
    }

    fn immediate_value(&self) -> u32 {
        // 15..0 (16b)
        0xFFFF & self.0
    }
}

#[derive(FromPrimitive, ToPrimitive)]
#[repr(u32)]
enum Opcode {
    LoadUpperImmediate = 0b0000_1111,
    OrImmediate = 0b0000_1101,
}
