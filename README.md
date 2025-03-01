# Rustberry Pi

> Learning embedded rust on a raspberry pi

## Setup

1. Install rust from the oficial site [rustup](https://rustup.rs/)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Check if it's all good

```bash
rustc --version
cargo --version
```

3. Install Compiling Tools 

```bash
cargo install elf2uf2-rs # for flashing the raspberry pi
rustup target add thumbv7em-none-eabihf # for cross compiling
```
4. Start a new project

```bash
cargo new --bin project
cd project
```

5. Make sure your .cargo/config file looks like this

```toml
[target.thumbv6m-none-eabi]
runner = "elf2uf2-rs -d"
```

6. If not, create it

```bash
mkdir .cargo
touch .cargo/config
```

7. Add your dependencies to the Cargo.toml file using `cargo add <dependency>`. It should look like this

```toml
[dependencies]
rp2040-hal = "0.7.0"
panic-halt = "0.2.0"
embedded-hal = { version = "0.2.5", features = ["unproven"] }
cortex-m = "0.7.2"
cortex-m-rt = "0.7"
rp2040-boot2 = "0.2.1"
```