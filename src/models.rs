use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// -----------------------------------------
// 1. ENVIRONMENT MODELS
// -----------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvVariable {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: Vec<EnvVariable>,
}

// -----------------------------------------
// 2. REQUEST MODELS
// -----------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    HEAD,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum BodyType {
    None,
    RawJson,
    RawText,
    FormData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestBody {
    pub body_type: BodyType,
    pub content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiRequest {
    pub id: String,
    pub name: String,
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: RequestBody,
}

// -----------------------------------------
// 3. COLLECTION MODELS
// -----------------------------------------

/// A Collection can contain direct requests or nested folders
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CollectionItem {
    Request(ApiRequest),
    Folder(Folder),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub items: Vec<CollectionItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub items: Vec<CollectionItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u128,
    pub new_cookies: HashMap<String, String>,
}
