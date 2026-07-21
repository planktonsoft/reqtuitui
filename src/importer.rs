use std::collections::HashMap;
use std::str::FromStr;

use uuid::Uuid;

use crate::models::{ApiRequest, BodyType, HttpMethod, RequestBody};

pub fn parse_curl(curl_string: &str) -> Result<ApiRequest, String> {
    let curl_start = curl_string
        .find("curl")
        .ok_or_else(|| "Input does not appear to be a valid curl command".to_string())?;

    let cleaned = curl_string[curl_start..].trim();

    let mut sanitized = String::new();
    let mut chars = cleaned.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&'\n') = chars.peek() {
                chars.next();
                continue;
            } else if let Some(&'\r') = chars.peek() {
                chars.next();
                if let Some(&'\n') = chars.peek() {
                    chars.next();
                }
                continue;
            }
        }

        if c == '\n' || c == '\r' {
            continue;
        }

        sanitized.push(match c {
            '‘' | '’' => '\'',
            '“' | '”' => '"',
            _ => c,
        });
    }

    // curl_parser doesn't support --request or --url properly
    let sanitized = sanitized.replace("--request ", "-X ");
    let sanitized = sanitized.replace("--url ", "");

    // curl_parser doesn't support --cookie or -b properly
    let sanitized = sanitized.replace("--cookie '", "--header 'Cookie: ");
    let sanitized = sanitized.replace("--cookie \"", "--header \"Cookie: ");
    let sanitized = sanitized.replace("-b '", "--header 'Cookie: ");
    let sanitized = sanitized.replace("-b \"", "--header \"Cookie: ");
    // If it's not quoted:
    let sanitized = sanitized.replace("--cookie ", "--header Cookie:");
    let sanitized = sanitized.replace("-b ", "--header Cookie:");

    let parsed = curl_parser::ParsedRequest::from_str(&sanitized)
        .map_err(|e| format!("Failed to parse curl: {}", e))?;

    let http_method = match parsed.method.as_str() {
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "DELETE" => HttpMethod::DELETE,
        "PATCH" => HttpMethod::PATCH,
        "GET" => HttpMethod::GET,
        _ => HttpMethod::GET, // default
    };

    let mut headers = HashMap::new();
    for (name, value) in parsed.headers.iter() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.to_string(), v.to_string());
        }
    }

    let req_body = if !parsed.body.is_empty() {
        let content = parsed.body.join("&");
        RequestBody {
            body_type: BodyType::RawJson,
            content: Some(content),
        }
    } else {
        RequestBody {
            body_type: BodyType::None,
            content: None,
        }
    };

    Ok(ApiRequest {
        id: Uuid::new_v4().to_string(),
        name: "Imported from cURL".to_string(),
        url: parsed.url.to_string(),
        method: http_method,
        headers,
        query_params: HashMap::new(),
        body: req_body,
        pre_script: None,
        post_script: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_curl_input() {
        let curl_string = r#"curl --request POST \
  --url https://localhost:8080/v1/login \
  --header 'accept: */*' \
  --header 'content-type: application/json' \
  --header 'cookie: token_a=.mock_token_a; token_b=mock_token_b' \
  --cookie 'token_a=.mock_token_a; token_b=mock_token_b' \
  --data '{
  "user_id": "MOCK-USER-ID-1234-5678",
  "customer_id": "MOCK_CUSTOMER_ID",
  "code": "MOCK_CODE"
}'"#;
        let res = parse_curl(curl_string);
        println!("{:?}", res);
        assert!(res.is_ok());

        let api_req = res.unwrap();
        // It should have cookie header with merged or multiple cookies if the crate handles it,
        // or at least not fail to parse.
        let has_cookie = api_req.headers.keys().any(|k| k.to_lowercase() == "cookie");
        assert!(has_cookie);
    }
}
