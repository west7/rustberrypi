//! Snake Game
//! 
//! Este é um jogo da cobrinha implementado para rodar em um microcontrolador 
//! RP2040, utilizando a placa BitDogLab.
//! 
//! Os principais recursos usados são: o canal PIO para controlar LEDs ws2812 e o 
//! canal ADC para ler os valores do joystick. 
//! 
//! 
//! Autor: Guilherme Westphall

#![no_std]
#![no_main]

use panic_halt as _;

use embedded_hal::digital::v2::OutputPin;
use cortex_m::asm;
use embedded_hal::adc::OneShot;
use rp_pico::entry;
use rp_pico::hal::gpio::Pin;
use rp_pico::hal::pio::PIOExt;
use rp_pico::hal::timer::Timer;
use rp_pico::hal::{self, adc::Adc, clocks::Clock, pac, watchdog::Watchdog};
use rp2040_hal::gpio::bank0::Gpio13;
use rp2040_hal::gpio::{Output, PushPull};
use smart_leds::{RGB8, SmartLedsWrite, brightness};
use ws2812_pio::Ws2812;

/// #[link_section = ".boot2"]
/// #[used]
/// pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

const BOARD_SIZE: usize = 5;
const NUM_LEDS: usize = BOARD_SIZE * BOARD_SIZE;

/// APPLE
pub struct Apple {
    pub x: usize,
    pub y: usize,
}

impl Apple {
    pub fn new(start: (usize, usize)) -> Self {
        Self {
            x: start.0,
            y: start.1,
        }
    }

    pub fn spawn_apple(&mut self, snake: &Snake, seed: u64) -> (usize, usize) {
        let mut attempt = seed;
        loop {
            let row = (attempt % BOARD_SIZE as u64) as usize;
            let col = ((attempt / BOARD_SIZE as u64) % BOARD_SIZE as u64) as usize;

            if !snake.occupies(row, col) {
                self.x = row;
                self.y = col;
                return (row, col);
            }
            attempt += 27;
        }
    }
}

/// SNAKE
#[derive(Clone, Copy)]
pub enum Direction {
    UP,
    DOWN,
    LEFT,
    RIGHT,
}

pub struct Snake {
    pub body: [(usize, usize); NUM_LEDS],
    pub length: usize,
    pub direction: Direction,
}

impl Snake {
    pub fn new(direction: Direction, start: (usize, usize)) -> Self {
        let mut body = [(0, 0); NUM_LEDS];
        body[0] = start;
        Self {
            body,
            length: 1,
            direction,
        }
    }

    pub fn occupies(&self, row: usize, col: usize) -> bool {
        for i in 0..self.length {
            if self.body[i] == (row, col) {
                return true;
            }
        }
        false
    }

    fn direction_to_delta(&self) -> (isize, isize) {
        match self.direction {
            Direction::UP => (-1, 0),
            Direction::DOWN => (1, 0),
            Direction::LEFT => (0, -1),
            Direction::RIGHT => (0, 1),
        }
    }

    // Move a cobra: desloca os segmentos e tenta mover a cabeça.
    // Se a nova posição estiver fora do tabuleiro, ignora o movimento.
    pub fn move_snake(&mut self) -> bool {
        // Desloca os segmentos (o último é descartado)
        for i in (1..self.length).rev() {
            self.body[i] = self.body[i - 1];
        }
        let (head_row, head_col) = self.body[0];
        let (dr, dc) = self.direction_to_delta();
        let new_row = head_row as isize + dr;
        let new_col = head_col as isize + dc;

        if new_row < 0
            || new_row >= BOARD_SIZE as isize
            || new_col < 0
            || new_col >= BOARD_SIZE as isize
        {
            return false;
        }

        let new_pos = (new_row as usize, new_col as usize);

        // Verifica colisão com o próprio corpo
        if self.body[..self.length].contains(&new_pos) {
            // Game over: colidiu consigo mesma
            return false;
        }

        self.body[0] = (new_row as usize, new_col as usize);
        true
    }

    pub fn check_colision(&self) -> bool {
        let (head_row, head_col) = self.body[0];

        if head_row >= BOARD_SIZE || head_col >= BOARD_SIZE {
            return true;
        }

        for i in 1..self.length {
            if self.body[i] == (head_row, head_col) {
                return true;
            }
        }

        false
    }
}

/// GAME LOGIC
#[derive(Clone, Copy)]
enum GameObjT {
    APPLE(RGB8),
    SNAKE(RGB8),
    EMPTY(RGB8),
}

impl GameObjT {
    fn color(&self) -> RGB8 {
        match self {
            GameObjT::APPLE(color) => *color,
            GameObjT::SNAKE(color) => *color,
            GameObjT::EMPTY(color) => *color,
        }
    }
}

