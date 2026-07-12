use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::models::ApiResponse;
use crate::formatter::format_json_response;

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

    pub fn render(&self, f: &mut Frame, area: Rect, is_loading: bool, active_response: Option<&ApiResponse>) {
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
