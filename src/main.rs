mod app;
mod engine;
mod models;
mod parser;
mod storage;
mod ui;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use engine::HttpManager;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::time::Duration;
use std::{io, sync::Arc};
use tokio::sync::mpsc;
use tui_input::backend::crossterm::EventHandler;

use crate::app::{CurrentScreen, Focus};
use crate::{
    app::{App, UiMessage, WorkMessage},
    storage::StorageManager,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Storage and Mock Data
    let storage = StorageManager::new(".requestui_db")?;

    // For demonstration, pull from Sled or fallback to a default mock vector
    let mock_requests = storage.get_all_requests().unwrap_or_default();
    let mut app = App::new(if mock_requests.is_empty() {
        vec![models::ApiRequest {
            id: "1".into(),
            name: "Get JSON data".into(),
            url: "https://jsonplaceholder.typicode.com/posts/1".into(),
            method: models::HttpMethod::GET,
            headers: std::collections::HashMap::new(),
            query_params: std::collections::HashMap::new(),
            body: models::RequestBody {
                body_type: models::BodyType::None,
                content: None,
            },
        }]
    } else {
        mock_requests
    });

    // 2. Setup Channels for Inter-Thread Communication
    let (tx_worker, mut rx_worker) = mpsc::channel::<WorkMessage>(100);
    let (tx_ui, mut rx_ui) = mpsc::channel::<UiMessage>(100);

    // 3. Spawn Background Network Worker
    let http_manager = Arc::new(HttpManager::new());
    tokio::spawn(async move {
        while let Some(message) = rx_worker.recv().await {
            match message {
                WorkMessage::RunRequest(req) => {
                    let _ = tx_ui.send(UiMessage::RequestStarted).await;
                    // Execute with no active environment mapping for now
                    match http_manager
                        .execute(req, None)
                        .await
                        .map_err(|e| e.to_string())
                    {
                        Ok(resp) => {
                            let _ = tx_ui.send(UiMessage::RequestCompleted(Ok(resp))).await;
                        }
                        Err(err_str) => {
                            let _ = tx_ui.send(UiMessage::RequestCompleted(Err(err_str))).await;
                        }
                    }
                }
            }
        }
    });

    // 4. Initialize Terminal TUI Canvas
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 5. Main TUI Event Loop
    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;
        // Non-blocking poll for user input events
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // --- GLOBAL TAB NAVIGATION ---
                if key.code == KeyCode::Tab {
                    app.focus = match app.focus {
                        Focus::Sidebar => Focus::UrlBar,
                        Focus::UrlBar => Focus::BodyEditor,
                        Focus::BodyEditor => Focus::Sidebar,
                    };
                    continue;
                }

                if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Step A: Get a mutable reference to the active request
                    let active_idx = app.selected_request_idx;
                    let mut request_to_save = app.requests[active_idx].clone();

                    // Step B: Sync the UI text fields into the request struct
                    request_to_save.url = app.url_input.value().to_string();
                    request_to_save.body.content = Some(app.body_input.lines().join("\n"));

                    // Step C: Save it to our Sled database!
                    // (Because Sled is synchronous and very fast, doing this directly in the UI thread is fine)
                    match storage.save_request(&request_to_save) {
                        Ok(_) => {
                            // Update the app's in-memory array so the sidebar reflects changes
                            app.requests[active_idx] = request_to_save;
                            app.status_message = Some("💾 Request saved successfully!".to_string());
                        }
                        Err(e) => {
                            app.status_message = Some(format!("❌ Failed to save: {}", e));
                        }
                    }
                    continue; // Skip the rest of the key handling
                }

                // --- PANE-SPECIFIC CONTROLS ---
                match app.focus {
                    Focus::Sidebar => {
                        match key.code {
                            KeyCode::Char('q') => app.current_screen = CurrentScreen::Exiting,
                            KeyCode::Down | KeyCode::Char('j') => {
                                if app.selected_request_idx < app.requests.len().saturating_sub(1) {
                                    app.selected_request_idx += 1;
                                    let req = &app.requests[app.selected_request_idx];
                                    app.url_input =
                                        app.url_input.clone().with_value(req.url.clone());

                                    // Update the text area with the new request's body
                                    let body_text = req.body.content.clone().unwrap_or_default();
                                    app.body_input = tui_textarea::TextArea::new(
                                        body_text.lines().map(String::from).collect(),
                                    );
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if app.selected_request_idx > 0 {
                                    app.selected_request_idx -= 1;
                                    app.url_input = app.url_input.clone().with_value(
                                        app.requests[app.selected_request_idx].url.clone(),
                                    );
                                }
                            }
                            KeyCode::Enter => {
                                let mut active_req = app.requests[app.selected_request_idx].clone();
                                active_req.url = app.url_input.value().to_string();
                                active_req.body.content = Some(app.body_input.lines().join("\n"));
                                tx_worker.send(WorkMessage::RunRequest(active_req)).await?;
                            }
                            KeyCode::Char('e') => {
                                // Press 'e' to Edit the URL
                                app.focus = Focus::UrlBar;
                            }
                            _ => {}
                        }
                    }

                    Focus::UrlBar => match key.code {
                        KeyCode::Esc => {
                            // Escape returns focus to the sidebar
                            app.focus = Focus::Sidebar;
                        }
                        KeyCode::Enter => {
                            // Save the URL to our state and go back to sidebar
                            app.requests[app.selected_request_idx].url =
                                app.url_input.value().to_string();
                            app.focus = Focus::Sidebar;
                        }
                        _ => {
                            // Pass all other keys (letters, backspace, arrows) directly to the
                            // input handler!
                            app.url_input.handle_event(&Event::Key(key));
                        }
                    },
                    Focus::BodyEditor => match key.code {
                        KeyCode::Esc => {
                            app.focus = Focus::Sidebar;
                        }
                        _ => {
                            app.body_input.input(key);
                        }
                    },
                }
            }
        }

        // Process any incoming messages from our network worker thread
        while let Ok(msg) = rx_ui.try_recv() {
            match msg {
                UiMessage::RequestStarted => {
                    app.is_loading = true;
                }
                UiMessage::RequestCompleted(res) => {
                    app.is_loading = false;
                    if let Ok(resp) = res {
                        app.active_response = Some(resp);
                    }
                }
            }
        }

        if let CurrentScreen::Exiting = app.current_screen {
            break;
        }
    }

    // 6. Restore Terminal Context on exit
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
