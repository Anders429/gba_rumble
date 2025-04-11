//! Example using the AGB crate.

#![no_std]
#![no_main]

use agb::{
    Gba,
    input::{Button, ButtonController},
    interrupt::{Interrupt, VBlank, add_interrupt_handler},
};

// `agb` does not currently have any support for serial input/output, so for now we use pointers to
// the mmio addresses.
const RCNT: *mut u16 = 0x0400_0134 as *mut u16;
const SIOCNT: *mut u16 = 0x0400_0128 as *mut u16;

#[agb::entry]
fn main(mut gba: Gba) -> ! {
    let _ = mgba_log::init();
    let serial_interrupt = unsafe {
        add_interrupt_handler(Interrupt::Serial, |_| {
            gba_rumble::game_boy_player_interrupt()
        })
    };
    let vblank = VBlank::get();
    unsafe {
        RCNT.write(0);
        SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
    }
    let mut button_controller = ButtonController::new();

    vblank.wait_for_vblank();
    if let Some(game_boy_player_rumble) = gba_rumble::GameBoyPlayer::detect() {
        loop {
            vblank.wait_for_vblank();
            button_controller.update();
            if button_controller.is_pressed(Button::A) {
                game_boy_player_rumble.start();
            } else if button_controller.is_pressed(Button::B) {
                game_boy_player_rumble.stop();
            } else if button_controller.is_pressed(Button::START) {
                game_boy_player_rumble.hard_stop();
            }
            game_boy_player_rumble.update();
        }
    } else {
        let gpio_rumble = gba_rumble::Gpio;
        loop {
            vblank.wait_for_vblank();
            button_controller.update();
            if button_controller.is_pressed(Button::A) {
                gpio_rumble.start();
            } else if button_controller.is_pressed(Button::B) {
                gpio_rumble.stop();
            }
        }
    }
}
