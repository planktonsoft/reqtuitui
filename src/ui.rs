use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};
use tokio::io::split;

use crate::app::{App, Focus, NodeType};
use crate::formatter::format_json_response;

pub fn render(f: &mut Frame, app: &mut App) {
    // Split the screen horizontally into Sidebar (25%) and Main Panel (75%)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(f.area());

    render_sidebar(f, app, main_chunks[0]);
    render_main_panel(f, app, main_chunks[1]);

    if app.env_popup_open {
        render_env_popup(f, app);
    }

    if app.rename_popup_open {
        render_rename_popup(f, app);
    }
}

// Helper function to create a centered rectangle for out popup
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_env_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 40, f.area());

    // Clear the background behind the popup
    f.render_widget(Clear, area);

    // Build the list of environments
    let mut items: Vec<ListItem> = vec![ListItem::new("No Environment")];

    for env in &app.environments {
        items.push(ListItem::new(env.name.clone()));
    }

    // Highlight the currently selected item in the popup menu
    let items = items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            if i == app.env_popup_selected_idx {
                item.style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::REVERSED),
                )
            } else {
                item
            }
        })
        .collect::<Vec<_>>();

    let list = List::new(items).block(
        Block::default()
            .title(" Select Environment (ESC to cancel) ")
            .borders(Borders::ALL),
    );

    f.render_widget(list, area);
}

fn render_rename_popup(f: &mut Frame, app: &App) {
    // Create a layout that centers a box exactly 3 lines high and 40 columns wide
    let vertical_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((f.area().height.saturating_sub(3)) / 2),
            Constraint::Length(3), // Exact height for an input box with borders
            Constraint::Min(0),
        ])
        .split(f.area());

    let horizontal_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((f.area().width.saturating_sub(40)) / 2),
            Constraint::Length(40),
            Constraint::Min(0),
        ])
        .split(vertical_split[1]);

    let area = horizontal_split[1];

    // Clear the background behind the popup so text doesn't bleed through
    f.render_widget(Clear, area);

    // Draw the input box
    let block = Block::default()
        .title(" Rename Request (Enter to save, Esc to cancel) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let input_widget = Paragraph::new(app.rename_input.value()).block(block);
    f.render_widget(input_widget, area);

    // Draw the blinking cursor inside the popup
    f.set_cursor_position(Position {
        x: area.x + 1 + app.rename_input.visual_cursor() as u16,
        y: area.y + 1,
    });
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let visible_nodes = app.get_visible_nodes();

    let items: Vec<ListItem> = visible_nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let style = if i == app.selected_node_idx {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Calculate visual indentation based on depth
            let indent = "  ".repeat(node.depth);

            // Format the icon and text based on type
            let text = match &node.node_type {
                NodeType::Folder { expended: true } => format!("{}▼ 📂 {}", indent, node.name),
                NodeType::Folder { expended: false } => format!("{}▶ 📁 {}", indent, node.name),
                NodeType::Request(req) => format!("{}  [{:?}] {}", indent, req.method, node.name),
            };

            ListItem::new(text).style(style)
        })
        .collect();

    let style = if app.focus == Focus::Sidebar {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let list = List::new(items).block(
        Block::default()
            .title(" Collections ")
            .borders(Borders::ALL)
            .border_style(style),
    );

    f.render_widget(list, area);
}

fn render_main_panel(f: &mut Frame, app: &mut App, area: Rect) {
    // 1. Split off the bottom line for the Status Bar first
    let main_and_status = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let main_area = main_and_status[0];
    let status_area = main_and_status[1];

    let nodes = app.get_visible_nodes();

    if let Some(active_node) = nodes.get(app.selected_node_idx) {
        match &active_node.node_type {
            NodeType::Request(req) => {
                // --- RENDER REQUEST UI ---
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),      // URL Bar
                        Constraint::Length(6),      // Headers Editor (Fixed height)
                        Constraint::Min(6),         // Body Editor (takes at least 10 lines)
                        Constraint::Percentage(45), // Response (takes remaining bottom half)
                    ])
                    .split(main_area);

                let url_style = if app.focus == Focus::UrlBar {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };

                let url_title = format!(
                    " [{:?}] {} (Press 'e' to edit, Ctrl+Y to change method) ",
                    req.method, active_node.name
                );

                // Top: Request URL Bar
                let url_block = Paragraph::new(app.url_input.value()).block(
                    Block::default()
                        .title(url_title)
                        .borders(Borders::ALL)
                        .border_style(url_style),
                );
                f.render_widget(url_block, chunks[0]);

                if app.focus == Focus::UrlBar {
                    // We calculate where the cursor should sit based on the chunk coordinates
                    // chunks[0].x + 1 accounts for the border thickness
                    f.set_cursor_position(Position {
                        x: chunks[0].x + 1 + app.url_input.visual_cursor() as u16,
                        y: chunks[0].y + 1,
                    });
                }

                let headers_style = if app.focus == Focus::HeadersEditor {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };

                app.headers_input.set_block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(headers_style)
                        .title(" Headers (Format: `Key: Value`) "),
                );
                f.render_widget(&app.headers_input, chunks[1]);

                let body_style = if app.focus == Focus::BodyEditor {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };

                app.body_input.set_block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(body_style)
                        .title(format!(
                            " Body ({:?}) - Press 'Tab' to switch focus ",
                            req.body.body_type
                        )),
                );

                f.render_widget(&app.body_input, chunks[2]);

                // Bottom: Response Area
                let response_content = if app.is_loading {
                    Text::raw("Sending request...")
                } else if let Some(resp) = &app.active_response {
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
                    .scroll((app.response_scroll, 0));
                f.render_widget(response_block, chunks[3]);

                let mut scrollbar_state = ScrollbarState::default()
                    .content_length(content_len)
                    .position(app.response_scroll as usize);

                f.render_stateful_widget(
                    Scrollbar::default()
                        .orientation(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("▲"))
                        .end_symbol(Some("▼")),
                    chunks[3],
                    &mut scrollbar_state,
                );
            }

            NodeType::Folder { .. } => {
                // --- RENDER FOLDER UI ---
                // If they highlight a folder, just show a clean instructional screen
                let msg = format!(
                    "\n\n📂 {}\n\nPress 'Enter' to open/close this folder.\nPress 'Ctrl + N' to create a new request inside it.\nPress 'Ctrl + R' to rename it.",
                    active_node.name
                );

                let folder_block = Paragraph::new(msg)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::DarkGray)),
                    );
                f.render_widget(folder_block, main_area);
            }
        }
    }
    // --- RENDER STSTUS BAR (Always visible at the bottom) ---
    let status_text = app.status_message.as_deref().unwrap_or("");
    let status_bar =
        Paragraph::new(status_text).style(Style::default().bg(Color::Blue).fg(Color::White));

    f.render_widget(status_bar, status_area);
}
