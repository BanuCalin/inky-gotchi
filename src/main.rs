use std::thread::sleep;
use std::time::Duration;

use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::iso_8859_15::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::Point;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::sysfs_gpio::Direction;
use linux_embedded_hal::Delay;
use linux_embedded_hal::{Pin, Spidev};

use ssd1675::{Builder, Color, Dimensions, Display, GraphicDisplay, Rotation};

const ROWS: u16 = 248;
const COLS: u8 = 120;

#[rustfmt::skip]
const LUT: [u8; 70] = [
    // Phase 0     Phase 1     Phase 2     Phase 3     Phase 4     Phase 5     Phase 6
    // A B C D     A B C D     A B C D     A B C D     A B C D     A B C D     A B C D
    // 0b01001000, 0b10100000, 0b00010000, 0b00010000, 0b00010011, 0b00000000, 0b00000000,  // LUT0 - Black
    // 0b01001000, 0b10100000, 0b10000000, 0b00000000, 0b00000011, 0b00000000, 0b00000000,  // LUTT1 - White
    // 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,  // IGNORE
    // 0b01001000, 0b10100101, 0b00000000, 0b10111011, 0b00000000, 0b00000000, 0b00000000,  // LUT3 - Red
    // 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,  // LUT4 - VCOM
    0b11111010, 0b10010100, 0b10001100, 0b11000000, 0b11010000, 0b00000000, 0b00000000,
    0b11111010, 0b10010100, 0b00101100, 0b10000000, 0b11100000, 0b00000000, 0b00000000,
    0b11111010, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b11111010, 0b10010100, 0b11111000, 0b10000000, 0b01010000, 0b00000000, 0b11001100,
    0b10111111, 0b01011000, 0b11111100, 0b10000000, 0b11010000, 0b00000000, 0b00010001,
    // Duration            |  Repeat
    // A   B     C     D   |
    // 64,   12,   32,   12,    6,   // 0 Flash
    // 16,   8,    4,    4,     6,   // 1 clear
    // 4,    8,    8,    16,    16,  // 2 bring in the black
    // 2,    2,    2,    64,    32,  // 3 time for red
    // 2,    2,    2,    2,     2,   // 4 final black sharpen phase
    // 0,    0,    0,    0,     0,   // 5
    // 0,    0,    0,    0,     0    // 6
    0x40, 0x10, 0x40, 0x10, 0x08,
    0x08, 0x10, 0x04, 0x04, 0x10,
    0x08, 0x08, 0x03, 0x08, 0x20,
    0x08, 0x04, 0x00, 0x00, 0x10,
    0x10, 0x08, 0x08, 0x00, 0x20,
    0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00
];

fn main() -> Result<(), std::io::Error> {
    // Configure SPI
    let mut spi = Spidev::open("/dev/spidev0.0").expect("SPI device");
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(4_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options).expect("SPI configuration");

    // https://pinout.xyz/pinout/inky_phat
    // Configure Digital I/O Pins
    let cs = Pin::new(8); // BCM8
                          // cs.export().expect("cs export");
                          // while !cs.is_exported() {}
                          // cs.set_direction(Direction::Out).expect("CS Direction");
                          // cs.set_value(1).expect("CS Value set to 1");

    let busy = Pin::new(17); // BCM17
    busy.export().expect("busy export");
    while !busy.is_exported() {}
    busy.set_direction(Direction::In).expect("busy Direction");

    let dc = Pin::new(22); // BCM22
    dc.export().expect("dc export");
    while !dc.is_exported() {}
    dc.set_direction(Direction::Out).expect("dc Direction");
    dc.set_value(1).expect("dc Value set to 1");

    let reset = Pin::new(27); // BCM27
    reset.export().expect("reset export");
    while !reset.is_exported() {}
    reset
        .set_direction(Direction::Out)
        .expect("reset Direction");
    reset.set_value(1).expect("reset Value set to 1");
    println!("Pins configured");

    // Initialise display controller
    let mut delay = Delay {};

    let controller = ssd1675::interface::Interface::new(spi, cs, busy, dc, reset);

    let mut black_buffer = [0u8; ROWS as usize * COLS as usize / 8];
    let mut red_buffer = [0u8; ROWS as usize * COLS as usize / 8];
    let config = Builder::new()
        .dimensions(Dimensions {
            rows: ROWS,
            cols: COLS,
        })
        .rotation(Rotation::Rotate270)
        .lut(&LUT)
        .build()
        .expect("invalid configuration");

    let display = Display::new(controller, config);
    let mut display = GraphicDisplay::new(display, &mut black_buffer, &mut red_buffer);

    let style = MonoTextStyle::new(&FONT_10X20, Color::Color);
    // Main loop. Displays CPU temperature, uname, and uptime every minute with a red Raspberry Pi
    // header.
    loop {
        display.reset(&mut delay).expect("error resetting display");
        println!("Reset and initialised");
        let one_minute = Duration::from_secs(60);

        display.clear(Color::White);
        println!("Clear");

        Text::new("Buna Mara!", Point::new(50, 50), style).draw(&mut display);

        display.update(&mut delay).expect("error updating display");
        println!("Update...");

        println!("Finished - going to sleep");
        display.deep_sleep()?;

        sleep(one_minute);
    }
}
