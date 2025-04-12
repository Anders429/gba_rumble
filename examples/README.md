# Examples

Here are some examples showing how to use `gba_rumble` with commonly used Rust GBA dev frameworks. There are examples using both the [`gba`](https://crates.io/crates/gba) and [`agb`](https://crates.io/crates/agb) crates. Note the versions being used, as some APIs used may change as newer versions of these crates are released.

Both examples have the same behavior: they attempt to detect the Game Boy Player, and then trigger rumble based on inputs:
- A button: start rumble
- B button: stop rumble
- Start button: hard stop rumble (only supported on Game Boy Player)
If the Game Boy Player is not detected, the cartridge's rumble will be used instead (through GPIO).

Nothing is displayed by these examples. If you see a white screen, it's working correctly.
