//! Library for enabling rumble functionality on the Game Boy Advance.
//!
//! This crate supports rumble through both the cartridge itself using general purpose I/O (GPIO)
//! and the Game Boy Player's rumble functionality. Functionality is provided for detecting
//! available rumble features and using them fully.
//!
//! The library is designed to be usable regardless of what other GBA development libraries may be
//! in use. It is usable with popular libraries like [`gba`](https://docs.rs/gba/latest/gba/) and
//! [`agb`](https://docs.rs/agb/latest/agb/index.html).
//!
//! # Usage
//! ## Cartridge (GPIO) Rumble
//! To use a cartridge's built-in rumble through general purpose I/O (GPIO), use the [`Gpio`]
//! struct. No detection logic is available, as there is no reliable way to detect GPIO rumble.
//! Calling these functions when rumble is not available will do nothing.
//!
//! ``` rust
//! let gpio = gba_rumble::Gpio;
//!
//! // Activate the cartridge's rumble. This will continue until `stop()` is called.
//! gpio.start();
//!
//! // Deactivate the cartridge's rumble.
//! gpio.stop();
//! ```

#![no_std]
#![cfg_attr(test, no_main)]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(gba_test::runner))]
#![cfg_attr(test, reexport_test_harness_main = "test_harness")]

#[cfg(test)]
extern crate alloc;

mod splash_screen;

use core::{
    arch::asm,
    fmt,
    fmt::{Debug, Formatter},
};
use deranged::RangedUsize;

const DATA: *mut Data = 0x080000c4 as *mut Data;
const READ_WRITE: *mut ReadWrite = 0x080000c6 as *mut ReadWrite;
const ENABLE: *mut u16 = 0x080000c8 as *mut u16;
const DISPCNT: *mut u16 = 0x0400_0000 as *mut u16;
const BG0CNT: *mut u16 = 0x0400_0008 as *mut u16;
const MAP: *mut [u8; 844] = 0x0600_0000 as *mut [u8; 844];
const TILES: *mut [u8; 0x4000] = 0x0600_8000 as *mut [u8; 0x4000];
const PALETTE: *mut [u8; 128] = 0x0500_0000 as *mut [u8; 128];
const KEYINPUT: *mut u16 = 0x0400_0130 as *mut u16;
const SIODATA: *mut u32 = 0x0400_0120 as *mut u32;
const SIOCNT: *mut u16 = 0x0400_0128 as *mut u16;

static mut GAME_BOY_PLAYER_RUMBLE: GameBoyPlayerRumble = GameBoyPlayerRumble::Stop;
static mut GAME_BOY_PLAYER_SIO_STATE: GameBoyPlayerSioState = GameBoyPlayerSioState::Handshake {
    index: RangedUsize::new_static::<0>(),
};

#[derive(Debug)]
#[repr(u16)]
enum ReadWrite {
    Read = 0,
    Write = 8,
}

#[derive(Debug)]
#[repr(u16)]
enum Data {
    Enabled = 8,
    Disabled = 0,
}

/// Waits until a new v-blank interrupt occurs.
#[instruction_set(arm::t32)]
fn wait_for_vblank() {
    unsafe {
        asm! {
            "swi #0x05",
            out("r0") _,
            out("r1") _,
            out("r3") _,
            options(preserves_flags),
        }
    };
}

/// Reset VRAM.
///
/// This resets both VRAM and palette data. It is called after detecting the Game Boy Player.
#[inline]
#[instruction_set(arm::t32)]
fn reset_vram() {
    unsafe {
        asm! {
            "swi #0x01",
            in("r0") 12,
        }
    };
}

#[derive(Clone, Copy, Debug)]
enum GameBoyPlayerRumble {
    Stop = 0x4000_0004,
    HardStop = 0x4000_0015,
    Start = 0x4000_0026,
}

#[derive(Debug, Eq, PartialEq)]
enum GameBoyPlayerSioState {
    Handshake { index: RangedUsize<0, 3> },
    Magic { index: RangedUsize<1, 3> },
    SendData,
}

impl GameBoyPlayerSioState {
    const HANDSHAKE: [u16; 4] = [0x494e, 0x544e, 0x4e45, 0x4f44];
    const MAGIC_VALUES: [u32; 4] = [0xB0BB8002, 0x10000010, 0x20000013, 0x40000004];

    fn new() -> Self {
        Self::Handshake {
            index: RangedUsize::new_static::<0>(),
        }
    }

    fn get_handshake_key(index: RangedUsize<0, 3>) -> u16 {
        unsafe { *Self::HANDSHAKE.get_unchecked(index.get()) }
    }

    fn get_magic_values(index: RangedUsize<1, 3>) -> (u32, u32) {
        unsafe {
            (
                *Self::MAGIC_VALUES.get_unchecked(index.get().unchecked_sub(1)),
                *Self::MAGIC_VALUES.get_unchecked(index.get()),
            )
        }
    }
}

