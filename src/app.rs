use crate::models::{ApiRequest, ApiResponse};
use tui_input::Input;
use tui_textarea::TextArea;

#[derive(PartialEq)]
pub enum Focus {
    Sidebar,
    UrlBar,
    BodyEditor,
}

pub enum CurrentScreen {
    Sidebar,
    RequestPanel,
    Exiting,
}

/// Messages sent from the TUI to the background HTTP worker
pub enum WorkMessage {
    RunRequest(ApiRequest),
}

/// Messages sent from the background HTTP worker back to the TUI
pub enum UiMessage {
    RequestStarted,
    RequestCompleted(Result<ApiResponse, String>),
}

pub struct App<'a> {
    pub current_screen: CurrentScreen,
    pub focus: Focus,
    pub requests: Vec<ApiRequest>,
    pub selected_request_idx: usize,
    pub url_input: Input,
    pub body_input: TextArea<'a>,
    pub active_response: Option<ApiResponse>,
    pub is_loading: bool,
    pub status_message: Option<String>,
}

impl<'a> App<'a> {
    pub fn new(initial_requests: Vec<ApiRequest>) -> Self {
        let active_req = initial_requests.first();
        let initial_url = active_req.map(|r| r.url.clone()).unwrap_or_default();

        // Load the existing body content into the text area
        let mut body_input = TextArea::default();
        if let Some(req) = active_req {
            if let Some(content) = &req.body.content {
                // Split the string into lines for the text area
                body_input = TextArea::new(content.lines().map(|s| s.to_string()).collect());
            }
        }
        Self {
            current_screen: CurrentScreen::Sidebar,
            focus: Focus::Sidebar,
            requests: initial_requests,
            selected_request_idx: 0,
            url_input: Input::default().with_value(initial_url),
            body_input,
            active_response: None,
            is_loading: false,
            status_message: Some(
                "Ready. Press Tab to navigate, Enter to send, Ctrl+S to save.".to_string(),
            ),
        }
    }
}
