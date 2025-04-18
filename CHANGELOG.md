# Changelog

## 0.1.2 - 2025-04-17
### Added
- `Gpio` now implements `Clone`, `Copy`, `PartialEq`, and `Eq`.
- `GameBoyPlayer` now implements `Clone` and `Copy`.

## 0.1.1 - 2025-04-16
### Fixed
- Fixed docs.rs configuration to allow it to build.


## 0.1.0 - 2025-04-16
### Added
- `Gpio` struct to interact with a cartridge's rumble.
- `GameBoyPlayer` struct to interact with the Game Boy Player's rumble, including detection.
- `game_boy_player_interrupt()` function to handle serial communication with the Game Boy Player from an interrupt handler.