/// Handles SIO interrupts for every stage of the Game Boy Player communication process.
///
/// This function should be called within an interrupt handler when the SIO interrupt is triggered.
#[unsafe(link_section = ".iwram")]
pub fn game_boy_player_interrupt() {
    let input = unsafe { SIODATA.read_volatile() };

    unsafe {
        GAME_BOY_PLAYER_SIO_STATE = match GAME_BOY_PLAYER_SIO_STATE {
            GameBoyPlayerSioState::Handshake { index } => {
                let key = GameBoyPlayerSioState::get_handshake_key(index);
                if input as u16 == key {
                    if (input >> 16) as u16 == !key {
                        if let Some(new_index) = index.checked_add(1) {
                            let new_key = GameBoyPlayerSioState::get_handshake_key(new_index);
                            SIODATA.write_volatile(input >> 16 | ((new_key as u32) << 16));
                            SIOCNT.write_volatile(SIOCNT.read_volatile() | (1 << 7));
                            GameBoyPlayerSioState::Handshake { index: new_index }
                        } else {
                            SIODATA.write_volatile(0x8000B0BB);
                            SIOCNT.write_volatile(SIOCNT.read_volatile() | (1 << 7));
                            GameBoyPlayerSioState::Magic {
                                index: RangedUsize::new_static::<1>(),
                            }
                        }
                    } else {
                        SIODATA.write_volatile((!key) as u32 | ((key as u32) << 16));
                        SIOCNT.write_volatile(SIOCNT.read_volatile() | (1 << 7));
                        GameBoyPlayerSioState::Handshake { index }
                    }
                } else {
                    // Unexpected input value. Reset.
                    GameBoyPlayerSioState::new()
                }
            }
            GameBoyPlayerSioState::Magic { index } => {
                let (old_key, new_key) = GameBoyPlayerSioState::get_magic_values(index);
                if input == old_key {
                    SIODATA.write_volatile(new_key);
                    SIOCNT.write_volatile(SIOCNT.read_volatile() | (1 << 7));
                    if let Some(new_index) = index.checked_add(1) {
                        GameBoyPlayerSioState::Magic { index: new_index }
                    } else {
                        GameBoyPlayerSioState::SendData
                    }
                } else {
                    // Unexpected input value. Reset.
                    GameBoyPlayerSioState::new()
                }
            }
            GameBoyPlayerSioState::SendData => {
                if input == 0x30000003 {
                    SIODATA.write_volatile(GAME_BOY_PLAYER_RUMBLE as u32);
                    SIOCNT.write_volatile(SIOCNT.read_volatile() | (1 << 7));
                    // We stay in this state until the input changes.
                    GameBoyPlayerSioState::SendData
                } else {
                    GameBoyPlayerSioState::new()
                }
            }
        }
    }
}

#[derive(Eq, PartialEq)]
pub struct GameBoyPlayer {
    private: (),
}

impl GameBoyPlayer {
    pub fn detect() -> Option<Self> {
        // Draw the Game Boy Player splash screen.
        let old_dispcnt = unsafe { DISPCNT.read_volatile() };
        let old_bg0cnt = unsafe { BG0CNT.read_volatile() };
        unsafe {
            // Mode 0 with BG 0 enabled;
            DISPCNT.write_volatile(256);
            // Character Base Block 2, Screen Base Block 15.
            BG0CNT.write_volatile(0x88);

            TILES.write_volatile(splash_screen::TILES);
            MAP.write_volatile(splash_screen::MAP);
            PALETTE.write_volatile(splash_screen::PALETTE);
        }

        let mut detected = None;
        // Detect Game Boy Player.
        for _ in 0..125 {
            wait_for_vblank();
            // 0x030F indicates that all 4 directional values are pressed at once. This is not
            // possible on a normal console, so the game boy player uses this value to indicate
            // that its extra functionality has been unlocked. See GBATEK for more information.
            if unsafe { KEYINPUT.read_volatile() } == 0x030F {
                detected = Some(GameBoyPlayer { private: () });
            }
        }

        unsafe {
            DISPCNT.write_volatile(old_dispcnt);
            BG0CNT.write_volatile(old_bg0cnt);
        }
        reset_vram();

        detected
    }

    pub fn start(&self) {
        unsafe {
            GAME_BOY_PLAYER_RUMBLE = GameBoyPlayerRumble::Start;
        }
    }

    pub fn stop(&self) {
        unsafe {
            GAME_BOY_PLAYER_RUMBLE = GameBoyPlayerRumble::Stop;
        }
    }

    pub fn hard_stop(&self) {
        unsafe {
            GAME_BOY_PLAYER_RUMBLE = GameBoyPlayerRumble::HardStop;
        }
    }

