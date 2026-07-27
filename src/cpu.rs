use std::fs::File;
use std::io::{Read, Result};
use minifb::{Key, Window};

#[derive(Debug, Clone)]
pub struct CPU {
    pub memory: [u8; 4096],
    pub display: [bool; 32 * 64],
    pub pc: u16,
    pub i: u16,
    pub stack: [u16; 16],
    pub sp: u8,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub v: [u8; 16],
    pub keypad: [bool; 16],
    pub prev_keypad: [bool; 16],
}

impl CPU {
    pub fn new() -> CPU {
        let mut memory: [u8; 4096] = [0; 4096];
        memory[FONT_START..FONT_START + FONT_SET.len()].copy_from_slice(&FONT_SET);
        CPU {
            memory,
            display: [false; 32 * 64],
            pc: 0x200,
            i: 0,
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            v: [0; 16],
            keypad: [false; 16],
            prev_keypad: [false; 16],
        }
    }

    // read the rom file and copy the binaries to the memory, starting from 0x200
    pub fn load_rom(&mut self, path: &str) -> Result<()> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        for (i, &byte) in buffer.iter().enumerate() {
            if 0x200 + i < self.memory.len() {
                self.memory[0x200 + i] = byte;
            }
        }
        Ok(())
    }

    pub fn step(&mut self) {
        let opcode = fetch(self);
        execute(self, opcode);
    }

    // get the buffer for minifb redering
    pub fn get_display_buffer(&self) -> Vec<u32> {
        self.display.iter().map(|&pixel| if pixel { 0xFFFFFFFF } else { 0x00000000 }).collect()
    }

    pub fn update_keypad(&mut self, window: &Window) {
        self.prev_keypad = self.keypad;
        for (i, &key) in KEY_MAP.iter().enumerate() {
            self.keypad[i] = window.is_key_down(key);
        }
    }

    // helper for implementation of DXYN
    pub fn draw(&mut self, x: u8, y: u8, n: u8) {
        let start_x = (self.v[x as usize] & 63) as usize;
        let start_y = (self.v[y as usize] & 31) as usize;
        self.v[0xF] = 0;

        for row in 0..n as usize{
            let py = start_y + row;
            // out of display
            if py >= 32 { break; }

            let byte = self.memory[(self.i as usize) + row];

            for col in 0..8 {
                let px = start_x + col;
                // out of display
                if px >= 64 { break; }
                
                let display_index = py * 64 + px;
                let display_pixel = self.display[display_index];
                // 0x80 = 1000 0000
                let sprite_pixel = (byte & (0x80 >> col)) != 0;

                if sprite_pixel {
                    if display_pixel { self.v[0x0F] = 1; }
                    self.display[display_index] ^= true;
                }
            }
        }
    }
}


pub fn fetch(cpu: &mut CPU) -> u16 {
    let opcode = (u16::from(cpu.memory[cpu.pc as usize]) << 8) | (u16::from(cpu.memory[(cpu.pc + 1) as usize]));
    cpu.pc += 2;
    opcode
}

pub fn decode(opcode: u16) -> (u8, u8, u8, u8) {
    (
        ((opcode & 0xF000) >> 12) as u8,
        ((opcode & 0x0F00) >> 8) as u8,
        ((opcode & 0x00F0) >> 4) as u8,
        ((opcode & 0x000F)) as u8
    )
}

