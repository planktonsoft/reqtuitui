mod app;
mod engine;
mod formatter;
mod importer;
mod models;
mod parser;
mod response_viewer;
mod storage;
mod ui;
mod vim;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use engine::HttpManager;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::collections::HashMap;
use std::time::Duration;
use std::{io, sync::Arc};
use tokio::sync::mpsc;
use tui_input::backend::crossterm::EventHandler;
use uuid::Uuid;

use crate::app::{CurrentScreen, Focus, NodeType};
use crate::models::{
    ApiRequest, BodyType, Collection, CollectionItem, EnvVariable, Folder, HttpMethod, RequestBody,
};
use crate::vim::{VimMode, apply_vim_style, handle_vim_key};
use crate::{
    app::{App, UiMessage, WorkMessage},
    storage::StorageManager,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Storage and Mock Data
    let storage = StorageManager::new(".reqtuitui_db")?;

    // 2. RESTORE WORKSPACE FROM STORAGE (or create default)
    let root_workspace_id = "root_workspace";

    let mut loaded_envs = storage.get_all_environments().unwrap_or_default();

    if loaded_envs.is_empty() {
        let default_env = models::Environment {
            id: "env_default_1".into(),
            name: "Local Dev".into(),
            variables: vec![models::EnvVariable {
                key: "base_url".into(),
                value: "http://localhost:8080".into(),
                enabled: true,
            }],
        };

        if let Err(e) = storage.save_environment(&default_env) {
            eprintln!("Failed to save default environment: {}", e);
        }

        loaded_envs.push(default_env);
    }

    let global_cookies = storage.get_global_cookies().unwrap_or_default();

    let my_workspace = match storage.get_collection(root_workspace_id) {
        Ok(Some(saved_collection)) => {
            // Successfuly loaded from Sled!
            saved_collection
        }
        _ => Collection {
            id: root_workspace_id.to_string(),
            name: "My Workspace".into(),
            description: None,
            items: vec![CollectionItem::Folder(Folder {
                id: "f_1".into(),
                name: "User API".into(),
                items: vec![CollectionItem::Request(ApiRequest {
                    id: "1".into(),
                    name: "Get JSON data".into(),
                    url: "{{base_url}}/posts/1".into(),
                    method: models::HttpMethod::GET,
                    headers: std::collections::HashMap::new(),
                    query_params: std::collections::HashMap::new(),
                    body: models::RequestBody {
                        body_type: models::BodyType::None,
                        content: None,
                    },
                })],
            })],
        },
    };

    let mut app = App::new(my_workspace, loaded_envs);
    app.global_cookies = global_cookies;

    // 2. Setup Channels for Inter-Thread Communication
    let (tx_worker, mut rx_worker) = mpsc::channel::<WorkMessage>(100);
    let (tx_ui, mut rx_ui) = mpsc::channel::<UiMessage>(100);

    // 3. Spawn Background Network Worker
    let http_manager = Arc::new(HttpManager::new());
    tokio::spawn(async move {
        while let Some(message) = rx_worker.recv().await {
            match message {
                WorkMessage::RunRequest(req, env) => {
                    let _ = tx_ui.send(UiMessage::RequestStarted).await;
                    match http_manager
                        .execute(req, env.as_ref())
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
                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                // --- POPUP INTERCEPTOR ---
                if app.cookie_popup_open {
                    if key.code == KeyCode::Esc {
                        app.cookie_popup_open = false;
                        continue;
                    }

                    if key.code == KeyCode::Char('s') && is_ctrl {
                        app.global_cookies.clear();
                        for line in app.cookie_input.lines() {
                            if let Some((k, v)) = line.split_once('=') {
                                app.global_cookies
                                    .insert(k.trim().to_string(), v.trim().to_string());
                            }
                        }
                        let _ = storage.save_global_cookies(&app.global_cookies);
                        app.status_message = Some("🍪 Cookie Jar saved!".to_string());
                        app.cookie_popup_open = false;
                        continue;
                    }
                    app.cookie_input.input(key);
                    continue;
                }

                if app.import_popup_open {
                    if key.code == KeyCode::Esc {
                        app.import_popup_open = false; // Cancel import 
                        continue;
                    }

                    if key.code == KeyCode::Char('s') && is_ctrl {
                        // 1. Get the raw text they pasted
                        let raw_curl = app.import_input.lines().join("\n");

                        // 2. Pass it to our parser
                        match importer::parse_curl(&raw_curl) {
                            Ok(new_request) => {
                                // 3. Add the parsed request intelligently to the tree!
                                app.add_new_request(new_request);

                                // 4. Save to database
                                let _ = storage.save_collection(&app.root_collection);

                                // 5. Update UI
                                app.sync_ui_to_selected_node();
                                app.status_message =
                                    Some("🚀 Successfully imported cURL command!".to_string());
                            }
                            Err(e) => {
                                app.status_message = Some(format!("❌ Import Failed: {}", e));
                            }
                        }
                        app.import_popup_open = false;
                        continue;
                    }

                    // Pass standard typing (and pasting!) directly into the text area
                    app.import_input.input(key);
                    continue;
                }

                if app.new_env_popup_open {
                    match key.code {
                        KeyCode::Esc => {
                            app.new_env_popup_open = false;
                        }
                        KeyCode::Enter => {
                            let env_name = app.new_env_input.value().to_string();

                            // Create a completely empty environment
                            let new_env = models::Environment {
                                id: uuid::Uuid::new_v4().to_string(),
                                name: env_name,
                                variables: vec![],
                            };

                            // Save to Sled Database
                            if let Err(e) = storage.save_environment(&new_env) {
                                app.status_message =
                                    Some(format!("❌ Failed to save environment: {}", e));
                            } else {
                                app.status_message =
                                    Some("🌍 New environment created!".to_string());

                                // Add to UI state and automatically select it
                                app.environments.push(new_env);
                                app.active_env_idx = Some(app.environments.len() - 1);
                            }

                            app.new_env_popup_open = false;
                        }
                        _ => {
                            app.new_env_input.handle_event(&Event::Key(key));
                        }
                    }
                    continue;
                }

                if app.env_var_popup_open {
                    if key.code == KeyCode::Esc {
                        app.env_var_popup_open = false;
                        continue;
                    }

                    if key.code == KeyCode::Char('s') && is_ctrl {
                        let env_idx = app.env_popup_selected_idx.saturating_sub(1);

                        let new_vars = parse_env_vars_from_ui(app.env_var_input.lines());

                        app.environments[env_idx].variables = new_vars;

                        let updated_env = &app.environments[env_idx];
                        if let Err(e) = storage.save_environment(updated_env) {
                            app.status_message =
                                Some(format!("❌ Failed to save variables: {}", e));
                        } else {
                            app.status_message =
                                Some(format!("✅ Variables saved for {}", updated_env.name));
                        }
                        app.env_var_popup_open = false;
                        continue;
                    }
                    app.env_var_input.input(key);
                    continue;
                }

                // If the popup is open, handle its logic and IGNORE everything else.
                if app.env_popup_open {
                    match key.code {
                        KeyCode::Char('n') => {
                            // Lanuch the new environment creator!
                            app.env_popup_open = false;
                            app.new_env_popup_open = true;
                            app.new_env_input = tui_input::Input::default();
                        }
                        KeyCode::Char('v') => {
                            // Open the variable editor!
                            if app.env_popup_selected_idx > 0 {
                                let env_idx = app.env_popup_selected_idx - 1;
                                let target_env = &app.environments[env_idx];

                                // Format the existing variables into "KEY=VALUE" lines
                                let var_lines: Vec<String> = target_env
                                    .variables
                                    .iter()
                                    .map(|v| format!("{}={}", v.key, v.value))
                                    .collect();

                                app.env_var_input = tui_textarea::TextArea::new(var_lines);
                                app.env_var_popup_open = true; // Show the editor
                            } else {
                                app.status_message = Some(
                                    "⚠️ Cannot edit variables for 'No Environment'.".to_string(),
                                );
                            }
                        }
                        KeyCode::Esc => {
                            app.env_popup_open = false; // Close without saving
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            // Max index is environemtns.len() because index 0 is "No Environment"
                            if app.env_popup_selected_idx < app.environments.len() {
                                app.env_popup_selected_idx += 1;
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.env_popup_selected_idx > 0 {
                                app.env_popup_selected_idx -= 1;
                            }
                        }
                        KeyCode::Enter => {
                            // Set the active environment based on selection
                            if app.env_popup_selected_idx == 0 {
                                app.active_env_idx = None;
                                app.status_message = Some("🌍 Environment cleared.".to_string());
                            } else {
                                app.active_env_idx = Some(app.env_popup_selected_idx - 1);
                                app.status_message = Some(format!(
                                    "🌍 Switched to environment: {}",
                                    app.environments[app.active_env_idx.unwrap()].name
                                ));
                            }
                            app.env_popup_open = false; // Close the popup
                        }
                        _ => {}
                    }
                    continue; // CRITICAL: Skip the rest of the UI logic while popup is open!
                }

                if app.rename_popup_open {
                    match key.code {
                        KeyCode::Esc => {
                            app.rename_popup_open = false;
                        }
                        KeyCode::Enter => {
                            let nodes = app.get_visible_nodes();
                            if let Some(active_node) = nodes.get(app.selected_node_idx) {
                                let new_name = app.rename_input.value().to_string();

                                // 1. Update the tree structure
                                app.rename_node(&active_node.id, &new_name);

                                // 2. Persist the entire collection to Sled
                                match storage.save_collection(&app.root_collection) {
                                    Ok(_) => {
                                        app.status_message =
                                            Some("✅ Node renamed successfully.".to_string())
                                    }
                                    Err(e) => {
                                        app.status_message = Some(format!("❌ Save failed: {}", e))
                                    }
                                }
                            }
                            app.rename_popup_open = false; // Close popup
                        }
                        _ => {
                            // Pass all typing, backspaces, and arrows to the input field
                            app.rename_input.handle_event(&Event::Key(key));
                        }
                    }
                    continue;
                }

                // Scroll Response Up
                if key.code == KeyCode::PageUp
                    || (is_ctrl && key.code == KeyCode::Char('u'))
                    || (is_shift && key.code == KeyCode::Up)
                {
                    app.response_viewer.scroll_up(3);
                    continue;
                }

                // Scroll Response Down
                if key.code == KeyCode::PageDown
                    || (is_ctrl && key.code == KeyCode::Char('d'))
                    || (is_shift && key.code == KeyCode::Down)
                {
                    app.response_viewer.scroll_down(3);
                    continue;
                }

                // --- GLOBAL TAB NAVIGATION ---
                if key.code == KeyCode::Tab {
                    app.zoom_editor_open = false;

                    app.focus = match app.focus {
                        Focus::Sidebar => Focus::UrlBar,
                        Focus::UrlBar => Focus::HeadersEditor,
                        Focus::HeadersEditor => Focus::BodyEditor,
                        Focus::BodyEditor => Focus::Sidebar,
                    };
                    continue;
                }

                if key.code == KeyCode::Char('e') && is_ctrl {
                    app.env_popup_open = true;
                    // Reset the popup cursor to the currently active environment
                    app.env_popup_selected_idx = app.active_env_idx.map(|idx| idx + 1).unwrap_or(0);
                    continue;
                }

                if key.code == KeyCode::Char('s') && is_ctrl {
                    let nodes = app.get_visible_nodes();
                    if nodes.is_empty() {
                        continue;
                    }

                    // Only allow saving if we are actually nighlighting a Request (not a folder)
                    if let NodeType::Request(req) = &nodes[app.selected_node_idx].node_type {
                        let mut request_to_save = req.clone();

                        request_to_save.url = app.url_input.value().to_string();
                        request_to_save.headers = parse_headers_from_ui(app.headers_input.lines());
                        request_to_save.body.content = Some(app.body_input.lines().join("\n"));
                        request_to_save.name = req.name.clone();

                        // Update the item deep inside the nested Collection AST
                        app.update_request_in_tree(&request_to_save);

                        // Save the ENTIRE updated collection to Sled!
                        match storage.save_collection(&app.root_collection) {
                            Ok(_) => app.status_message = Some("💾 Collection saved!".to_string()),
                            Err(e) => {
                                app.status_message = Some(format!("❌ Failed to save: {}", e))
                            }
                        }
                    }

                    continue;
                }

                if key.code == KeyCode::Char('n') && is_ctrl {
                    let new_id = Uuid::new_v4().to_string();
                    let blank_request = ApiRequest {
                        id: new_id.clone(),
                        name: "New Request".to_string(),
                        url: "http://".to_string(),
                        method: HttpMethod::GET,
                        headers: std::collections::HashMap::new(),
                        query_params: std::collections::HashMap::new(),
                        body: RequestBody {
                            body_type: BodyType::None,
                            content: None,
                        },
                    };

                    app.add_new_request(blank_request);

                    if let Err(e) = storage.save_collection(&app.root_collection) {
                        app.status_message = Some(format!("❌ Save failed: {}", e));
                        continue;
                    }

                    // 3. Move the user's cursor to the very bottom of the list
                    let nodes = app.get_visible_nodes();
                    if let Some(new_idx) = nodes.iter().position(|n| n.id == new_id) {
                        app.selected_node_idx = new_idx;
                    }

                    // 4. Sync the text fields beautifully
                    app.sync_ui_to_selected_node();

                    app.status_message = Some("✨ New request created!".to_string());

                    continue;
                }

                if key.code == KeyCode::Char('y') && is_ctrl {
                    let nodes = app.get_visible_nodes();

                    if let Some(active_node) = nodes.get(app.selected_node_idx) {
                        // Only allow method cycling if it's a Request!
                        if let NodeType::Request(req) = &active_node.node_type {
                            let mut updated_req = req.clone();

                            // Cycle the method of the active request
                            updated_req.method = match req.method {
                                HttpMethod::GET => HttpMethod::POST,
                                HttpMethod::POST => HttpMethod::PUT,
                                HttpMethod::PUT => HttpMethod::DELETE,
                                HttpMethod::DELETE => HttpMethod::PATCH,
                                HttpMethod::PATCH => HttpMethod::GET,
                                _ => HttpMethod::GET,
                            };

                            // Automatically adjust body type based on method transition
                            updated_req.body.body_type = match updated_req.method {
                                HttpMethod::POST | HttpMethod::PUT | HttpMethod::PATCH => {
                                    if updated_req.body.body_type == BodyType::None {
                                        BodyType::RawJson
                                    } else {
                                        updated_req.body.body_type
                                    }
                                }
                                HttpMethod::GET
                                | HttpMethod::DELETE
                                | HttpMethod::HEAD
                                | HttpMethod::OPTIONS => BodyType::None,
                            };

                            updated_req.url = app.url_input.value().to_string();
                            updated_req.headers = parse_headers_from_ui(app.headers_input.lines());
                            updated_req.body.content = Some(app.body_input.lines().join("\n"));

                            app.update_request_in_tree(&updated_req);

                            let _ = storage.save_collection(&app.root_collection);

                            app.status_message =
                                Some(format!("🔄 Method changed to {:?}", updated_req.method));
                        } else {
                            app.status_message =
                                Some("⚠️ Cannot change HTTP method of a folder.".to_string());
                        }
                    }

                    continue;
                }

                if key.code == KeyCode::Char('b') && is_ctrl {
                    let nodes = app.get_visible_nodes();

                    if let Some(active_node) = nodes.get(app.selected_node_idx) {
                        // Only allow body type cycling if it's a Request!
                        if let NodeType::Request(req) = &active_node.node_type {
                            let mut updated_req = req.clone();

                            // Cycle the body type of the active request
                            updated_req.body.body_type = match req.body.body_type {
                                BodyType::None => BodyType::RawJson,
                                BodyType::RawJson => BodyType::RawText,
                                BodyType::RawText => BodyType::FormData,
                                BodyType::FormData => BodyType::None,
                            };

                            updated_req.url = app.url_input.value().to_string();
                            updated_req.headers = parse_headers_from_ui(app.headers_input.lines());
                            updated_req.body.content = Some(app.body_input.lines().join("\n"));

                            app.update_request_in_tree(&updated_req);

                            let _ = storage.save_collection(&app.root_collection);

                            app.status_message = Some(format!(
                                "🔄 Body type changed to {:?}",
                                updated_req.body.body_type
                            ));
                        } else {
                            app.status_message =
                                Some("⚠️ Cannot change body type of a folder.".to_string());
                        }
                    }

                    continue;
                }

                if key.code == KeyCode::Char('r') && is_ctrl {
                    // 1. Get the flattened view of the tree
                    let nodes = app.get_visible_nodes();

                    // 2. Safely grab the currently highlighted node
                    if let Some(active_node) = nodes.get(app.selected_node_idx) {
                        app.rename_popup_open = true;

                        let current_name = active_node.name.clone();
                        app.rename_input = tui_input::Input::default().with_value(current_name);
                    }

                    continue;
                }

                if key.code == KeyCode::Char('f') && is_ctrl {
                    let new_id = Uuid::new_v4().to_string();
                    let blank_folder = models::Folder {
                        id: new_id.clone(),
                        name: "New Folder".to_string(),
                        items: vec![], // Starts empty
                    };

                    // 1. Add it to our nested tree state
                    app.add_new_folder(blank_folder);

                    // 2. Save the entire updated collection to the database
                    if let Err(e) = storage.save_collection(&app.root_collection) {
                        app.status_message = Some(format!("❌ Save failed: {}", e));
                        continue;
                    }

                    // 3. Move the user's cursor to the newly created folder
                    let nodes = app.get_visible_nodes();
                    if let Some(new_idx) = nodes.iter().position(|n| n.id == new_id) {
                        app.selected_node_idx = new_idx;
                    }

                    // 4. Sync the text fields (This will clear the editors and show the Folder
                    //    Splash Screen)
                    app.sync_ui_to_selected_node();

                    app.status_message = Some("📁 New folder created!".to_string());
                    continue;
                }

                if key.code == KeyCode::Char('o') && is_ctrl {
                    app.import_popup_open = true;
                    app.import_input = tui_textarea::TextArea::default(); // Reset the input box
                    continue;
                }

                if key.code == KeyCode::Char('g') && is_ctrl {
                    app.cookie_popup_open = true;
                    // Format existing cookies into the text editor
                    let lines: Vec<String> = app
                        .global_cookies
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();
                    app.cookie_input = tui_textarea::TextArea::new(lines);
                    continue;
                }

                if key.code == KeyCode::F(2) {
                    if app.focus == Focus::HeadersEditor || app.focus == Focus::BodyEditor {
                        app.zoom_editor_open = !app.zoom_editor_open;
                    } else {
                        app.status_message =
                            Some("⚠️ You can only zoom the Headers or Body editors.".to_string());
                    }
                    continue;
                }

                if key.code == KeyCode::F(3) {
                    if app.focus == Focus::BodyEditor {
                        let raw_text = app.body_input.lines().join("\n");

                        if let Ok(parsed_json) =
                            serde_json::from_str::<serde_json::Value>(&raw_text)
                        {
                            if let Ok(pretty_json) = serde_json::to_string_pretty(&parsed_json) {
                                app.body_input = tui_textarea::TextArea::new(
                                    pretty_json.lines().map(String::from).collect(),
                                );
                                app.status_message =
                                    Some("✨ JSON perfectly formatted!".to_string());
                            }
                        } else {
                            app.status_message =
                                Some("❌ Invalid JSON! Cannot format.".to_string());
                        }
                    } else {
                        app.status_message =
                            Some("⚠️ You must be in the Body Editor to format JSON.".to_string());
                    }
                    continue;
                }

                if key.code == KeyCode::F(4) {
                    app.vim_emulation_active = !app.vim_emulation_active;
                    app.vim_mode = VimMode::Normal; // Reset to normal on toggle
                    let status = if app.vim_emulation_active {
                        "🟢 Vim Mode Enabled"
                    } else {
                        "🔴 Vim Mode Disabled"
                    };
                    app.status_message = Some(status.to_string());
                    continue;
                }

                // --- PANE-SPECIFIC CONTROLS ---
                match app.focus {
                    Focus::Sidebar => {
                        match key.code {
                            KeyCode::Char('q') => app.current_screen = CurrentScreen::Exiting,
                            KeyCode::Down | KeyCode::Char('j') => {
                                let nodes = app.get_visible_nodes();
                                if app.selected_node_idx < nodes.len().saturating_sub(1) {
                                    app.selected_node_idx += 1;

                                    app.sync_ui_to_selected_node();
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if app.selected_node_idx > 0 {
                                    app.selected_node_idx -= 1;

                                    app.sync_ui_to_selected_node();
                                }
                            }
                            KeyCode::Enter => {
                                let nodes = app.get_visible_nodes();
                                if nodes.is_empty() {
                                    continue;
                                }

                                let active_node = &nodes[app.selected_node_idx];

                                match &active_node.node_type {
                                    NodeType::Folder { expended } => {
                                        // It's a folder! Toggle its state in our HashSet.
                                        if *expended {
                                            app.expanded_folders.remove(&active_node.id);
                                        } else {
                                            app.expanded_folders.insert(active_node.id.clone());
                                        }
                                    }
                                    NodeType::Request(req) => {
                                        // It's a request! Build it and fire it.
                                        let mut active_req = req.clone();
                                        active_req.url = app.url_input.value().to_string();
                                        active_req.headers =
                                            parse_headers_from_ui(app.headers_input.lines());
                                        active_req.body.content =
                                            Some(app.body_input.lines().join("\n"));

                                        if !app.global_cookies.is_empty() {
                                            let cookie_str = app
                                                .global_cookies
                                                .iter()
                                                .map(|(k, v)| format!("{}={}", k, v))
                                                .collect::<Vec<_>>()
                                                .join("; ");

                                            active_req
                                                .headers
                                                .insert("Cookie".to_string(), cookie_str);
                                        }

                                        // Grab a clone of the active environment, if one is selected
                                        let active_env = app
                                            .active_env_idx
                                            .map(|idx| app.environments[idx].clone());
                                        tx_worker
                                            .send(WorkMessage::RunRequest(active_req, active_env))
                                            .await?;
                                    }
                                }
                            }
                            KeyCode::Char('e') => {
                                // Press 'e' to Edit the URL
                                app.focus = Focus::UrlBar;
                            }
                            KeyCode::Delete | KeyCode::Backspace => {
                                let nodes = app.get_visible_nodes();
                                if nodes.is_empty() {
                                    continue;
                                }

                                let active_node = &nodes[app.selected_node_idx];
                                let target_id = active_node.id.clone();

                                // 1. Delete from our memory tree
                                app.delete_node(&target_id);

                                // 2. Save the newly truncated collection to Sled
                                if let Err(e) = storage.save_collection(&app.root_collection) {
                                    app.status_message =
                                        Some(format!("❌ Failed to save after delete: {}", e));
                                } else {
                                    app.status_message = Some("🗑️ Item deleted.".to_string());
                                }

                                // 3. Shift the cursor up so it doesn't crash on out-of-bounds
                                let updated_nodes = app.get_visible_nodes();
                                if app.selected_node_idx >= updated_nodes.len() {
                                    app.selected_node_idx = updated_nodes.len().saturating_sub(1);
                                }

                                // 4. Sync the text editors with whatever item we landed on
                                app.sync_ui_to_selected_node();
                            }
                            _ => {}
                        }
                    }

                    Focus::UrlBar => match key.code {
                        KeyCode::Esc => {
                            app.zoom_editor_open = false;
                            app.focus = Focus::Sidebar;
                        }
                        KeyCode::Enter => {
                            // 1. Get the current active node
                            let nodes = app.get_visible_nodes();

                            if let Some(active_node) = nodes.get(app.selected_node_idx) {
                                if let NodeType::Request(req) = &active_node.node_type {
                                    let mut updated_req = req.clone();

                                    // 2. Sync ALL text fields to prevent data loss
                                    updated_req.url = app.url_input.value().to_string();
                                    updated_req.headers =
                                        parse_headers_from_ui(app.headers_input.lines());
                                    updated_req.body.content =
                                        Some(app.body_input.lines().join("\n"));

                                    // 3. Update the item deep inside the nested tree
                                    app.update_request_in_tree(&updated_req);

                                    app.status_message = Some(
                                        "📝 URL updated in memory. (Ctrl+S to save)".to_string(),
                                    );
                                }
                            }
                            app.focus = Focus::Sidebar;
                        }
                        _ => {
                            // Pass all other keys (letters, backspace, arrows) directly to the
                            // input handler!
                            app.url_input.handle_event(&Event::Key(key));
                        }
                    },
                    Focus::HeadersEditor => {
                        if app.vim_emulation_active {
                            apply_vim_style(&mut app.body_input, true, app.vim_mode);

                            if handle_vim_key(&mut app.body_input, key, &mut app.vim_mode) {
                                if key.code == KeyCode::Esc && app.vim_mode == VimMode::Normal {
                                    app.zoom_editor_open = false;
                                    app.focus = Focus::Sidebar;
                                }
                                continue;
                            }
                        } else {
                            apply_vim_style(&mut app.body_input, false, app.vim_mode);
                        }

                        match key.code {
                            KeyCode::Esc => {
                                app.zoom_editor_open = false;
                                app.focus = Focus::Sidebar;
                            }
                            KeyCode::Char('s') if is_ctrl => {
                                // SAVE LOGIC (Ctrl + S)
                                let nodes = app.get_visible_nodes();
                                if let Some(active_node) = nodes.get(app.selected_node_idx) {
                                    if let NodeType::Request(req) = &active_node.node_type {
                                        let mut updated_req = req.clone();

                                        updated_req.url = app.url_input.value().to_string();
                                        updated_req.headers =
                                            parse_headers_from_ui(app.headers_input.lines());
                                        updated_req.body.content =
                                            Some(app.body_input.lines().join("\n"));

                                        app.update_request_in_tree(&updated_req);
                                        if let Err(e) =
                                            storage.save_collection(&app.root_collection)
                                        {
                                            app.status_message =
                                                Some(format!("❌ Failed to save headers: {}", e));
                                        } else {
                                            app.status_message =
                                                Some("💾 Headers saved!".to_string());
                                        }
                                    }
                                }
                            }
                            _ => {
                                app.body_input.input(key);
                            }
                        }
                    }
                    Focus::BodyEditor => {
                        if app.vim_emulation_active {
                            apply_vim_style(&mut app.body_input, true, app.vim_mode);

                            if handle_vim_key(&mut app.body_input, key, &mut app.vim_mode) {
                                if key.code == KeyCode::Esc && app.vim_mode == VimMode::Normal {
                                    app.zoom_editor_open = false;
                                    app.focus = Focus::Sidebar;
                                }
                                continue;
                            }
                        } else {
                            apply_vim_style(&mut app.body_input, false, app.vim_mode);
                        }

                        match key.code {
                            KeyCode::Esc => {
                                app.zoom_editor_open = false;
                                app.focus = Focus::Sidebar;
                            }
                            KeyCode::Char('s') if is_ctrl => {
                                // SAVE LOGIC (Ctrl + S)
                                let nodes = app.get_visible_nodes();
                                if let Some(active_node) = nodes.get(app.selected_node_idx) {
                                    if let NodeType::Request(req) = &active_node.node_type {
                                        let mut updated_req = req.clone();

                                        updated_req.url = app.url_input.value().to_string();
                                        updated_req.headers =
                                            parse_headers_from_ui(app.headers_input.lines());
                                        updated_req.body.content =
                                            Some(app.body_input.lines().join("\n"));

                                        app.update_request_in_tree(&updated_req);
                                        if let Err(e) =
                                            storage.save_collection(&app.root_collection)
                                        {
                                            app.status_message =
                                                Some(format!("❌ Failed to save body: {}", e));
                                        } else {
                                            app.status_message = Some("💾 Body saved!".to_string());
                                        }
                                    }
                                }
                            }
                            _ => {
                                app.body_input.input(key);
                            }
                        }
                    }
                }
            }
        }

        // Process any incoming messages from our network worker thread
        while let Ok(msg) = rx_ui.try_recv() {
            match msg {
                UiMessage::RequestStarted => {
                    app.is_loading = true;
                    app.response_viewer.reset_scroll();
                    app.active_response = None;
                }
                UiMessage::RequestCompleted(res) => {
                    app.is_loading = false;
                    match res {
                        Ok(resp) => {
                            // Update global cookies with any new cookies from the response
                            if !resp.new_cookies.is_empty() {
                                for (k, v) in resp.new_cookies.iter() {
                                    app.global_cookies.insert(k.clone(), v.clone());
                                }
                                let _ = storage.save_global_cookies(&app.global_cookies);
                            }
                            app.active_response = Some(resp);
                            app.status_message = Some("✅ Request completed.".to_string());
                        }
                        Err(e) => app.status_message = Some(format!("❌ Network Error: {}", e)),
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

fn parse_headers_from_ui(lines: &[String]) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in lines {
        // Find the first colon to separate Key from Value
        if let Some((key, value)) = line.split_once(':') {
            let k = key.trim().to_string();
            let v = value.trim().to_string();
            if !k.is_empty() {
                headers.insert(k, v);
            }
        }
    }
    headers
}

fn parse_env_vars_from_ui(lines: &[String]) -> Vec<EnvVariable> {
    let mut vars = Vec::new();
    for line in lines {
        // Split at the first '=' sign
        if let Some((key, value)) = line.split_once('=') {
            let k = key.trim().to_string();
            let v = value.trim().to_string();
            if !k.is_empty() {
                vars.push(EnvVariable {
                    key: k,
                    value: v,
                    enabled: true, // Default to true
                });
            }
        }
    }
    vars
}
