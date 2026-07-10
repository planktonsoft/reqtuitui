use std::{collections::HashMap, str::FromStr, time::Instant};

use reqwest::{
    Client, Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};

use crate::models::{ApiRequest, ApiResponse, BodyType, Environment};
use crate::parser::TemplateParser;
pub struct HttpManager {
    client: Client,
    parser: TemplateParser,
}

impl HttpManager {
    pub fn new() -> Self {
        Self {
            // A single client instance handles connection pooling
            // TODO: Add a configuration option to allow invalid SSL certificates.
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

        // Map your custom HttpMethod to reqwest's Method
        let method = match req_data.method {
            crate::models::HttpMethod::GET => Method::GET,
            crate::models::HttpMethod::POST => Method::POST,
            _ => Method::GET,
        };

        // Convert HashMap headers to reqwest HeaderMap
        let mut headers = HeaderMap::new();
        for (k, v) in req_data.headers {
            let parsed_key = self.parser.parse_string(&k, active_env);
            let parsed_val = self.parser.parse_string(&v, active_env);

            if let (Ok(name), Ok(value)) = (
                HeaderName::from_str(&parsed_key),
                HeaderValue::from_str(&parsed_val),
            ) {
                headers.insert(name, value);
            }
        }

        // Build and send the request
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

        // Extract response body
        let body_text = response.text().await.unwrap_or_default();

        Ok(ApiResponse {
            status_code,
            headers: HashMap::new(),
            body: body_text,
            duration_ms,
        })
    }
}
