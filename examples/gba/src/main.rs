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
    if bits.serial() {
        gba_rumble::game_boy_player_interrupt();
    }
}

#[unsafe(no_mangle)]
pub fn main() {
    let _ = mgba_log::init();
    RUST_IRQ_HANDLER.write(Some(irq_handler));
    DISPSTAT.write(DisplayStatus::new().with_irq_vblank(true));
    IE.write(IrqBits::new().with_vblank(true).with_serial(true));
    IME.write(true);
    RCNT.write(0);
    SIOCNT.write(0x4000 | 0x1000 | 8);

    VBlankIntrWait();
    if let Some(game_boy_player_rumble) = gba_rumble::GameBoyPlayer::detect() {
        loop {
            VBlankIntrWait();
            let keys = KEYINPUT.read();
            if keys.a() {
                game_boy_player_rumble.start();
            } else if keys.b() {
                game_boy_player_rumble.stop();
            } else if keys.start() {
                game_boy_player_rumble.hard_stop();
            }
            game_boy_player_rumble.update();
        }
    } else {
        let gpio_rumble = gba_rumble::Gpio;
        loop {
            VBlankIntrWait();
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
