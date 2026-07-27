use chip8::{self, CPU};
use minifb::{Key, Scale, Window, WindowOptions};

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

fn main() {
    let mut cpu = CPU::new();

    cpu.load_rom("game-rom/1dcell.ch8").expect("Failed to read ROM file");

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
    window.set_target_fps(120);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        cpu.update_keypad(&window);

        for _ in 0..100 {
            cpu.step();
        }

        cpu.delay_timer = cpu.delay_timer.saturating_sub(1);
        cpu.sound_timer = cpu.sound_timer.saturating_sub(1);
        
        window.update_with_buffer(&cpu.get_display_buffer(), WIDTH, HEIGHT).expect("Failed to update window.");
    }
}
