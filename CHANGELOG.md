# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] - 2026-07-17

### Added
- Asynchronous, per-request state tracking for API calls.
- Optional Vim emulation mode for text editors with toggleable `F4` keybinding.

### Changed
- Improved curl import parsing with newline handling and flag aliases.

## [0.1.3] - 2026-07-14

### Added
- Word wrapping in response viewer for better readability.

## [0.1.2] - 2026-07-12

### Added
- Manual body type cycling shortcut `Ctrl+B` (None, RawJson, RawText, FormData).
- Automatic body type transitions when cycling HTTP methods (defaulting to `RawJson` for `POST`/`PUT`/`PATCH` and `None` for others).
- Automatic default `Content-Type` header injection based on request body type.
- Zen Mode for Headers and Body editors (via `F2` key) to zoom/expand the text editor for focused editing.
- Automatic JSON formatting for request body in the Body Editor (via `F3` key).

### Fixed
- Fixed client request engine incorrectly defaulting `PUT`, `DELETE`, `PATCH`, `OPTIONS`, and `HEAD` methods to `GET`.
- Fixed potential socket/request hangs by automatically stripping manual/stale `Content-Length` headers from outgoing requests.

## [0.1.1] - 2026-07-11

### Added
- Persistent global cookie jar with management UI, automatic header injection, and response-based cookie extraction persistence.
- Local mock server for request inspection and debugging (`test_server.py`).
- Detailed documentation for the Cookie Jar and shortcut mappings in the project README.

## [0.1.0] - 2026-07-10

### Added
- Import cURL command via `Ctrl+O` popup.
- Folder creation logic and sibling insertion in the sidebar tree.
- Configured HTTP client to accept invalid SSL certificates.
- Configured automated GitHub Actions release workflow via `cargo-dist` with Homebrew tap publishing.
- Keyboard shortcut documentation to the project README.