    pub fn update(&self) {
        unsafe {
            SIOCNT.write_volatile(SIOCNT.read_volatile() | (1 << 7));
        }
    }
}

impl Debug for GameBoyPlayer {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("GameBoyPlayer")
    }
}

#[derive(Debug)]
pub struct Gpio;

impl Gpio {
    pub fn start(&self) {
        unsafe {
            ENABLE.write_volatile(1);
            READ_WRITE.write_volatile(ReadWrite::Write);
            DATA.write_volatile(Data::Enabled);
        }
    }

    pub fn stop(&self) {
        unsafe {
            DATA.write_volatile(Data::Disabled);
        }
    }
}

#[cfg(test)]
#[unsafe(no_mangle)]
pub fn main() {
    let _ = mgba_log::init();
    test_harness()
}

#[cfg(test)]
mod tests {
    #![allow(static_mut_refs)]

    use super::{GAME_BOY_PLAYER_RUMBLE, GameBoyPlayer};
    use crate::{
        GAME_BOY_PLAYER_SIO_STATE, GameBoyPlayerRumble, GameBoyPlayerSioState, SIODATA,
        game_boy_player_interrupt,
    };
    use alloc::format;
    use claims::{assert_matches, assert_none, assert_some_eq};
    use deranged::RangedUsize;
    use gba_test::test;

    const DISPSTAT: *mut u16 = 0x0400_0004 as *mut u16;
    const IME: *mut bool = 0x0400_0208 as *mut bool;
    const IE: *mut u16 = 0x0400_0200 as *mut u16;
    const RCNT: *mut u16 = 0x0400_0134 as *mut u16;
    const SIOCNT: *mut u16 = 0x0400_0128 as *mut u16;

    #[test]
    fn game_boy_player_debug() {
        assert_eq!(
            format!("{:?}", GameBoyPlayer { private: () }),
            "GameBoyPlayer"
        );
    }

