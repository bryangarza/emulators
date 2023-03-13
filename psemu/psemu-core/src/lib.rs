#[macro_use]
extern crate num_derive;

use num_traits::{FromPrimitive, ToPrimitive};
use tracing::{instrument, warn};

const PROGRAM_COUNTER_RESET_VALUE: u32 = 0xbfc00000;
const BIOS_ADDR_RANGE: AddressRange = AddressRange {
    starting_addr: 0xbfc00000,
    last_addr: 0xbfc00000 + (512 * 1024),
    // size: 512 * 1024,
};

const MEM_CONTROL_ADDR_RANGE: AddressRange = AddressRange {
    starting_addr: 0x1f801000,
    last_addr: 0x1f801004 + 32,
};

pub const REGISTER_NAMES: [&'static str; 32] = [
    "$zero", "$at", "$v0", "$v1", "$a0", "$a1", "$a2", "$a3", "$t0", "$t1", "$t2", "$t3", "$t4",
    "$t5", "$t6", "$t7", "$s0", "$s1", "$s2", "$s3", "$s4", "$s5", "$s6", "$s7", "$t8", "$t9",
    "$k0", "$k1", "$gp", "$sp", "$fp", "$ra",
];

pub struct AddressRange {
    starting_addr: u32,
    last_addr: u32,
    // size: u32,
}

pub struct Cpu {
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

