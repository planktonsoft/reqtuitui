# reqtuitui

A beautiful and efficient Terminal UI (TUI) for making HTTP requests, built with Rust and Ratatui.

## Features

- **Interactive TUI:** Fast, keyboard-driven interface for crafting and sending HTTP requests.
- **cURL Support:** Import and parse cURL commands directly into the UI.
- **JSON Support:** Built-in JSON formatting and request bodies.
- **State Persistence:** Automatically saves your session state and requests history.

## Installation

### Using Homebrew (macOS/Linux)

You can install `reqtuitui` via Homebrew using our custom tap:

```bash
brew install planktonsoft/tap/reqtuitui
```

### Using Cargo

If you have Rust installed, you can easily install `reqtuitui` using `cargo`:

```bash
cargo install reqtuitui
```

### From Source

```bash
git clone https://github.com/planktonsoft/reqtuitui.git
cd reqtuitui
cargo build --release
```
The binary will be located in `target/release/reqtuitui`.

## Usage

Simply run the command in your terminal:

```bash
reqtuitui
```

## Contributing

Contributions are welcome! Feel free to open an issue or submit a pull request.

## License

This project is licensed under the terms of the MIT license.
