use std::collections::{HashMap, HashSet};

use crate::models::{
    ApiRequest, ApiResponse, Collection, CollectionItem, Environment, Folder,
};
use crate::response_viewer::ResponseViewer;
use crate::vim::VimMode;
use tui_input::Input;
use tui_textarea::TextArea;

// A helper enum to figure out what type of row we are rendering
pub enum NodeType {
    Folder { expended: bool },
    Request(ApiRequest),
}

// Represents one visible line in the sidebar
pub struct VisibleNode {
    pub id: String,
    pub name: String,
    pub depth: usize, // How many spaces to indent
    pub node_type: NodeType,
}

#[derive(PartialEq)]
pub enum Focus {
    Sidebar,
    UrlBar,
    HeadersEditor,
    BodyEditor,
}

pub enum CurrentScreen {
    Sidebar,
    RequestPanel,
    Exiting,
}

/// Messages sent from the TUI to the background HTTP worker
pub enum WorkMessage {
    RunRequest(ApiRequest, Option<Environment>),
}

/// Messages sent from the background HTTP worker back to the TUI
pub enum UiMessage {
    RequestStarted(String),
    RequestCompleted(String, Result<ApiResponse, String>),
}

pub struct App<'a> {
    pub current_screen: CurrentScreen,
    pub focus: Focus,
    pub url_input: Input,
    pub headers_input: TextArea<'a>,
    pub body_input: TextArea<'a>,
    pub active_response: Option<ApiResponse>,
    pub is_loading: bool,
    pub status_message: Option<String>,
    pub response_viewer: ResponseViewer,

    // --- Concurrent Request Tracking ---
    pub active_request_id: Option<String>,
    pub loading_requests: HashSet<String>,
    pub responses: HashMap<String, ApiResponse>,

    // --- Environment State ---
    pub environments: Vec<Environment>,
    pub active_env_idx: Option<usize>, // None means "No Environment"

    // --- Popup State ---
    pub env_popup_open: bool,
    pub env_popup_selected_idx: usize,

    pub rename_popup_open: bool,
    pub rename_input: Input,

    // --- Tree State ---
    pub root_collection: Collection,
    pub expanded_folders: HashSet<String>, // Stores IDs of open Folders
    pub selected_node_idx: usize,          // Replaces selected_request_idx

    // --- New Environment Popup State ---
    pub new_env_popup_open: bool,
    pub new_env_input: Input,

    // --- Environment Variable Editor State ---
    pub env_var_popup_open: bool,
    pub env_var_input: tui_textarea::TextArea<'a>,

    // --- Import Popup State ---
    pub import_popup_open: bool,
    pub import_input: tui_textarea::TextArea<'a>,

    // --- Cookie Jar State ---
    pub cookie_popup_open: bool,
    pub cookie_input: tui_textarea::TextArea<'a>,
    pub global_cookies: HashMap<String, String>,

    // --- Zen Mode State ---
    pub zoom_editor_open: bool,

    // --- Vim State ---
    pub vim_emulation_active: bool,
    pub vim_mode: VimMode,
}

impl<'a> App<'a> {
    pub fn new(root_collection: Collection, initial_envs: Vec<Environment>) -> Self {
        let mut app = Self {
            root_collection,
            expanded_folders: HashSet::new(),
            selected_node_idx: 0,
            current_screen: CurrentScreen::Sidebar,
            focus: Focus::Sidebar,
            url_input: Input::default(),
            headers_input: tui_textarea::TextArea::default(),
            body_input: tui_textarea::TextArea::default(),
            active_response: None,
            is_loading: false,
            status_message: Some(
                "Ready. Press Tab to navigate, Enter to send, Ctrl+S to save.".to_string(),
            ),
            response_viewer: ResponseViewer::new(),
            environments: initial_envs,
            active_env_idx: None,
            env_popup_open: false,
            env_popup_selected_idx: 0,
            rename_popup_open: false,
            rename_input: Input::default(),
            new_env_popup_open: false,
            new_env_input: Input::default(),
            env_var_popup_open: false,
            env_var_input: tui_textarea::TextArea::default(),

            import_popup_open: false,
            import_input: tui_textarea::TextArea::default(),

            cookie_popup_open: false,
            cookie_input: tui_textarea::TextArea::default(),
            global_cookies: HashMap::new(),

            zoom_editor_open: false,

            active_request_id: None,
            loading_requests: HashSet::new(),
            responses: HashMap::new(),

            vim_emulation_active: false,
            vim_mode: VimMode::Normal, // Start in Normal mode when enabled
        };

        app.sync_ui_to_selected_node();

        app
    }

