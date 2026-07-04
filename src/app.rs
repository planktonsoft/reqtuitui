use crate::models::{ApiRequest, ApiResponse};

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

pub struct App {
    pub current_screen: CurrentScreen,
    pub requests: Vec<ApiRequest>,
    pub selected_request_idx: usize,
    pub active_response: Option<ApiResponse>,
    pub is_loading: bool,
}

impl App {
    pub fn new(initial_requests: Vec<ApiRequest>) -> Self {
        Self {
            current_screen: CurrentScreen::Sidebar,
            requests: initial_requests,
            selected_request_idx: 0,
            active_response: None,
            is_loading: false,
        }
    }
}
