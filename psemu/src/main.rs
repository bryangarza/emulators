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
    interconnect: Interconnect,
}

impl Cpu {

    pub fn new() -> Self {
        Cpu {
            pc: PROGRAM_COUNTER_RESET_VALUE,
            interconnect: Interconnect::new(),
        }
    }

    pub fn load32(&self, addr: u32) -> Result<u32, String> {
        self.interconnect.load32(addr)
    }

    pub fn run_single_cycle(&mut self) {
        let instr = self.load32(self.pc).expect("Unable to load next instruction");
        self.pc = self.pc.wrapping_add(1);
        self.execute_instr(instr);
    }

    pub fn execute_instr(&mut self, instr: u32) {
        panic!("If I could, I'd handle instr 0x{instr:x}");
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
        if (addr >= BIOS_METADATA.starting_addr || addr < BIOS_METADATA.last_addr) {
            // The addr relative to BIOS' starting address 
            let offset = addr - BIOS_METADATA.starting_addr;
            return Ok(self.bios.load32(offset))
        }

        Err(format!("Addr {addr} not in range for any peripheral").to_string())   
    }
}