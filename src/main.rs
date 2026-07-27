use chip8::{self, CPU};
use minifb::{Key, Scale, Window, WindowOptions};

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

fn main() {
    let mut cpu = CPU::new();

    cpu.load_rom("test-rom/2-ibm-logo.ch8").expect("Failed to read ROM file");

    let mut window = Window::new(
        "Chip-8 Emulator",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X8,
            ..Default::default()
        }
    ).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // frame limit 60 FPS
    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for _ in 0..10 {
            cpu.step();
        }
        
        window.update_with_buffer(&cpu.get_display_buffer(), WIDTH, HEIGHT).expect("Failed to update window.");
    }
}
