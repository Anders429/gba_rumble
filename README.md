# gba_rumble

Library for enabling rumble functionality on the Game Boy Advance.

This crate supports rumble through both the cartridge itself using general purpose I/O (GPIO) and the Game Boy Player's rumble functionality. Functionality is provided for detecting available rumble features and using them fully.

The library is designed to be usable regardless of what other GBA development libraries may be in use. It is usable with popular libraries like [`gba`](https://docs.rs/gba/latest/gba/) and [`agb`](https://docs.rs/agb/latest/agb/index.html).

## Usage
There are two ways to use this library: by using a cartridge's built-in rumble through [`Gpio`] and by using the Game Boy Player's rumble functionality through [`GameBoyPlayer`].

### Cartridge (GPIO) Rumble
To use a cartridge's built-in rumble through general purpose I/O (GPIO), use the [`Gpio`] struct.

``` rust
let gpio = gba_rumble::Gpio;

// Activate the cartridge's rumble. This will continue until `stop()` is called.
gpio.start();

// Deactivate the cartridge's rumble.
gpio.stop();
```

### Game Boy Player
To use the Game Boy Player's rubmle functionality, detect the Game Boy Player by calling [`GameBoyPlayer::detect()`] at the beginning of your program.

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