    pub fn store32(&mut self, addr: u32, val: u32) -> Result<(), String> {
        self.interconnect.store32(addr, val)
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
        if let Some(op) = instr.sop() {
            match op {
                Opcode::Special => self.execute_special_op_instr(instr_),
                Opcode::LoadUpperImmediate => self.op_lui(instr),
                Opcode::OrImmediate => self.op_ori(instr),
                Opcode::StoreWord => self.op_sw(instr),
                Opcode::AddImmediateUnsignedWord => self.op_addiu(instr),
            }
        } else {
            panic!(
                "If I could, I'd handle this instruction: {instr_:#x} {instr_:#b}
            registers = {:#x?}",
                self.registers
            );
        }
    }

    pub fn execute_special_op_instr(&mut self, instr_: u32) {
        let instr = Instruction(instr_);
        if let Some(sop) = instr.secondary_opcode() {
            match sop {
                SecondaryOpcode::ShiftLeftLogical => self.op_sll(instr),
            }
        } else {
            panic!(
                "If I could, I'd handle this secondary-op instruction: {instr_:#x} {instr_:#b}
            registers = {:#x?}",
                self.registers
            );
        }
    }

    pub fn get_register(&self, register_index: u32) -> u32 {
        self.registers[register_index as usize]
    }

    pub fn get_registers(&self) -> &[u32] {
        &self.registers
    }

    pub fn set_register(&mut self, reg_idx: u32, val: u32) {
        self.registers[reg_idx as usize] = val;
        // Never overwrite $zero
        self.registers[0] = 0;
    }

    /// Load Upper Immediate
    // rt = immediate << 16
    fn op_lui(&mut self, instr: Instruction) {
        let rt = instr.gpr_rt();
        let imm = instr.immediate();
        self.set_register(rt, imm << 16);
    }

    /// Or Immediate
    /// rt = rs | immediate
    fn op_ori(&mut self, instr: Instruction) {
        let rt = instr.gpr_rt();
        let imm = instr.immediate();
        let res = self.get_register(rt) | imm;
        self.set_register(rt, res);
    }

    /// Store Word
    /// memory[base+offset] = rt
    fn op_sw(&mut self, instr: Instruction) {
        let rt = instr.gpr_rt();
        let base = instr.base();
        let offset = instr.offset__sign_extended();

        let addr = self.get_register(base).wrapping_add(offset);
        let val = self.get_register(rt);
        self.store32(addr, val).unwrap();
    }

    /// Shift Left Logical
    /// rd = rt << sa
    fn op_sll(&mut self, instr: Instruction) {
        let rt = instr.gpr_rt();
        let rd = instr.gpr_rd();
        let sa = instr.sa();

        let val = self.get_register(rt) << sa;

        self.set_register(rd, val);
    }

    /// Add Immediate Unsigned Word
    /// rt = rs + immediate
    ///
    /// Note: Here is what the MIPS reference says:
    ///
    /// The term “unsigned” in the instruction name is a misnomer; this
    /// operation is 32-bit modulo arithmetic that does not trap on overflow.
    /// This instruction is appropriate for unsigned arithmetic, such as
    /// address arithmetic, or integer arithmetic environments that ignore
    /// overflow, such as C language arithmetic.
    fn op_addiu(&mut self, instr: Instruction) {
        let rt = instr.gpr_rt();
        let rs = instr.gpr_rs();
        let imm = instr.immediate__sign_extended();

        let val = self.get_register(rs).wrapping_add(imm);
        self.set_register(rt, val);
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

    // Little endian (LSB goes first, i.e., the left side)
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

    #[instrument(skip(self, addr), fields(addr=%format!("{addr:#x}")))]
    pub fn load32(&self, addr: u32) -> Result<u32, String> {
        // Word addresses must be aligned by 4
        if addr % 4 != 0 {
            return Err(format!("Addr {addr} is not aligned").to_string());
        }
        if addr >= BIOS_ADDR_RANGE.starting_addr || addr < BIOS_ADDR_RANGE.last_addr {
            // The addr relative to BIOS' starting address
            let offset = addr - BIOS_ADDR_RANGE.starting_addr;
            return Ok(self.bios.load32(offset));
        }

        Err(format!("Addr {addr} not in range for any peripheral").to_string())
    }

    #[instrument(skip(self, addr, val), fields(addr=%format!("{addr:#x}"), val=%format!("{val:#x}")))]
    pub fn store32(&mut self, addr: u32, val: u32) -> Result<(), String> {
        // Word addresses must be aligned by 4
        if addr % 4 != 0 {
            return Err(format!("Addr {addr} is not aligned").to_string());
        }
        if addr >= MEM_CONTROL_ADDR_RANGE.starting_addr || addr < MEM_CONTROL_ADDR_RANGE.last_addr {
            // The addr relative to BIOS' starting address
            let offset = addr - MEM_CONTROL_ADDR_RANGE.starting_addr;

            // These registers contain the base address of the expansion 1 and 2 register
            // maps, respectively. Should never be changed from these hardcoded values.
            if offset == 0 && val != 0x1f000000 {
                return Err(
                    format!("Attempted to set bad expansion 1 base address {addr:#x}").to_string(),
                );
            }

            if offset == 4 && val != 0x1f802000 {
                return Err(
                    format!("Attempted to set bad expansion 2 base address {addr:#x}").to_string(),
                );
            }

            warn!(offset, "Unhandled write to MEM_CONTROL register");
            return Ok(());
        } else {
            todo!("Interconnect::store32!!! addr: {addr:#x}, value: {val:#x}");
        }
    }
}

struct Instruction(u32);

impl Instruction {
    fn sop(&self) -> Option<Opcode> {
        // 31..26 (6b)
        let op = self.0 >> 26;
        Opcode::from_u32(op)
    }

    // Used when primary sop == Opcode::Special
    fn secondary_opcode(&self) -> Option<SecondaryOpcode> {
        // 5..0 (6b)
        let sop = 0b0011_1111 & self.0;
        SecondaryOpcode::from_u32(sop)
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

    fn gpr_rd(&self) -> u32 {
        // 15..11 (5b)
        0b0001_1111 & (self.0 >> 11)
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

    // Force the compiler to sign-extend val
    fn immediate__sign_extended(&self) -> u32 {
        let val = self.immediate() as i16;
        val as u32
    }

    // Force the compiler to sign-extend val
    fn offset__sign_extended(&self) -> u32 {
        let val = self.immediate() as i16;
        val as u32
    }

    // I think sa stands for 'shift amount', maybe...
    fn sa(&self) -> u32 {
        // 10..6 (5b)
        0b0001_1111 & (self.0 >> 6)
    }
}

#[derive(FromPrimitive, ToPrimitive)]
#[repr(u32)]
enum Opcode {
    Special = 0,
    LoadUpperImmediate = 0b0000_1111,
    OrImmediate = 0b0000_1101,
    StoreWord = 0b0010_1011,
    AddImmediateUnsignedWord = 0b0000_1001,
}

#[derive(FromPrimitive, ToPrimitive)]
#[repr(u32)]
enum SecondaryOpcode {
    ShiftLeftLogical = 0,
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
