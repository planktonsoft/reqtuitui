use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Modifier, Style};
use tui_textarea::{CursorMove, TextArea};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum VimMode {
    Normal,
    Insert,
}

/// Returns `true` if the key was absorbed by the Vim engine,
/// or `false` if the standard input handler should process it.
pub fn handle_vim_key(textarea: &mut TextArea, key: KeyEvent, vim_mode: &mut VimMode) -> bool {
    match vim_mode {
        VimMode::Normal => {
            match key.code {
                // --- INSERT MODE TRIGGERS ---
                KeyCode::Char('i') => *vim_mode = VimMode::Insert,
                KeyCode::Char('I') => {
                    textarea.move_cursor(CursorMove::Head);
                    *vim_mode = VimMode::Insert;
                }
                KeyCode::Char('a') => {
                    textarea.move_cursor(CursorMove::Forward);
                    *vim_mode = VimMode::Insert;
                }
                KeyCode::Char('A') => {
                    textarea.move_cursor(CursorMove::End);
                    *vim_mode = VimMode::Insert;
                }
                KeyCode::Char('o') => {
                    textarea.move_cursor(CursorMove::End);
                    textarea.insert_newline();
                    *vim_mode = VimMode::Insert;
                }
                KeyCode::Char('O') => {
                    textarea.move_cursor(CursorMove::Head);
                    textarea.insert_newline();
                    textarea.move_cursor(CursorMove::Up);
                    *vim_mode = VimMode::Insert;
                }

                // --- NAVIGATION ---
                KeyCode::Char('h') => textarea.move_cursor(CursorMove::Back),
                KeyCode::Char('j') => textarea.move_cursor(CursorMove::Down),
                KeyCode::Char('k') => textarea.move_cursor(CursorMove::Up),
                KeyCode::Char('l') => textarea.move_cursor(CursorMove::Forward),
                KeyCode::Char('w') => textarea.move_cursor(CursorMove::WordForward),
                KeyCode::Char('b') => textarea.move_cursor(CursorMove::WordBack),
                KeyCode::Char('0') => textarea.move_cursor(CursorMove::Head),
                KeyCode::Char('$') => textarea.move_cursor(CursorMove::End),

                // --- EDITING ---
                KeyCode::Char('x') => {
                    textarea.delete_next_char();
                }
                KeyCode::Char('u') => {
                    textarea.undo();
                }
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    textarea.redo();
                }
                _ => {} // Ignore unmapped keys in Normal mode
            }
            true
        }
        VimMode::Insert => {
            if key.code == KeyCode::Esc {
                *vim_mode = VimMode::Normal;
                textarea.move_cursor(CursorMove::Back); // Vim steps back 1 char on Esc
                true
            } else {
                false // Let the user type normally!
            }
        }
    }
}

/// Helper to update the cursor block style based on Vim mode
pub fn apply_vim_style(textarea: &mut TextArea, is_active: bool, mode: VimMode) {
    if is_active {
        if mode == VimMode::Normal {
            // Solid block cursor for Normal mode
            textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        } else {
            // Line cursor for Insert mode
            textarea.set_cursor_style(Style::default().add_modifier(Modifier::UNDERLINED));
        }
    } else {
        // Default block for non-vim
        textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    }
}
