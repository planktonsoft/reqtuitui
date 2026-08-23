use std::{collections::HashMap, str::FromStr, time::Instant};

use reqwest::{
    Client, Method,
    header::{HeaderMap, HeaderName, HeaderValue, SET_COOKIE},
};

use crate::models::{ApiRequest, ApiResponse, BodyType, Environment, HttpMethod};
use crate::parser::TemplateParser;

pub struct HttpManager {
    client: Client,
    parser: TemplateParser,
}

fn map_method(method: HttpMethod) -> Method {
    match method {
        HttpMethod::GET => Method::GET,
        HttpMethod::POST => Method::POST,
        HttpMethod::PUT => Method::PUT,
        HttpMethod::DELETE => Method::DELETE,
        HttpMethod::PATCH => Method::PATCH,
        HttpMethod::OPTIONS => Method::OPTIONS,
        HttpMethod::HEAD => Method::HEAD,
    }
}

fn default_content_type(body_type: &BodyType) -> Option<&'static str> {
    match body_type {
        BodyType::RawJson => Some("application/json"),
        BodyType::RawText => Some("text/plain"),
        BodyType::FormData => Some("application/x-www-form-urlencoded"),
        BodyType::None => None,
    }
}

fn build_headers(
    raw_headers: HashMap<String, String>,
    body_type: &BodyType,
    parser: &TemplateParser,
    active_env: Option<&Environment>,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let mut has_content_type = false;

    for (k, v) in raw_headers {
        let parsed_key = parser.parse_string(&k, active_env);
        let parsed_val = parser.parse_string(&v, active_env);

        if parsed_key.eq_ignore_ascii_case("content-length") {
            continue;
        }

        if parsed_key.eq_ignore_ascii_case("content-type") {
            has_content_type = true;
        }

        if let (Ok(name), Ok(value)) = (
            HeaderName::from_str(&parsed_key),
            HeaderValue::from_str(&parsed_val),
        ) {
            headers.insert(name, value);
        }
    }

    if !has_content_type {
        if let Some(ct) = default_content_type(body_type) {
            if let Ok(value) = HeaderValue::from_str(ct) {
                headers.insert(reqwest::header::CONTENT_TYPE, value);
            }
        }
    }

    headers
}

fn extract_cookies(headers: &HeaderMap) -> HashMap<String, String> {
    let mut extracted_cookies = HashMap::new();
    for cookie_header in headers.get_all(SET_COOKIE) {
        if let Ok(c_str) = cookie_header.to_str() {
            if let Some(first_part) = c_str.split(';').next() {
                if let Some((k, v)) = first_part.split_once('=') {
                    extracted_cookies.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
    }
    extracted_cookies
}

impl HttpManager {
    pub fn new() -> Self {
        Self {
            // A single client instance handles connection pooling
            client: Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap_or_default(),
            parser: TemplateParser::new(),
        }
    }

    pub async fn execute(
        &self,
        req_data: ApiRequest,
        active_env: Option<&Environment>,
    ) -> Result<ApiResponse, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let parsed_url = self.parser.parse_string(&req_data.url, active_env);
        let method = map_method(req_data.method);
        let headers = build_headers(
            req_data.headers,
            &req_data.body.body_type,
            &self.parser,
            active_env,
        );

        let mut request_builder = self.client.request(method, parsed_url).headers(headers);

        if req_data.body.body_type != BodyType::None {
            if let Some(raw_body) = req_data.body.content {
                let parsed_body = self.parser.parse_string(&raw_body, active_env);
                request_builder = request_builder.body(parsed_body);
            }
        }

        let response = request_builder.send().await?;
        let duration_ms = start_time.elapsed().as_millis();
        let status_code = response.status().as_u16();
        let new_cookies = extract_cookies(response.headers());

        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read response body".to_string());

        Ok(ApiResponse {
            status_code,
            headers: HashMap::new(),
            body: body_text,
            duration_ms,
            new_cookies,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BodyType, HttpMethod, RequestBody};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn test_map_method() {
        assert_eq!(map_method(HttpMethod::GET), Method::GET);
        assert_eq!(map_method(HttpMethod::POST), Method::POST);
        assert_eq!(map_method(HttpMethod::PUT), Method::PUT);
        assert_eq!(map_method(HttpMethod::DELETE), Method::DELETE);
        assert_eq!(map_method(HttpMethod::PATCH), Method::PATCH);
        assert_eq!(map_method(HttpMethod::OPTIONS), Method::OPTIONS);
        assert_eq!(map_method(HttpMethod::HEAD), Method::HEAD);
    }

    #[test]
    fn test_default_content_type() {
        assert_eq!(
            default_content_type(&BodyType::RawJson),
            Some("application/json")
        );
        assert_eq!(default_content_type(&BodyType::RawText), Some("text/plain"));
        assert_eq!(
            default_content_type(&BodyType::FormData),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(default_content_type(&BodyType::None), None);
    }

    #[test]
    fn test_build_headers_default_and_custom() {
        let parser = TemplateParser::new();
        let mut raw = HashMap::new();
        raw.insert("Content-Length".to_string(), "100".to_string());
        raw.insert("X-Custom".to_string(), "value".to_string());
        raw.insert(
            "invalid header name with spaces".to_string(),
            "val".to_string(),
        );

        let headers = build_headers(raw, &BodyType::RawJson, &parser, None);
        assert!(!headers.contains_key("content-length"));
        assert_eq!(headers.get("x-custom").unwrap(), "value");
        assert_eq!(headers.get("content-type").unwrap(), "application/json");

        // When Content-Type is explicitly provided
        let mut explicit_raw = HashMap::new();
        explicit_raw.insert("content-type".to_string(), "application/xml".to_string());
        let headers_explicit = build_headers(explicit_raw, &BodyType::RawJson, &parser, None);
        assert_eq!(
            headers_explicit.get("content-type").unwrap(),
            "application/xml"
        );

        // When body type is None
        let headers_none = build_headers(HashMap::new(), &BodyType::None, &parser, None);
        assert!(!headers_none.contains_key("content-type"));
    }

    #[test]
    fn test_extract_cookies() {
        let mut headers = HeaderMap::new();
        headers.append(
            SET_COOKIE,
            HeaderValue::from_str("session_id=12345; Path=/; HttpOnly").unwrap(),
        );
        headers.append(
            SET_COOKIE,
            HeaderValue::from_str("theme=dark; Secure").unwrap(),
        );

        let cookies = extract_cookies(&headers);
        assert_eq!(cookies.get("session_id"), Some(&"12345".to_string()));
        assert_eq!(cookies.get("theme"), Some(&"dark".to_string()));
    }

    #[tokio::test]
    async fn test_http_manager_execute_success() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let response = "HTTP/1.1 200 OK\r\nSet-Cookie: user=bob; Path=/\r\nContent-Length: 13\r\n\r\nHello, World!";
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });

        let manager = HttpManager::new();
        let req = ApiRequest {
            id: "1".to_string(),
            name: "Test".to_string(),
            url: format!("http://{}", addr),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: RequestBody {
                body_type: BodyType::RawText,
                content: Some("ping".to_string()),
            },
            pre_script: None,
            post_script: None,
        };

        let result = manager.execute(req, None).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body, "Hello, World!");
        assert_eq!(resp.new_cookies.get("user"), Some(&"bob".to_string()));
    }
}
