use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::formatter::format_json_response;
use crate::models::ApiResponse;

pub struct ResponseViewer {
    pub scroll: u16,
}

impl ResponseViewer {
    pub fn new() -> Self {
        Self { scroll: 0 }
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll = self.scroll.saturating_add(amount);
    }

    pub fn reset_scroll(&mut self) {
        self.scroll = 0;
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        is_loading: bool,
        active_response: Option<&ApiResponse>,
    ) {
        let response_content = if is_loading {
            Text::raw("Sending request...")
        } else if let Some(resp) = active_response {
            // Run the body through our syntax highlighter
            let mut text = format_json_response(&resp.body);

            // Prepend the status code and time to the top of the formatted text
            let meta_line = Line::from(format!(
                "Status: {}\nTime: {}ms\n",
                resp.status_code, resp.duration_ms
            ));
            text.lines.insert(0, meta_line);
            text.lines.insert(1, Line::raw("")); // Blank line for spacing

            text
        } else {
            Text::raw("Awaiting request...")
        };

        let content_len = response_content.lines.len();

        let response_block = Paragraph::new(response_content)
            .block(
                Block::default()
                    .title(" Response (PageUp/PageDown to scroll) ")
                    .borders(Borders::ALL),
            )
            .scroll((self.scroll, 0));
        f.render_widget(response_block, area);

        let mut scrollbar_state = ScrollbarState::default()
            .content_length(content_len)
            .position(self.scroll as usize);

        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼")),
            area,
            &mut scrollbar_state,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use std::collections::HashMap;

    #[test]
    fn test_new() {
        let viewer = ResponseViewer::new();
        assert_eq!(viewer.scroll, 0);
    }

    #[test]
    fn test_scroll() {
        let mut viewer = ResponseViewer::new();
        viewer.scroll_down(5);
        assert_eq!(viewer.scroll, 5);
        viewer.scroll_up(2);
        assert_eq!(viewer.scroll, 3);
        viewer.scroll_up(10);
        assert_eq!(viewer.scroll, 0);

        viewer.scroll_down(10);
        viewer.reset_scroll();
        assert_eq!(viewer.scroll, 0);
    }

    #[test]
    fn test_render_loading() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let viewer = ResponseViewer::new();

        terminal
            .draw(|f| {
                viewer.render(f, f.area(), true, None);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut found = false;
        for y in 0..buffer.area.height {
            let mut line = String::new();
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                line.push_str(cell.symbol());
            }
            if line.contains("Sending request...") {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "Could not find 'Sending request...' in the render output"
        );
    }

    #[test]
    fn test_render_awaiting() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let viewer = ResponseViewer::new();

        terminal
            .draw(|f| {
                viewer.render(f, f.area(), false, None);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut found = false;
        for y in 0..buffer.area.height {
            let mut line = String::new();
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                line.push_str(cell.symbol());
            }
            if line.contains("Awaiting request...") {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "Could not find 'Awaiting request...' in the render output"
        );
    }

    #[test]
    fn test_render_response() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let viewer = ResponseViewer::new();

        let response = ApiResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: r#"{"foo": "bar"}"#.to_string(),
            duration_ms: 42,
            new_cookies: HashMap::new(),
        };

        terminal
            .draw(|f| {
                viewer.render(f, f.area(), false, Some(&response));
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        let mut lines = Vec::new();
        for y in 0..buffer.area.height {
            let mut line = String::new();
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                line.push_str(cell.symbol());
            }
            lines.push(line);
        }

        // Verify status and time are rendered
        let has_status_time = lines
            .iter()
            .any(|l| l.contains("Status: 200") && l.contains("Time: 42ms"));
        assert!(
            has_status_time,
            "Could not find status code and duration metadata"
        );

        // Verify JSON body content is rendered
        let has_foo = lines.iter().any(|l| l.contains("foo"));
        let has_bar = lines.iter().any(|l| l.contains("bar"));
        assert!(has_foo, "Could not find 'foo' in the render output");
        assert!(has_bar, "Could not find 'bar' in the render output");
    }
}
