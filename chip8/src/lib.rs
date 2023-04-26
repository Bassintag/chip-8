use rand::Rng;
use rand::rngs::ThreadRng;

macro_rules! nnn {
    ($op0: expr, $op1: expr) => {
        ((($op0) & 0x0f) as u16) << 8 | (($op1) as u16)
    };
}

macro_rules! lo {
    ($op0: expr) => (($op0) & 0x0f);
}

macro_rules! hi {
    ($op0: expr) => ((($op0) & 0xf0) >> 4);
}

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

const MEMORY_SIZE: usize = 4096;
const RESERVED_MEMORY_SIZE: usize = 512;
const REGISTERS: usize = 16;
const FRAME_DURATION: isize = 16666;

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;
pub const DISPLAY_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT / 8;

pub struct Chip8 {
    rng: ThreadRng,
    pub memory: [u8; MEMORY_SIZE],
    pub pc: u16,
    pub i: u16,
    pub stack: Vec<u16>,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub registers: [u8; REGISTERS],
    pub display: [u8; DISPLAY_SIZE],
    pub keypad: u16,
}

impl Chip8 {
    pub fn new() -> Self {
        let rng = rand::thread_rng();
        let mut memory = [0; MEMORY_SIZE];
        memory[0..FONT.len()].copy_from_slice(&FONT);
        return Self {
            rng,
            memory,
            pc: (RESERVED_MEMORY_SIZE) as u16,
            i: 0,
            stack: vec![],
            delay_timer: 0,
            sound_timer: 0,
            registers: [0; REGISTERS],
            display: [0; DISPLAY_SIZE],
            keypad: 0,
        };
    }

    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), String> {
        if rom.len() > MEMORY_SIZE - RESERVED_MEMORY_SIZE {
            return Err("not enough memory to load rom".to_string());
        }
        self.memory[RESERVED_MEMORY_SIZE..RESERVED_MEMORY_SIZE + rom.len()].copy_from_slice(rom);
        return Ok(());
    }

    pub fn frame(&mut self) -> Result<(), String> {
        if self.delay_timer != 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer != 0 {
            self.sound_timer -= 1;
        }
        let mut time: isize = FRAME_DURATION;
        while time > 0 {
            if self.pc as usize >= MEMORY_SIZE - 1 {
                return Err("pc out of memory bounds".to_string());
            }
            let op0 = self.memory[self.pc as usize];
            let op1 = self.memory[(self.pc + 1) as usize];
            self.pc += 2;
            let op_time = self.step(op0, op1)?;
            time -= op_time as isize;
        }
        return Ok(());
    }

    pub fn step(&mut self, op0: u8, op1: u8) -> Result<usize, String> {
        // println!("0x{:x}{:x}{:x}{:x}", hi!(op0), lo!(op0), hi!(op1), lo!(op1));
        return Ok(match op0 & 0xf0 {
            0x00 => match op1 {
                // 00e0
                0xe0 => self.op_cls(),
                0xee => self.op_ret(),
                _ => {
                    return Err(format!("Invalid op {:x}{:x}{:x}{:x}", hi!(op0), lo!(op0), hi!(op1), lo!(op1)));
                }
            }
            // 1nnn
            0x10 => self.op_jp(nnn!(op0, op1)),
            // 2nnn
            0x20 => self.op_call(nnn!(op0, op1)),
            // 3xnn
            0x30 => self.op_se(lo!(op0), op1),
            // 4xnn
            0x40 => self.op_sne(lo!(op0), op1),
            // 5xy0
            0x50 => self.op_sexy(lo!(op0), hi!(op1)),
            // 6xnn
            0x60 => self.op_ld(lo!(op0), op1),
            // 7xnn
            0x70 => self.op_add(lo!(op0), op1),
            0x80 => match op1 & 0x0f {
                // 8xy0
                0x00 => self.op_ldxy(lo!(op0), hi!(op1)),
                // 8xy1
                0x01 => self.op_orxy(lo!(op0), hi!(op1)),
                // 8xy2
                0x02 => self.op_andxy(lo!(op0), hi!(op1)),
                // 8xy3
                0x03 => self.op_xorxy(lo!(op0), hi!(op1)),
                // 8xy4
                0x04 => self.op_addxy(lo!(op0), hi!(op1)),
                // 8xy5
                0x05 => self.op_subxy(lo!(op0), hi!(op1)),
                // 8xy6
                0x06 => self.op_shrxy(lo!(op0)),
                // 8xy7
                0x07 => self.op_subnxy(lo!(op0), hi!(op1)),
                // 8xyE
                0x0E => self.op_shlxy(lo!(op0)),
                _ => {
                    return Err(format!("Invalid op {:x}{:x}{:x}{:x}", hi!(op0), lo!(op0), hi!(op1), lo!(op1)));
                }
            }
            // 9xy0
            0x90 => self.op_snexy(lo!(op0), hi!(op1)),
            // Annn
            0xA0 => self.op_ldi(nnn!(op0, op1)),
            // Bnnn
            0xB => self.op_jp0(nnn!(op0, op1)),
            // Cxkk
            0xC0 => self.op_rndx(lo!(op0), op1),
            // Dxyn
            0xD0 => self.op_drw(lo!(op0), hi!(op1), lo!(op1)),
            0xE0 => match op1 {
                //Ex9E
                0x9E => self.op_skpx(lo!(op0)),
                //ExA1
                0xA1 => self.op_sknpx(lo!(op0)),
                _ => {
                    return Err(format!("Invalid op {:x}{:x}{:x}{:x}", hi!(op0), lo!(op0), hi!(op1), lo!(op1)));
                }
            }
            0xF0 => match op1 {
                // 0xFx07
                0x07 => self.op_ldxdt(lo!(op0)),
                // 0Fx0A
                0x0A => self.op_ldxk(lo!(op0)),
                // 0xFx15
                0x15 => self.op_lddtx(lo!(op0)),
                // 0xFx15
                0x18 => self.op_ldstx(lo!(op0)),
                // 0xFx1E
                0x1E => self.op_addix(lo!(op0)),
                // 0xFx29
                0x29 => self.op_ldfx(lo!(op0)),
                // 0xFx33
                0x33 => self.op_ldbx(lo!(op0)),
                // 0xFx55
                0x55 => self.op_ldix(lo!(op0)),
                // 0xFx65
                0x65 => self.op_ldxi(lo!(op0)),
                _ => {
                    return Err(format!("Invalid op {:x}{:x}{:x}{:x}", hi!(op0), lo!(op0), hi!(op1), lo!(op1)));
                }
            },
            _ => {
                return Err(format!("Invalid op {:x}{:x}{:x}{:x}", hi!(op0), lo!(op0), hi!(op1), lo!(op1)));
            }
        });
    }

    // 00e0
    fn op_cls(&mut self) -> usize {
        self.display.fill(0);
        return 109;
    }

    // 00e0
    fn op_ret(&mut self) -> usize {
        let addr = self.stack.pop().unwrap();
        self.pc = addr;
        return 105;
    }

    // 1nnn
    fn op_jp(&mut self, addr: u16) -> usize {
        self.pc = addr;
        return 105;
    }

    // 2nnn
    fn op_call(&mut self, addr: u16) -> usize {
        self.stack.push(self.pc);
        self.pc = addr;
        return 105;
    }

    // 3xnn
    fn op_se(&mut self, vx: u8, byte: u8) -> usize {
        if self.registers[vx as usize] == byte {
            self.pc += 2;
            return 64;
        }
        return 46;
    }

    // 4xnn
    fn op_sne(&mut self, vx: u8, byte: u8) -> usize {
        if self.registers[vx as usize] != byte {
            self.pc += 2;
            return 64;
        }
        return 46;
    }

    // 5xy0
    fn op_sexy(&mut self, vx: u8, vy: u8) -> usize {
        if self.registers[vx as usize] == self.registers[vy as usize] {
            self.pc += 2;
            return 82;
        }
        return 64;
    }

    // 6xnn
    fn op_ld(&mut self, vx: u8, byte: u8) -> usize {
        self.registers[vx as usize] = byte;
        return 27;
    }

    // 7xnn
    fn op_add(&mut self, vx: u8, byte: u8) -> usize {
        self.registers[vx as usize] = self.registers[vx as usize].wrapping_add(byte);
        return 45;
    }

    // 8xy0
    fn op_ldxy(&mut self, vx: u8, vy: u8) -> usize {
        self.registers[vx as usize] = self.registers[vy as usize];
        return 200;
    }

    // 8xy1
    fn op_orxy(&mut self, vx: u8, vy: u8) -> usize {
        self.registers[vx as usize] |= self.registers[vy as usize];
        return 200;
    }

    // 8xy2
    fn op_andxy(&mut self, vx: u8, vy: u8) -> usize {
        self.registers[vx as usize] &= self.registers[vy as usize];
        return 200;
    }

    // 8xy3
    fn op_xorxy(&mut self, vx: u8, vy: u8) -> usize {
        self.registers[vx as usize] ^= self.registers[vy as usize];
        return 200;
    }

    // 8xy4
    fn op_addxy(&mut self, vx: u8, vy: u8) -> usize {
        let (result, overflows) = self.registers[vx as usize]
            .overflowing_add(self.registers[vy as usize]);
        self.registers[vx as usize] = result;
        self.registers[0xf] = if overflows { 1 } else { 0 };
        return 200;
    }

    // 8xy5
    fn op_subxy(&mut self, vx: u8, vy: u8) -> usize {
        let (result, overflows) = self.registers[vx as usize]
            .overflowing_sub(self.registers[vy as usize]);
        self.registers[vx as usize] = result;
        self.registers[0xf] = if overflows { 0 } else { 1 };
        return 200;
    }

    // 8xy6
    fn op_shrxy(&mut self, vx: u8) -> usize {
        let x = self.registers[vx as usize];
        let (res, _) = x.overflowing_shr(1);
        self.registers[vx as usize] = res;
        self.registers[0xf] = x & 0b00000001;
        return 200;
    }

    // 8xy7
    fn op_subnxy(&mut self, vx: u8, vy: u8) -> usize {
        let (result, overflows) = self.registers[vy as usize]
            .overflowing_sub(self.registers[vx as usize]);
        self.registers[vx as usize] = result;
        self.registers[0xf] = if overflows { 0 } else { 1 };
        return 200;
    }

    // 8xyE
    fn op_shlxy(&mut self, vx: u8) -> usize {
        let x = self.registers[vx as usize];
        let (res, _) = x.overflowing_shl(1);
        self.registers[vx as usize] = res;
        self.registers[0xf] = (x & 0b10000000) >> 7;
        return 200;
    }

    // 9xy0
    fn op_snexy(&mut self, vx: u8, vy: u8) -> usize {
        if self.registers[vx as usize] != self.registers[vy as usize] {
            self.pc += 2;
            return 82;
        }
        return 64;
    }

    // Annn
    fn op_ldi(&mut self, addr: u16) -> usize {
        self.i = addr;
        return 55;
    }

    // Bnnn
    fn op_jp0(&mut self, addr: u16) -> usize {
        self.pc = addr + self.registers[0x0] as u16;
        return 105;
    }

    // Cxkk
    fn op_rndx(&mut self, vx: u8, byte: u8) -> usize {
        let r: u8 = self.rng.gen();
        self.registers[vx as usize] = r & byte;
        return 164;
    }

    // Dxyn
    fn op_drw(&mut self, vx: u8, vy: u8, nibble: u8) -> usize {
        let x = self.registers[vx as usize];
        let y = self.registers[vy as usize];
        let display_x = x as usize % DISPLAY_WIDTH;
        let shift = x % 8;
        let display_column_left = display_x / 8;
        let display_column_right = (display_column_left + 1) % (DISPLAY_WIDTH / 8);
        let mut prev: u8 = 0;

        for idx in 0..nibble as usize {
            let display_y = (y as usize + idx) % DISPLAY_HEIGHT;
            let row = display_y * DISPLAY_WIDTH / 8;
            let byte = self.memory[self.i as usize + idx];

            let shifted_left = byte >> shift;
            let prev_left = &mut self.display[row + display_column_left];
            *prev_left ^= shifted_left;
            prev |= *prev_left & shifted_left;

            if shift > 0 {
                let shifted_right = byte << (8 - shift);
                let prev_right = &mut self.display[row + display_column_right];
                *prev_right ^= shifted_right;
                prev |= *prev_right & shifted_right;
            }
        }
        self.registers[0xf] = if prev != 0 { 1 } else { 0 };
        return 22734;
    }

    // Ex9E
    fn op_skpx(&mut self, vx: u8) -> usize {
        let x = self.registers[vx as usize];
        if self.keypad & ((1 as u16) << x) != 0 {
            self.pc += 2;
            return 64;
        }
        return 82;
    }

    // ExA1
    fn op_sknpx(&mut self, vx: u8) -> usize {
        let x = self.registers[vx as usize];
        if self.keypad & ((1 as u16) << x) == 0 {
            self.pc += 2;
            return 64;
        }
        return 82;
    }

    // Fx07
    fn op_ldxdt(&mut self, vx: u8) -> usize {
        self.registers[vx as usize] = self.delay_timer;
        return 45;
    }

    // Fx0A
    fn op_ldxk(&mut self, vx: u8) -> usize {
        for i in 0..16 {
            if 1 << i & self.keypad != 0 {
                self.registers[vx as usize] = i as u8;
                return 200;
            }
        }
        self.pc -= 2;
        return FRAME_DURATION as usize;
    }

    // Fx15
    fn op_lddtx(&mut self, vx: u8) -> usize {
        self.delay_timer = self.registers[vx as usize];
        return 45;
    }

    // Fx18
    fn op_ldstx(&mut self, vx: u8) -> usize {
        self.sound_timer = self.registers[vx as usize];
        return 45;
    }

    // Fx1e
    fn op_addix(&mut self, vx: u8) -> usize {
        let x = self.registers[vx as usize];
        self.i = self.i.wrapping_add(x as u16);
        return 86;
    }

    // Fx29
    fn op_ldfx(&mut self, vx: u8) -> usize {
        let x = self.registers[vx as usize];
        self.i = (x as u16) * 5;
        return 91;
    }

    // Fx33
    fn op_ldbx(&mut self, vx: u8) -> usize {
        let x = self.registers[vx as usize];
        let first = x / 100;
        let second = x / 10 % 10;
        let third = x % 10;
        self.memory[self.i as usize] = first;
        self.memory[self.i as usize + 1] = second;
        self.memory[self.i as usize + 2] = third;
        return 364 + (first as usize + second as usize + third as usize) * 73;
    }

    // Fx55
    fn op_ldix(&mut self, vx: u8) -> usize {
        for i in 0..(vx + 1) {
            let v = self.registers[i as usize];
            self.memory[i as usize + self.i as usize] = v;
        }
        return 64 * (vx as usize + 2);
    }

    // Fx65
    fn op_ldxi(&mut self, vx: u8) -> usize {
        for i in 0..(vx + 1) {
            self.registers[i as usize] = self.memory[i as usize + self.i as usize];
        }
        return 64 * (vx as usize + 2);
    }
}