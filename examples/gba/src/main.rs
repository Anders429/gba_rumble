//! Example using the GBA crate.

#![no_std]
#![no_main]

use gba::prelude::*;

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    mgba_log::fatal!("{info}");
    loop {}
}

#[unsafe(link_section = ".iwram")]
extern "C" fn irq_handler(bits: IrqBits) {
    // To use the Game Boy Player's rumble when it is present, configure the interrupt handler to
    // handle incoming serial inputs using `game_boy_player_interrupt()`. The function will respond
    // with the appropriate messages through serial output.
    if bits.serial() {
        gba_rumble::game_boy_player_interrupt();
    }
}

#[unsafe(no_mangle)]
pub fn main() {
    RUST_IRQ_HANDLER.write(Some(irq_handler));
    DISPSTAT.write(DisplayStatus::new().with_irq_vblank(true));
    IE.write(IrqBits::new().with_vblank(true).with_serial(true));
    IME.write(true);

    VBlankIntrWait();
    // Detecting the Game Boy Player must be one of the first things done in your program.
    if let Some(game_boy_player_rumble) = gba_rumble::GameBoyPlayer::detect() {
        // Enable serial communication.
        RCNT.write(0);
        SIOCNT.write(0x4000 | 0x1000 | 8);

        loop {
            VBlankIntrWait();
            // The Game Boy Player supports starting, stopping, and hard stopping the rumble motor
            // in the controller.
            let keys = KEYINPUT.read();
            if keys.a() {
                game_boy_player_rumble.start();
            } else if keys.b() {
                game_boy_player_rumble.stop();
            } else if keys.start() {
                game_boy_player_rumble.hard_stop();
            }
            // You must call `update()` every frame to restart the serial communication.
            game_boy_player_rumble.update();
        }
    } else {
        // Rumble can also be done with the cartridge directly by using GPIO.
        let gpio_rumble = gba_rumble::Gpio;
        loop {
            VBlankIntrWait();
            // GPIO supports starting and stopping the rumble motor in the cartridge.
            let keys = KEYINPUT.read();
            if keys.a() {
                gpio_rumble.start();
            } else if keys.b() {
                gpio_rumble.stop();
            }
        }
    }
}

#[unsafe(no_mangle)]
pub fn __sync_synchronize() {}
