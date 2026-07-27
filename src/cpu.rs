use std::fs::File;
use std::io::{Read, Result};

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
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            memory: [0; 4096],
            display: [false; 32 * 64],
            pc: 0x200,
            i: 0,
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            v: [0; 16],
            keypad: [false; 16]
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
        (0x0, 0x0, 0xE, 0x0) => cpu.display.fill(false),
        (0x1, _, _, _) => cpu.pc = opcode & 0x0FFF,
        (0x6, x, _, _) => {
            cpu.v[x as usize] = (opcode & 0x00FF) as u8;
        },
        (0x7, x, _, _) => {
            cpu.v[x as usize] = cpu.v[x as usize].wrapping_add((opcode & 0x00FF) as u8);
        },
        (0xA, _, _, _) => cpu.i = opcode & 0x0FFF,
        (0xD, x, y, n) => cpu.draw(x, y, n),
        _ => todo!("Unimplemented instruction: {:#06X} in program counter: {:#06X}", opcode, cpu.pc - 2),
    }

}