const APPLE_COLOR: RGB8 = RGB8 { r: 255, g: 0, b: 0 };
const SNAKE_COLOR: RGB8 = RGB8 { r: 0, g: 128, b: 0 };
const EMPTY_COLOR: RGB8 = RGB8 { r: 0, g: 0, b: 0 };

/// Função auxiliar para finalizar o jogo.
fn game_over(led: &mut Pin<Gpio13, Output<PushPull>>) {
    led.set_high().unwrap();
    loop {}
}

/// Função auxiliar para ler o joystick e retornar a direção correspondente.
fn read_joystick(x: u16, y: u16, current: Direction) -> Direction {
    const DEADZONE: u16 = 50;
    const ADC_MAX: u16 = 4096; // (0..4095)

    if y >= ADC_MAX - DEADZONE {
        Direction::UP
    } else if y <= DEADZONE {
        Direction::DOWN
    } else if x >= ADC_MAX - DEADZONE {
        Direction::RIGHT
    } else if x <= DEADZONE {
        Direction::LEFT
    } else {
        current
    }
}

/// Função auxiliar para mapear as coordenadas da matriz para o
/// vetor de leds
fn pos_to_index(row: usize, col: usize) -> usize {
    let r = BOARD_SIZE - 1 - row;
    if r % 2 == 0 {
        // Linha par a partir da base: mapeia da direita para a esquerda.
        r * BOARD_SIZE + (BOARD_SIZE - 1 - col)
    } else {
        // Linha ímpar a partir da base: mapeia da esquerda para a direita.
        r * BOARD_SIZE + col
    }
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let _core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = hal::Sio::new(pac.SIO);
    let pins = rp2040_hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let mut red_pin = pins.gpio13.into_push_pull_output();

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    // Configura o PIO para controlar os LEDs WS2812
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);

    let mut ws2812 = Ws2812::new(
        pins.gpio7.into_mode(),
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
        timer.count_down(),
    );

    let mut adc = Adc::new(pac.ADC, &mut pac.RESETS);

    let mut adc_pin_y = pins.gpio26.into_floating_input();
    let mut adc_pin_x = pins.gpio27.into_floating_input();

    let mut game_board = [GameObjT::EMPTY(EMPTY_COLOR); NUM_LEDS];
    let strip_brightness = 24u8;

    let mut apple = Apple::new((2, 2));
    let mut snake = Snake::new(Direction::UP, (4, 0));

    let mut last_update = timer.get_counter().ticks();
    let move_interval = (clocks.peripheral_clock.freq().to_Hz() / 160) as u64;

    // Inicializa o game_board com a maçã e a cobra
    game_board[pos_to_index(apple.y, apple.x)] = GameObjT::APPLE(APPLE_COLOR);
    game_board[pos_to_index(snake.body[0].0, snake.body[0].1)] = GameObjT::SNAKE(SNAKE_COLOR);

    loop {
        // Verifica se a cobra colidiu 
        if snake.check_colision() {
            game_over(&mut red_pin);
        }

        let adc_raw_x: u16 = adc.read(&mut adc_pin_x).unwrap();
        let adc_raw_y: u16 = adc.read(&mut adc_pin_y).unwrap();

        snake.direction = read_joystick(adc_raw_x, adc_raw_y, snake.direction);

        let current_time = timer.get_counter().ticks();

        if current_time - last_update >= move_interval {
            last_update = current_time;

            let tail = snake.body[snake.length - 1];

            // Move a cobra e verifica se houve colisão
            if !snake.move_snake() {
                game_over(&mut red_pin);
            }

            if snake.body[0] == (apple.y, apple.x) {
                snake.body[snake.length] = tail;
                snake.length += 1;

                let (new_row, new_col) = apple.spawn_apple(&snake, current_time);
                apple.x = new_row;
                apple.y = new_col;
            }

            // Limpa o game_board
            for slot in game_board.iter_mut() {
                *slot = GameObjT::EMPTY(EMPTY_COLOR);
            }

            // Atualiza a posição da maçã
            game_board[pos_to_index(apple.y, apple.x)] = GameObjT::APPLE(APPLE_COLOR);

            // Atualiza a posição da cobra: para cada segmento, calcula o índice e coloca a cor correspondente.
            for i in 0..snake.length {
                let (row, col) = snake.body[i];
                if row < BOARD_SIZE && col < BOARD_SIZE {
                    game_board[pos_to_index(row, col)] = GameObjT::SNAKE(SNAKE_COLOR);
                }
            }
        }

        // Atualiza os LEDs
        ws2812
            .write(brightness(
                game_board.iter().map(|obj| obj.color()),
                strip_brightness,
            ))
            .ok();

        asm::delay(500_000);
    }
}

