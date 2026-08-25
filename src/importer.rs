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

    // Unquote HTTP methods for -X and --request (curl-parser expects unquoted methods like -X PUT)
    let method_regex = regex::Regex::new(r#"(?i)(?:-X|--request)\s+['"]([A-Za-z]+)['"]"#).unwrap();
    let sanitized = method_regex.replace_all(&sanitized, "-X $1").to_string();

    // curl_parser doesn't support --request or --url properly
    let sanitized = sanitized.replace("--request ", "-X ");
    let sanitized = sanitized.replace("--url ", "");

    // curl_parser doesn't support --data-raw, --data-binary, --data-ascii, --data-urlencode
    let sanitized = sanitized.replace("--data-raw ", "--data ");
    let sanitized = sanitized.replace("--data-binary ", "--data ");
    let sanitized = sanitized.replace("--data-ascii ", "--data ");
    let sanitized = sanitized.replace("--data-urlencode ", "--data ");

    // Remove flags not supported by curl_parser
    let mut sanitized = sanitized.replace("--compressed ", " ");
    sanitized = sanitized.replace("--compressed", " ");

    let flag_regex = regex::Regex::new(
        r#"(?i)(?:\s|^)(?:--silent|--verbose|--show-error|--include|-[sSvi])(?:\s|$)"#,
    )
    .unwrap();
    while flag_regex.is_match(&sanitized) {
        sanitized = flag_regex.replace_all(&sanitized, " ").to_string();
    }

    // curl_parser doesn't support --cookie or -b properly
    let sanitized = sanitized.replace("--cookie '", "--header 'Cookie: ");
    let sanitized = sanitized.replace("--cookie \"", "--header \"Cookie: ");
    let sanitized = sanitized.replace("-b '", "--header 'Cookie: ");
    let sanitized = sanitized.replace("-b \"", "--header \"Cookie: ");
    // If it's not quoted:
    let sanitized = sanitized.replace("--cookie ", "--header Cookie:");
    let sanitized = sanitized.replace("-b ", "--header Cookie:");

    // Handle header ending in semicolon (e.g. -H 'x-refresh-token;' or -H "x-refresh-token;" or -H x-refresh-token;)
    // In curl, trailing semicolon sends an empty header without a value.
    let header_single_quote_regex =
        regex::Regex::new(r#"(?i)(-H|--header)\s+'([^'":;]+);\s*'"#).unwrap();
    let sanitized = header_single_quote_regex
        .replace_all(&sanitized, "$1 '$2:'")
        .to_string();

    let header_double_quote_regex =
        regex::Regex::new(r#"(?i)(-H|--header)\s+"([^'":;]+);\s*""#).unwrap();
    let sanitized = header_double_quote_regex
        .replace_all(&sanitized, "$1 \"$2:\"")
        .to_string();

    let header_unquoted_regex = regex::Regex::new(r#"(?i)(-H|--header)\s+([^'":;\s]+);"#).unwrap();
    let sanitized = header_unquoted_regex
        .replace_all(&sanitized, "$1 '$2:'")
        .to_string();

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
    }

    #[test]
    fn test_parse_user_curl() {
        let curl_string = r#"curl --url 'https://api.example.com/v1/users' \
  -X 'PUT' \
  -H 'accept: application/json' \
  -H 'accept-language: en-US,en;q=0.9' \
  -H 'authorization: Bearer' \
  -H 'content-type: application/json' \
  -b 'access_token=mock_access_token_jwt; refresh_token=mock_refresh_token_jwt; authStore={%22state%22:{%22token%22:%22mockToken%22}%2C%22version%22:0}' \
  -H 'origin: https://api.example.com' \
  -H 'priority: u=1, i' \
  -H 'referer: https://api.example.com/settings' \
  -H 'sec-ch-ua: "Not=A?Brand";v="99", "Google Chrome";v="151", "Chromium";v="151"' \
  -H 'sec-ch-ua-mobile: ?0' \
  -H 'sec-ch-ua-platform: "macOS"' \
  -H 'sec-fetch-dest: empty' \
  -H 'sec-fetch-mode: cors' \
  -H 'sec-fetch-site: same-origin' \
  -H 'user-agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36' \
  -H 'x-refresh-token;' \
  --data-raw '{"email":"[EMAIL_ADDRESS]","full_name":"John Doe","user_id":"MOCK-USER-ID-1234-5678","role":"admin"}' \
  --insecure"#;
        let res = parse_curl(curl_string);
        assert!(res.is_ok(), "parse_curl error: {:?}", res.err());
        let req = res.unwrap();
        assert_eq!(req.method, HttpMethod::PUT);
        assert_eq!(req.url, "https://api.example.com/v1/users");
        assert!(req.headers.contains_key("cookie") || req.headers.contains_key("Cookie"));
        assert!(
            req.headers.contains_key("x-refresh-token")
                || req.headers.contains_key("X-Refresh-Token")
        );
        assert!(req.body.content.is_some());
    }

    #[test]
    fn test_parse_curl_edge_cases() {
        let curl = r#"curl -X "PATCH" 'https://example.com/api' \
            --compressed \
            -s -v \
            --data-binary '{"key":"val"}' \
            -H custom-empty;"#;
        let res = parse_curl(curl);
        assert!(res.is_ok(), "edge case failed: {:?}", res.err());
        let req = res.unwrap();
        assert_eq!(req.method, HttpMethod::PATCH);
        assert!(req.headers.contains_key("custom-empty"));
    }
}
