# gba_rumble

[![GitHub Workflow Status](https://img.shields.io/github/check-runs/Anders429/gba_rumble/master?label=tests)](https://github.com/Anders429/gba_rumble/actions?query=branch%3Amaster)
[![crates.io](https://img.shields.io/crates/v/gba_rumble)](https://crates.io/crates/gba_rumble)
[![docs.rs](https://docs.rs/gba_rumble/badge.svg)](https://docs.rs/gba_rumble)
[![License](https://img.shields.io/crates/l/gba_rumble)](#license)

Library for enabling rumble functionality on the Game Boy Advance.

This crate supports rumble through both the cartridge itself using general purpose I/O (GPIO) and the Game Boy Player's rumble functionality. Functionality is provided for detecting available rumble features and using them fully.

The library is designed to be usable regardless of what other GBA development libraries may be in use. It is usable with popular libraries like [`gba`](https://crates.io/crates/gba) and [`agb`](https://crates.io/crates/agb).

## Usage
There are two ways to use this library: by using a cartridge's built-in rumble through [`Gpio`](https://docs.rs/gba_rumble/latest/gba_rumble/struct.Gpio.html) and by using the Game Boy Player's rumble functionality through [`GameBoyPlayer`](https://docs.rs/gba_rumble/latest/gba_rumble/struct.GameBoyPlayer.html).

### Cartridge (GPIO) Rumble
To use a cartridge's built-in rumble through general purpose I/O (GPIO), use the [`Gpio`](https://docs.rs/gba_rumble/latest/gba_rumble/struct.Gpio.html) struct.

``` rust
let gpio = gba_rumble::Gpio;

// Activate the cartridge's rumble. This will continue until `stop()` is called.
gpio.start();

// Deactivate the cartridge's rumble.
gpio.stop();
```

### Game Boy Player
To use the Game Boy Player's rubmle functionality, detect the Game Boy Player by calling [`GameBoyPlayer::detect()`](https://docs.rs/gba_rumble/latest/gba_rumble/struct.GameBoyPlayer.html#method.detect) at the beginning of your program.

``` rust
if let Some(game_boy_player) = gba_rumble::GameBoyPlayer::detect() {    
    // When a Game Boy Player is detected, you can interact with it through the returned
    // `GameBoyPlayer` object.
    //
    // To actually use it, you must also call `game_boy_player_interrupt()` when a serial
    // interrupt is received. This will be specific to your own code and any frameworks you may
    // be using.

    // Update the serial connection once a frame.
    game_boy_player.update();
    // Activate rumble in the controller. This will continue until `stop()` or `hard_stop()`
    // is called.
    game_boy_player.start();

    // Deactivate rumble in the controller.
    game_boy_player.stop();
}
```

## License
This project is licensed under either of

* Apache License, Version 2.0
([LICENSE-APACHE](https://github.com/Anders429/gba_rumble/blob/HEAD/LICENSE-APACHE) or
http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
([LICENSE-MIT](https://github.com/Anders429/gba_rumble/blob/HEAD/LICENSE-MIT) or
http://opensource.org/licenses/MIT)

at your option.

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
