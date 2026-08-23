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
    let modifier = match (is_active, mode) {
        (true, VimMode::Insert) => Modifier::UNDERLINED,
        _ => Modifier::REVERSED,
    };
    textarea.set_cursor_style(Style::default().add_modifier(modifier));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_vim_style() {
        let mut textarea = TextArea::default();

        apply_vim_style(&mut textarea, true, VimMode::Normal);
        assert_eq!(
            textarea.cursor_style(),
            Style::default().add_modifier(Modifier::REVERSED)
        );

        apply_vim_style(&mut textarea, true, VimMode::Insert);
        assert_eq!(
            textarea.cursor_style(),
            Style::default().add_modifier(Modifier::UNDERLINED)
        );

        apply_vim_style(&mut textarea, false, VimMode::Normal);
        assert_eq!(
            textarea.cursor_style(),
            Style::default().add_modifier(Modifier::REVERSED)
        );

        apply_vim_style(&mut textarea, false, VimMode::Insert);
        assert_eq!(
            textarea.cursor_style(),
            Style::default().add_modifier(Modifier::REVERSED)
        );
    }

    #[test]
    fn test_handle_vim_key_normal_mode_insert_triggers() {
        let mut textarea = TextArea::default();
        let mut mode = VimMode::Normal;

        // 'i'
        let handled = handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
            &mut mode,
        );
        assert!(handled);
        assert_eq!(mode, VimMode::Insert);

        // Reset to normal via Esc
        let handled = handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            &mut mode,
        );
        assert!(handled);
        assert_eq!(mode, VimMode::Normal);

        // 'a'
        handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            &mut mode,
        );
        assert_eq!(mode, VimMode::Insert);

        mode = VimMode::Normal;
        // 'I'
        handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT),
            &mut mode,
        );
        assert_eq!(mode, VimMode::Insert);

        mode = VimMode::Normal;
        // 'A'
        handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT),
            &mut mode,
        );
        assert_eq!(mode, VimMode::Insert);

        mode = VimMode::Normal;
        // 'o'
        handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE),
            &mut mode,
        );
        assert_eq!(mode, VimMode::Insert);

        mode = VimMode::Normal;
        // 'O'
        handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('O'), KeyModifiers::SHIFT),
            &mut mode,
        );
        assert_eq!(mode, VimMode::Insert);
    }

    #[test]
    fn test_handle_vim_key_navigation_and_editing() {
        let mut textarea = TextArea::default();
        textarea.insert_str("hello world");
        let mut mode = VimMode::Normal;

        // Navigation keys
        let nav_keys = ['h', 'j', 'k', 'l', 'w', 'b', '0', '$'];
        for k in nav_keys {
            let handled = handle_vim_key(
                &mut textarea,
                KeyEvent::new(KeyCode::Char(k), KeyModifiers::NONE),
                &mut mode,
            );
            assert!(handled);
            assert_eq!(mode, VimMode::Normal);
        }

        // Delete char 'x'
        let handled = handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            &mut mode,
        );
        assert!(handled);

        // Undo 'u'
        let handled = handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE),
            &mut mode,
        );
        assert!(handled);

        // Redo 'Ctrl+r'
        let handled = handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            &mut mode,
        );
        assert!(handled);

        // Unmapped key in normal mode
        let handled = handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
            &mut mode,
        );
        assert!(handled);
    }

    #[test]
    fn test_handle_vim_key_insert_mode_typing() {
        let mut textarea = TextArea::default();
        let mut mode = VimMode::Insert;

        // In insert mode, normal typing is not absorbed by vim engine
        let handled = handle_vim_key(
            &mut textarea,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            &mut mode,
        );
        assert!(!handled);
        assert_eq!(mode, VimMode::Insert);
    }
}