    /// Helper to pull data from the active tree node into the text editors
    pub fn sync_ui_to_selected_node(&mut self) {
        let nodes = self.get_visible_nodes();

        // Ensure our index is valid
        if let Some(active_node) = nodes.get(self.selected_node_idx) {
            self.active_request_id = Some(active_node.id.clone());
            self.is_loading = self.loading_requests.contains(&active_node.id);
            self.active_response = self.responses.get(&active_node.id).cloned();

            // We only update text areas if the user is highlighting a Request
            if let NodeType::Request(req) = &active_node.node_type {
                self.url_input = tui_input::Input::default().with_value(req.url.clone());

                let header_lines: Vec<String> = req
                    .headers
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                self.headers_input = tui_textarea::TextArea::new(header_lines);

                let body_text = req.body.content.clone().unwrap_or_default();
                self.body_input =
                    tui_textarea::TextArea::new(body_text.lines().map(String::from).collect());
            } else {
                // If they highligh a folder, we can clear the inputs to avoid confusion
                self.url_input = tui_input::Input::default();
                self.headers_input = tui_textarea::TextArea::default();
                self.body_input = tui_textarea::TextArea::default();
            }
        }
    }

    // Recursively flattens the collection into a 1D list for the UI
    pub fn get_visible_nodes(&self) -> Vec<VisibleNode> {
        let mut result = Vec::new();
        Self::flatten_items(
            &self.root_collection.items,
            &self.expanded_folders,
            0,
            &mut result,
        );
        result
    }

    fn flatten_items(
        items: &[CollectionItem],
        expanded: &HashSet<String>,
        depth: usize,
        result: &mut Vec<VisibleNode>,
    ) {
        for item in items {
            match item {
                CollectionItem::Folder(f) => {
                    let is_expanded = expanded.contains(&f.id);
                    result.push(VisibleNode {
                        id: f.id.clone(),
                        name: f.name.clone(),
                        depth,
                        node_type: NodeType::Folder {
                            expended: is_expanded,
                        },
                    });

                    // If it's open, recursively add its children right below it!
                    if is_expanded {
                        Self::flatten_items(&f.items, expanded, depth + 1, result);
                    }
                }
                CollectionItem::Request(r) => {
                    result.push(VisibleNode {
                        id: r.id.clone(),
                        name: r.name.clone(),
                        depth,
                        node_type: NodeType::Request(r.clone()),
                    });
                }
            }
        }
    }

    /// Intelligently adds a new request to the tree based on cursor position
    pub fn add_new_request(&mut self, new_req: ApiRequest) {
        let nodes = self.get_visible_nodes();
        let new_item = CollectionItem::Request(new_req);

        if let Some(active_node) = nodes.get(self.selected_node_idx) {
            match &active_node.node_type {
                NodeType::Folder { expended } => {
                    if *expended {
                        Self::insert_item_into_folder(
                            &mut self.root_collection.items,
                            &active_node.id,
                            new_item,
                        );
                    } else {
                        Self::insert_sibling_after(
                            &mut self.root_collection.items,
                            &active_node.id,
                            new_item,
                        );
                    }
                }
                NodeType::Request(_) => {
                    Self::insert_sibling_after(
                        &mut self.root_collection.items,
                        &active_node.id,
                        new_item,
                    );
                }
            }
        } else {
            // Tree is completely empty. add to root
            self.root_collection.items.push(new_item);
        }
    }

