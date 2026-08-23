use lazy_static::lazy_static;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use regex::{Captures, Regex};
use serde_json::Value;

lazy_static! {
    static ref JSON_HIGHLIGHT_RE: Regex =
        Regex::new(r#"(".*?"\s*:)|(".*?")|(\b\d+\.?\d*\b)|(\b(?:true|false|null)\b)"#)
            .expect("valid json highlight regex");
}

fn prettify_json(raw: &str) -> String {
    match serde_json::from_str::<Value>(raw) {
        Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|_| raw.to_string()),
        Err(_) => raw.to_string(),
    }
}

fn capture_style(cap: &Captures) -> Style {
    if cap.get(1).is_some() {
        Style::default().fg(Color::LightBlue) // Keys
    } else if cap.get(2).is_some() {
        Style::default().fg(Color::LightGreen) // Strings
    } else if cap.get(3).is_some() {
        Style::default().fg(Color::LightYellow) // Numbers
    } else if cap.get(4).is_some() {
        Style::default().fg(Color::LightMagenta) // Booleans/Null
    } else {
        Style::default()
    }
}

fn highlight_line(line: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut last_end = 0;

    for cap in JSON_HIGHLIGHT_RE.captures_iter(line) {
        let m = cap.get(0).unwrap();

        if m.start() > last_end {
            spans.push(Span::raw(line[last_end..m.start()].to_string()));
        }

        let style = capture_style(&cap);
        spans.push(Span::styled(m.as_str().to_string(), style));
        last_end = m.end();
    }

    if last_end < line.len() {
        spans.push(Span::raw(line[last_end..].to_string()));
    }

    Line::from(spans)
}

pub fn format_json_response(raw: &str) -> Text<'static> {
    let pretty = prettify_json(raw);
    let lines: Vec<Line<'static>> = pretty.lines().map(highlight_line).collect();
    Text::from(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prettify_json_valid() {
        let raw = r#"{"name":"test","count":1}"#;
        let formatted = prettify_json(raw);
        assert!(formatted.contains('\n'));
        assert!(formatted.contains(r#""name": "test""#));
    }

    #[test]
    fn test_prettify_json_invalid() {
        let raw = "<html><body>Hello</body></html>";
        let formatted = prettify_json(raw);
        assert_eq!(formatted, raw);
    }

    #[test]
    fn test_format_json_response_all_types() {
        let raw = r#"{
            "string_key": "string_val",
            "number_val": 42.5,
            "bool_true": true,
            "bool_false": false,
            "null_val": null,
            "nested": {
                "arr": [1, 2, "three"]
            }
        }"#;

        let text = format_json_response(raw);
        assert!(!text.lines.is_empty());

        let has_blue = text
            .lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.style.fg == Some(Color::LightBlue)));
        assert!(has_blue, "Expected key styling in blue");

        let has_green = text.lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.style.fg == Some(Color::LightGreen))
        });
        assert!(has_green, "Expected string styling in green");

        let has_yellow = text.lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.style.fg == Some(Color::LightYellow))
        });
        assert!(has_yellow, "Expected number styling in yellow");

        let has_magenta = text.lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.style.fg == Some(Color::LightMagenta))
        });
        assert!(has_magenta, "Expected boolean/null styling in magenta");
    }

    #[test]
    fn test_format_json_response_plain_text() {
        let raw = "plain unformatted text without json";
        let text = format_json_response(raw);
        assert_eq!(text.lines.len(), 1);
    }
}
