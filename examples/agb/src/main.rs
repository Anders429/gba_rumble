//! Example using the `agb` crate.

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
fn main(mut _gba: Gba) -> ! {
    let vblank = VBlank::get();
    let mut button_controller = ButtonController::new();

    vblank.wait_for_vblank();
    // Detecting the Game Boy Player must be one of the first things done in your program.
    if let Some(game_boy_player_rumble) = gba_rumble::GameBoyPlayer::detect() {
        // To use the Game Boy Player's rumble when it is present, configure the interrupt handler
        // to handle incoming serial inputs using `game_boy_player_interrupt()`. The function will
        // respond with the appropriate messages through serial output.
        let _serial_interrupt = unsafe {
            add_interrupt_handler(Interrupt::Serial, |_| {
                gba_rumble::game_boy_player_interrupt()
            })
        };
        // Enable serial communication. `agb` doesn't currently natively support this, so we have
        // to do it manually.
        unsafe {
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
        }
        loop {
            vblank.wait_for_vblank();
            // The Game Boy Player supports starting, stopping, and hard stopping the rumble motor
            // in the controller.
            button_controller.update();
            if button_controller.is_pressed(Button::A) {
                game_boy_player_rumble.start();
            } else if button_controller.is_pressed(Button::B) {
                game_boy_player_rumble.stop();
            } else if button_controller.is_pressed(Button::START) {
                game_boy_player_rumble.hard_stop();
            }
            // You must call `update()` every frame to restart the serial communication.
            game_boy_player_rumble.update();
        }
    } else {
        // Rumble can also be done with the cartridge directly by using GPIO.
        let gpio_rumble = gba_rumble::Gpio;
        loop {
            vblank.wait_for_vblank();
            // GPIO supports starting and stopping the rumble motor in the cartridge.
            button_controller.update();
            if button_controller.is_pressed(Button::A) {
                gpio_rumble.start();
            } else if button_controller.is_pressed(Button::B) {
                gpio_rumble.stop();
            }
        }
    }
}