    pub fn add_new_folder(&mut self, new_folder: Folder) {
        let nodes = self.get_visible_nodes();
        let new_item = CollectionItem::Folder(new_folder);

        if let Some(active_node) = nodes.get(self.selected_node_idx) {
            match &active_node.node_type {
                NodeType::Folder { expended } => {
                    if *expended {
                        Self::insert_item_into_folder(
                            &mut self.root_collection.items,
                            &active_node.id,
                            new_item,
                        );
                    } else {
                        Self::insert_sibling_after(
                            &mut self.root_collection.items,
                            &active_node.id,
                            new_item,
                        );
                    }
                }
                NodeType::Request(_) => {
                    Self::insert_sibling_after(
                        &mut self.root_collection.items,
                        &active_node.id,
                        new_item,
                    );
                }
            }
        } else {
            self.root_collection.items.push(new_item);
        }
    }

    fn insert_item_into_folder(
        items: &mut [CollectionItem],
        target_folder_id: &str,
        new_item: CollectionItem,
    ) -> bool {
        for item in items.iter_mut() {
            if let CollectionItem::Folder(f) = item {
                if f.id == target_folder_id {
                    f.items.push(new_item.clone());
                    return true;
                }
                // Recursively check sub-folders
                if Self::insert_item_into_folder(&mut f.items, target_folder_id, new_item.clone()) {
                    return true;
                }
            }
        }
        false
    }

    /// Recursively finds a request in the tree and updates it
    pub fn update_request_in_tree(&mut self, updated_req: &ApiRequest) {
        Self::update_recursive(&mut self.root_collection.items, updated_req);
    }

    fn update_recursive(items: &mut [CollectionItem], updated_req: &ApiRequest) -> bool {
        for item in items.iter_mut() {
            match item {
                CollectionItem::Request(r) if r.id == updated_req.id => {
                    *r = updated_req.clone();
                    return true;
                }
                CollectionItem::Folder(f) => {
                    if Self::update_recursive(&mut f.items, updated_req) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Recursively finds and deletes a node (Folder or Request)
    pub fn delete_node(&mut self, target_id: &str) {
        Self::delete_recursive(&mut self.root_collection.items, target_id);
    }

    fn delete_recursive(items: &mut Vec<CollectionItem>, target_id: &str) -> bool {
        // Check if the target is in the current list level
        if let Some(pos) = items.iter().position(|i| match i {
            CollectionItem::Request(r) => r.id == target_id,
            CollectionItem::Folder(f) => f.id == target_id,
        }) {
            items.remove(pos);
            return true;
        }

        // Otherwise, dig deeper into sub-folders
        for item in items.iter_mut() {
            if let CollectionItem::Folder(f) = item {
                if Self::delete_recursive(&mut f.items, target_id) {
                    return true;
                }
            }
        }
        false
    }

    pub fn rename_node(&mut self, target_id: &str, new_name: &str) {
        Self::rename_recursive(&mut self.root_collection.items, target_id, new_name);
    }

    fn rename_recursive(items: &mut Vec<CollectionItem>, target_id: &str, new_name: &str) -> bool {
        for item in items.iter_mut() {
            match item {
                CollectionItem::Request(r) if r.id == target_id => {
                    r.name = new_name.to_string();
                    return true;
                }
                CollectionItem::Folder(f) => {
                    if f.id == target_id {
                        f.name = new_name.to_string();
                        return true;
                    }
                    if Self::rename_recursive(&mut f.items, target_id, new_name) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Finds the target node and inserts the new item exactly one slot below it
    fn insert_sibling_after(
        items: &mut Vec<CollectionItem>,
        target_id: &str,
        new_item: CollectionItem,
    ) -> bool {
        // Check if the target exists in the current level of the array
        if let Some(pos) = items.iter().position(|i| match i {
            CollectionItem::Request(r) => r.id == target_id,
            CollectionItem::Folder(f) => f.id == target_id,
        }) {
            // Found it! Insert the new item right below the target
            items.insert(pos + 1, new_item);
            return true;
        }

        // If not found at this level, recursively search inside sub-folders
        for item in items.iter_mut() {
            if let CollectionItem::Folder(f) = item {
                if Self::insert_sibling_after(&mut f.items, target_id, new_item.clone()) {
                    return true;
                }
            }
        }
        false
    }
}
