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

    pub fn store32(&mut self, addr: u32, value: u32) -> Result<u32, String> {
        self.interconnect.store32(addr, value)
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
                Opcode::StoreWord => self.op_sw(instr),
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
    // rt = immediate << 16
    fn op_lui(&mut self, instr: Instruction) {
        let rt = instr.gpr_rt();
        let imm = instr.immediate();
        self.set_register(rt, imm << 16);
    }

    // Or Immediate
    // rt = rs | immediate
    fn op_ori(&mut self, instr: Instruction) {
        let rt = instr.gpr_rt();
        let imm = instr.immediate();
        let res = self.get_register(rt) | imm;
        self.set_register(rt, res);
    }

    // Store Word
    // memory[base+offset] = rt
    fn op_sw(&mut self, instr: Instruction) {
        let rt = instr.gpr_rt();
        let base = instr.base();
        let offset = instr.offset();

        let addr = self.get_register(base).wrapping_add(offset);
        let val = self.get_register(rt);
        self.store32(addr, val).unwrap();
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
        // Word addresses must be aligned by 4
        if addr % 4 != 0 {
            return Err(format!("Addr {addr} is not aligned").to_string());
        }
        if addr >= BIOS_METADATA.starting_addr || addr < BIOS_METADATA.last_addr {
            // The addr relative to BIOS' starting address
            let offset = addr - BIOS_METADATA.starting_addr;
            return Ok(self.bios.load32(offset));
        }

        Err(format!("Addr {addr} not in range for any peripheral").to_string())
    }

    pub fn store32(&mut self, addr: u32, value: u32) -> Result<u32, String> {
        // Word addresses must be aligned by 4
        if addr % 4 != 0 {
            return Err(format!("Addr {addr} is not aligned").to_string());
        }
        todo!("Interconnect::store32!!! addr: {addr:#x}, value: {value:#x}");
    }
}

struct Instruction(u32);

impl Instruction {
    fn opcode(&self) -> Option<Opcode> {
        // 31..26 (6b)
        let op = self.0 >> 26;
        Opcode::from_u32(op)
    }

    fn gpr_rs(&self) -> u32 {
        // 25..21 (5b)
        0b0001_1111 & (self.0 >> 21)
    }

    // Alias; same as above
    fn base(&self) -> u32 {
        // 25..21 (5b)
        0b0001_1111 & (self.0 >> 21)
    }

    fn gpr_rt(&self) -> u32 {
        // 20..16 (5b)
        0b0001_1111 & (self.0 >> 16)
    }

    fn immediate(&self) -> u32 {
        // 15..0 (16b)
        0xFFFF & self.0
    }

    // Alias; same as above
    fn offset(&self) -> u32 {
        // 15..0 (16b)
        0xFFFF & self.0
    }
}

#[derive(FromPrimitive, ToPrimitive)]
#[repr(u32)]
enum Opcode {
    LoadUpperImmediate = 0b0000_1111,
    OrImmediate = 0b0000_1101,
    StoreWord = 0b0010_1011,
}