pub fn execute(cpu: &mut CPU, opcode: u16) {
    let nibbles = decode(opcode);
    match nibbles {
        // 00E0: clear display
        (0x0, 0x0, 0xE, 0x0) => cpu.display.fill(false),
        // 1NNN: jump to NNN
        (0x1, _, _, _) => cpu.pc = opcode & 0x0FFF,
        // 6XNN: set VX to NN
        (0x6, x, _, _) => {
            cpu.v[x as usize] = (opcode & 0x00FF) as u8;
        },
        // 7XNN: add NN to VX
        (0x7, x, _, _) => {
            cpu.v[x as usize] = cpu.v[x as usize].wrapping_add((opcode & 0x00FF) as u8);
        },
        // ANNN: set i to NNN
        (0xA, _, _, _) => cpu.i = opcode & 0x0FFF,
        // DXYN: draw sprite of height N at position (VX, VY)
        (0xD, x, y, n) => cpu.draw(x, y, n),
        // 00EE: return from subroutine
        (0x0, 0x0, 0xE, 0xE) => {
            cpu.sp -= 1;
            cpu.pc = cpu.stack[cpu.sp as usize];
        },
        // 2NNN: enter subroutine
        (0x2, _, _, _) => {
            cpu.stack[cpu.sp as usize] = cpu.pc;
            cpu.sp += 1;
            cpu.pc = opcode & 0x0FFF;
        },
        // 3XNN: skip one instruction if VX == NN
        // 4XNN: skip one instruction if VX != NN
        // 5XY0: skip one instruction if VX == VY
        // 9XY0: skip one instruction if VX != VY
        (0x3, x, _, _) => {
            if cpu.v[x as usize] == (opcode & 0x00FF) as u8 {
                cpu.pc += 2;
            }
        },
        (0x4, x, _, _) => {
            if cpu.v[x as usize] != (opcode & 0x00FF) as u8 {
                cpu.pc += 2;
            }
        },
        (0x5, x, y, 0x0) => {
            if cpu.v[x as usize] == cpu.v[y as usize] {
                cpu.pc += 2;
            }
        },
        (0x9, x, y, 0x0) => {
            if cpu.v[x as usize] != cpu.v[y as usize] {
                cpu.pc += 2;
            }
        },
        (0x8, x, y, 0x0) => cpu.v[x as usize] = cpu.v[y as usize],
        (0x8, x, y, 0x1) => cpu.v[x as usize] |= cpu.v[y as usize],
        (0x8, x, y, 0x2) => cpu.v[x as usize] &= cpu.v[y as usize],
        (0x8, x, y, 0x3) => cpu.v[x as usize] ^= cpu.v[y as usize],
        (0x8, x, y, 0x4) => {
            let vx = cpu.v[x as usize];
            let vy = cpu.v[y as usize];
            let (result, overflowed) = vx.overflowing_add(vy);
            cpu.v[x as usize] = result;
            cpu.v[0xF] = if overflowed { 1 } else { 0 };
        },
        (0x8, x, y, 0x5) => {
            let vx = cpu.v[x as usize];
            let vy = cpu.v[y as usize];
            let (result, overflowed) = vx.overflowing_sub(vy);
            cpu.v[x as usize] = result;
            cpu.v[0xF] = if overflowed { 0 } else { 1 };
        },
        (0x8, x, y, 0x7) => {
            let vx = cpu.v[x as usize];
            let vy = cpu.v[y as usize];
            let (result, overflowed) = vy.overflowing_sub(vx);
            cpu.v[x as usize] = result;
            cpu.v[0xF] = if overflowed { 0 } else { 1 };
        },
        (0x8, x, y, 0x6) => {
            cpu.v[x as usize] = cpu.v[y as usize];// optional part
            let bit = (0b00000001 & cpu.v[x as usize]) == 0b00000001;
            cpu.v[x as usize] = cpu.v[x as usize] >> 1;
            cpu.v[0xF] = if bit { 1 } else { 0 };
        },
        (0x8, x, y, 0xE) => {
            cpu.v[x as usize] = cpu.v[y as usize];// optional part
            let bit = (0b10000000 & cpu.v[x as usize]) == 0b10000000;
            cpu.v[x as usize] = cpu.v[x as usize] << 1;
            cpu.v[0xF] = if bit { 1 } else { 0 };
        },
        (0xB, _, _, _) => cpu.pc = (opcode & 0x0FFF) + cpu.v[0x0] as u16,
        (0xC, x, _, _) => {
            let rand: u8 = rand::random();
            cpu.v[x as usize] = rand & ((opcode & 0x00FF) as u8);
        },
        (0xE, x, 0x9, 0xE) => if cpu.keypad[cpu.v[x as usize] as usize] { cpu.pc += 2 },
        (0xE, x, 0xA, 0x1) => if !cpu.keypad[cpu.v[x as usize] as usize] { cpu.pc += 2 },
        (0xF, x, 0x0, 0x7) => cpu.v[x as usize] = cpu.delay_timer,
        (0xF, x, 0x1, 0x5) => cpu.delay_timer = cpu.v[x as usize],
        (0xF, x, 0x1, 0x8) => cpu.sound_timer = cpu.v[x as usize],
        (0xF, x, 0x1, 0xE) => cpu.i = cpu.i.wrapping_add(cpu.v[x as usize] as u16),
        (0xF, x, 0x0, 0xA) => {
            let mut key_released = None;
            for i in 0..16 { 
                if cpu.prev_keypad[i] && !cpu.keypad[i] {
                    key_released = Some(i);
                    break;
                }
            }
            match key_released {
                Some(i) => cpu.v[x as usize] = i as u8,
                None => cpu.pc -= 2,
            }
        },
        (0xF, x, 0x3, 0x3) => {
            let mut vx = cpu.v[x as usize];
            for i in 0..3 {
                cpu.memory[(cpu.i + (2 - i)) as usize] = vx % 10;
                vx /= 10;
            }
        },
        (0xF, x, 0x5, 0x5) => {
            // quirk
            for counter in 0..=x as u16{
                cpu.memory[(cpu.i + counter) as usize] = cpu.v[counter as usize];
            }
        },
        (0xF, x, 0x6, 0x5) => {
            // quirk
            for counter in 0..=x as u16 {
                cpu.v[counter as usize] = cpu.memory[(cpu.i + counter) as usize];
            }
        },
        (0xF, x, 0x2, 0x9) => {
            let digit = cpu.v[x as usize] & 0x0F;
            cpu.i = FONT_START as u16 + (digit as u16) * 5;
        }
        _ => todo!("Unimplemented instruction: {:#06X} in program counter: {:#06X}", opcode, cpu.pc - 2),
    }

}

const FONT_START: usize = 0x050;
const FONT_SET: [u8; 80] = [
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
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

const KEY_MAP: [Key; 16] = [
    Key::X,    // 0
    Key::Key1, // 1
    Key::Key2, // 2
    Key::Key3, // 3
    Key::Q,    // 4
    Key::W,    // 5
    Key::E,    // 6
    Key::A,    // 7
    Key::S,    // 8
    Key::D,    // 9
    Key::Z,    // A
    Key::C,    // B
    Key::Key4, // C
    Key::R,    // D
    Key::F,    // E
    Key::V,    // F
];