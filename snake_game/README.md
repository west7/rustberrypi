# Snake Game

> Simple snake game made with rust at the Raspberry Pi Pico W.

## Hardware

- Raspberry Pi Pico W
- 5x5 LED Matrix
- Joystick
- Red LED
  
## Software

- [Rust](https://www.rust-lang.org/)
- Crates
  - cortex-m
  - cortex-m-rt
  - embedded-hal
  - panic-halt
  - rp-pico
  - rp2040-hal
  - smart-leds
  - ws2812-pio

## How to run

Inside the directory:
```
cargo run --bin snake_game
```

## How to play

- Use the joystick to move the snake
- Eat the red led to grow
- Don't hit the walls or yourself
