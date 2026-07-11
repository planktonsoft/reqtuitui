# reqtuitui

A beautiful and efficient Terminal UI (TUI) for making HTTP requests, built with Rust and Ratatui.

## Features

- **Interactive TUI:** Fast, keyboard-driven interface for crafting and sending HTTP requests.
- **cURL Support:** Import and parse cURL commands directly into the UI.
- **JSON Support:** Built-in JSON formatting and request bodies.
- **State Persistence:** Automatically saves your session state and requests history.
- **Persistent Global Cookie Jar:** A central, state-saved store for cookies that automatically injects them into outgoing requests and extracts new cookies from server responses.

## Installation

### Using Homebrew (macOS/Linux)

You can install `reqtuitui` via Homebrew using our custom tap:

```bash
brew install planktonsoft/homebrew-reqtuitui/reqtuitui
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

### Keyboard Shortcuts

`reqtuitui` is heavily keyboard-driven. Here are the primary shortcuts to navigate and use the application efficiently:

#### Global Shortcuts
- **`Tab`**: Switch focus between panes (Sidebar -> URL Bar -> Headers -> Body)
- **`Ctrl + N`**: Create a new request
- **`Ctrl + F`**: Create a new folder
- **`Ctrl + S`**: Save the current request/collection (or save within the Cookie Jar popup)
- **`Ctrl + E`**: Open Environment variables popup
- **`Ctrl + G`**: Open the Global Cookie Jar popup editor
- **`Ctrl + O`**: Import cURL command
- **`Ctrl + Y`**: Cycle HTTP method (GET, POST, PUT, DELETE, PATCH)
- **`Ctrl + R`**: Rename selected folder or request
- **`PageUp` / `PageDown`**: Scroll response view up/down (also `Ctrl+U`/`Ctrl+D`)
- **`Esc`**: Close popups or cancel actions

#### Sidebar (List) Controls
When the sidebar is focused:
- **`Up` / `k`**: Move selection up
- **`Down` / `j`**: Move selection down
- **`Enter`**: Toggle folder expansion or execute the selected request
- **`e`**: Edit the URL of the selected request
- **`Delete` / `Backspace`**: Delete the selected item
- **`q`**: Quit application

### Cookie Jar Management

`reqtuitui` features a built-in global cookie jar that helps manage session states seamlessly:

1. **Viewing/Editing Cookies:** Press **`Ctrl + G`** at any time to open the Global Cookie Jar. Type your cookies in the format `key=value` (one pair per line).
2. **Saving:** Press **`Ctrl + S`** to save your changes and close the editor. The cookies are stored in a persistent local database.
3. **Automatic Injection:** If cookies exist in the jar, `reqtuitui` automatically formats and injects them as a `Cookie` header (e.g., `Cookie: token=test; session=1234`) on every outgoing request.
4. **Auto-Extraction:** When a server responds with `Set-Cookie` headers, `reqtuitui` automatically parses them, updates the global cookie jar, and persists them database-wide.

## Contributing

Contributions are welcome! Feel free to open an issue or submit a pull request.

## License

This project is licensed under the terms of the MIT license.