    #[test]
    #[cfg_attr(
        not(game_boy_player),
        ignore = "This test should be run on a Game Boy Player (or emulator with Game Boy Player functionality). Pass `--cfg game_boy_player` to enable."
    )]
    fn game_boy_player_detect_successful() {
        unsafe {
            DISPSTAT.write_volatile(8);
            IE.write_volatile(1);
            IME.write(true);
        }
        assert_some_eq!(GameBoyPlayer::detect(), GameBoyPlayer { private: () });
    }

    #[test]
    #[cfg_attr(
        game_boy_player,
        ignore = "This test should be run on a console that is not a Game Boy Player (or emulator with Game Boy Player functionality disabled). Omit `--cfg game_boy_player` to enable."
    )]
    fn game_boy_player_detect_failure() {
        unsafe {
            DISPSTAT.write_volatile(8);
            IE.write_volatile(1);
            IME.write(true);
        }
        assert_none!(GameBoyPlayer::detect());
    }

    #[test]
    fn game_boy_player_start() {
        let game_boy_player = GameBoyPlayer { private: () };

        game_boy_player.start();

        assert_matches!(
            unsafe { GAME_BOY_PLAYER_RUMBLE },
            GameBoyPlayerRumble::Start
        );
    }

    #[test]
    fn game_boy_player_stop() {
        let game_boy_player = GameBoyPlayer { private: () };

        game_boy_player.stop();

        assert_matches!(unsafe { GAME_BOY_PLAYER_RUMBLE }, GameBoyPlayerRumble::Stop);
    }

    #[test]
    fn game_boy_player_hard_stop() {
        let game_boy_player = GameBoyPlayer { private: () };

        game_boy_player.hard_stop();

        assert_matches!(
            unsafe { GAME_BOY_PLAYER_RUMBLE },
            GameBoyPlayerRumble::HardStop
        );
    }

    #[test]
    fn game_boy_player_sio_state_get_handshake_key() {
        assert_eq!(
            GameBoyPlayerSioState::get_handshake_key(RangedUsize::new_static::<0>()),
            0x494e
        );
        assert_eq!(
            GameBoyPlayerSioState::get_handshake_key(RangedUsize::new_static::<1>()),
            0x544e
        );
        assert_eq!(
            GameBoyPlayerSioState::get_handshake_key(RangedUsize::new_static::<2>()),
            0x4e45
        );
        assert_eq!(
            GameBoyPlayerSioState::get_handshake_key(RangedUsize::new_static::<3>()),
            0x4f44
        );
    }

    #[test]
    fn game_boy_player_sio_state_get_magic_values() {
        assert_eq!(
            GameBoyPlayerSioState::get_magic_values(RangedUsize::new_static::<1>()),
            (0xB0BB8002, 0x10000010)
        );
        assert_eq!(
            GameBoyPlayerSioState::get_magic_values(RangedUsize::new_static::<2>()),
            (0x10000010, 0x20000013)
        );
        assert_eq!(
            GameBoyPlayerSioState::get_magic_values(RangedUsize::new_static::<3>()),
            (0x20000013, 0x40000004)
        );
    }

    #[test]
    fn game_boy_player_interrupt_handshake_partial_match_0() {
        unsafe {
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x0000494E);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x494EB6B1);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<0>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_partial_match_1() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Handshake {
                index: RangedUsize::new_static::<1>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0xB6B1544E);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x544EABB1);

            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<1>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_partial_match_2() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Handshake {
                index: RangedUsize::new_static::<2>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0xABB14E45);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x4E45B1BA);

            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<2>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_partial_match_3() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Handshake {
                index: RangedUsize::new_static::<3>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0xB1BA4F44);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x4F44B0BB);

            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<3>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_full_match_0() {
        unsafe {
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0xB6B1494E);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x544EB6B1);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<1>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_full_match_1() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Handshake {
                index: RangedUsize::new_static::<1>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0xABB1544E);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x4E45ABB1);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<2>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_full_match_2() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Handshake {
                index: RangedUsize::new_static::<2>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0xB1BA4E45);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x4F44B1BA);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<3>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_full_match_3() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Handshake {
                index: RangedUsize::new_static::<3>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0xB0BB4F44);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x8000B0BB);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Magic {
                    index: RangedUsize::new_static::<1>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_no_match_0() {
        unsafe {
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x12345678);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x12345678);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<0>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_no_match_1() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Handshake {
                index: RangedUsize::new_static::<1>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x12345678);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x12345678);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<0>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_no_match_2() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Handshake {
                index: RangedUsize::new_static::<2>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x12345678);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x12345678);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<0>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_handshake_no_match_3() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Handshake {
                index: RangedUsize::new_static::<3>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x12345678);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x12345678);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<0>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_magic_match_1() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Magic {
                index: RangedUsize::new_static::<1>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0xB0BB8002);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x10000010);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Magic {
                    index: RangedUsize::new_static::<2>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_magic_match_2() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Magic {
                index: RangedUsize::new_static::<2>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x10000010);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x20000013);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Magic {
                    index: RangedUsize::new_static::<3>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_magic_match_3() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Magic {
                index: RangedUsize::new_static::<3>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x20000013);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x40000004);
            assert_eq!(GAME_BOY_PLAYER_SIO_STATE, GameBoyPlayerSioState::SendData);
        }
    }

    #[test]
    fn game_boy_player_interrupt_magic_no_match_1() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Magic {
                index: RangedUsize::new_static::<1>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x12345678);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x12345678);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<0>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_magic_no_match_2() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Magic {
                index: RangedUsize::new_static::<2>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x12345678);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x12345678);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<0>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_magic_no_match_3() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::Magic {
                index: RangedUsize::new_static::<3>(),
            };
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x12345678);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x12345678);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<0>()
                }
            );
        }
    }

    #[test]
    fn game_boy_player_interrupt_send_data_start() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::SendData;
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x30000003);
            GAME_BOY_PLAYER_RUMBLE = GameBoyPlayerRumble::Start;
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), GameBoyPlayerRumble::Start as u32);
            assert_eq!(GAME_BOY_PLAYER_SIO_STATE, GameBoyPlayerSioState::SendData);
        }
    }

    #[test]
    fn game_boy_player_interrupt_send_data_stop() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::SendData;
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x30000003);
            GAME_BOY_PLAYER_RUMBLE = GameBoyPlayerRumble::Stop;
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), GameBoyPlayerRumble::Stop as u32);
            assert_eq!(GAME_BOY_PLAYER_SIO_STATE, GameBoyPlayerSioState::SendData);
        }
    }

    #[test]
    fn game_boy_player_interrupt_send_data_hard_stop() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::SendData;
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x30000003);
            GAME_BOY_PLAYER_RUMBLE = GameBoyPlayerRumble::HardStop;
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(
                SIODATA.read_volatile(),
                GameBoyPlayerRumble::HardStop as u32
            );
            assert_eq!(GAME_BOY_PLAYER_SIO_STATE, GameBoyPlayerSioState::SendData);
        }
    }

    #[test]
    fn game_boy_player_interrupt_send_data_no_match() {
        unsafe {
            GAME_BOY_PLAYER_SIO_STATE = GameBoyPlayerSioState::SendData;
            RCNT.write_volatile(0);
            SIOCNT.write_volatile(0x4000 | 0x1000 | 8);
            SIODATA.write_volatile(0x12345678);
        }

        game_boy_player_interrupt();

        unsafe {
            assert_eq!(SIODATA.read_volatile(), 0x12345678);
            assert_eq!(
                GAME_BOY_PLAYER_SIO_STATE,
                GameBoyPlayerSioState::Handshake {
                    index: RangedUsize::new_static::<0>()
                }
            );
        }
    }
}
