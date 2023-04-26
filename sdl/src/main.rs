use std::{env, fs};
use std::io::{self, Read};
use std::time::{Duration, Instant};

use sdl2::audio::{AudioCallback, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};

use chip8::Chip8;

#[derive(Debug)]
pub enum FrontError {
    Chip8(String),
    Io(io::Error),
}

impl From<io::Error> for FrontError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<String> for FrontError {
    fn from(err: String) -> Self {
        Self::Chip8(err)
    }
}


struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn main() -> Result<(), FrontError> {
    let argv: Vec<_> = env::args().collect();
    if argv.len() != 2 {
        println!("Usage: {} <program_path>", &argv[0]);
        return Ok(());
    }
    let mut rom: Vec<u8> = Vec::new();
    fs::OpenOptions::new()
        .read(true)
        .open(&argv[1])?
        .read_to_end(&mut rom)?;
    let mut chip8 = Chip8::new();
    chip8.load_rom(&rom)?;

    let sdl_context = sdl2::init()?;
    let audio_subsystem = sdl_context.audio()?;

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1), // mono
        samples: None,     // default sample size
    };

    let device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        // initialize the audio callback
        SquareWave {
            phase_inc: 440.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.25,
        }
    })?;

    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window(
            "chip8",
            chip8::DISPLAY_WIDTH as u32 * 16,
            chip8::DISPLAY_HEIGHT as u32 * 16,
        )
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let texture_creator = canvas.texture_creator();
    let mut tex_display = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGB24,
            chip8::DISPLAY_WIDTH as u32,
            chip8::DISPLAY_HEIGHT as u32,
        )
        .map_err(|e| e.to_string())?;

    let frame_duration = Duration::new(0, 1_000_000_000u32 / 60);
    let mut timestamp = Instant::now();

    let mut event_pump = sdl_context.event_pump()?;

    let mut keypad: u16 = 0u16;

    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    break 'main;
                },
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    keypad |= match keycode {
                        Keycode::Num1 => 1 << 0x1,
                        Keycode::Num2 => 1 << 0x2,
                        Keycode::Num3 => 1 << 0x3,
                        Keycode::Num4 => 1 << 0xC,
                        Keycode::Q => 1 << 0x4,
                        Keycode::W => 1 << 0x5,
                        Keycode::E => 1 << 0x6,
                        Keycode::R => 1 << 0xD,
                        Keycode::A => 1 << 0x7,
                        Keycode::S => 1 << 0x8,
                        Keycode::D => 1 << 0x9,
                        Keycode::F => 1 << 0xE,
                        Keycode::Z => 1 << 0xA,
                        Keycode::X => 1 << 0x0,
                        Keycode::C => 1 << 0xB,
                        Keycode::V => 1 << 0xF,
                        _ => 0,
                    };
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => {
                    keypad &= !match keycode {
                        Keycode::Num1 => 1 << 0x1,
                        Keycode::Num2 => 1 << 0x2,
                        Keycode::Num3 => 1 << 0x3,
                        Keycode::Num4 => 1 << 0xC,
                        Keycode::Q => 1 << 0x4,
                        Keycode::W => 1 << 0x5,
                        Keycode::E => 1 << 0x6,
                        Keycode::R => 1 << 0xD,
                        Keycode::A => 1 << 0x7,
                        Keycode::S => 1 << 0x8,
                        Keycode::D => 1 << 0x9,
                        Keycode::F => 1 << 0xE,
                        Keycode::Z => 1 << 0xA,
                        Keycode::X => 1 << 0x0,
                        Keycode::C => 1 << 0xB,
                        Keycode::V => 1 << 0xF,
                        _ => 0,
                    };
                }
                _ => {}
            }
        }

        if chip8.sound_timer > 0 {
            device.resume();
        } else {
            device.pause();
        }
        chip8.keypad = keypad;

        chip8.frame()?;

        tex_display.with_lock(None, |buffer: &mut [u8], _pitch: usize| {
            for display_idx in 0..chip8::DISPLAY_SIZE {
                let byte = chip8.display[display_idx];
                for byte_idx in 0..8 {
                    let lit = (byte) >> byte_idx & 1 != 0;
                    let buffer_idx = (display_idx * 8 + (7 - byte_idx)) * 3;
                    let color = if lit {
                        255
                    } else {
                        0
                    };
                    buffer[buffer_idx] = color;
                    buffer[buffer_idx + 1] = color;
                    buffer[buffer_idx + 2] = color;
                }
            }
        })?;

        canvas.clear();
        canvas.copy(&tex_display, None, None)?;
        canvas.present();

        let now = Instant::now();
        let sleep_dur = frame_duration
            .checked_sub(now.saturating_duration_since(timestamp))
            .unwrap_or(Duration::new(0, 0));
        std::thread::sleep(sleep_dur);
        timestamp = now;
    }

    Ok(())
}