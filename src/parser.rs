use std::collections::HashMap;

use regex::Regex;

use crate::models::Environment;

pub struct TemplateParser {
    // We store the compiled regex so we don't recompile it on every keystroke
    matcher: Regex,
}

impl TemplateParser {
    pub fn new() -> Self {
        Self {
            // This regex matches exactly two curly braces, capturing the text inside non-greedily
            matcher: Regex::new(r"\{\{(.+?)\}\}").expect("Failed to compile regex"),
        }
    }

    /// Takes a raw string (like a URL or Body) and injects the environment variables.
    pub fn parse_string(&self, raw_text: &str, env: Option<&Environment>) -> String {
        // If no environment is active, just return the raw text
        let env = match env {
            Some(e) => e,
            None => return raw_text.to_string(),
        };

        // Convert the environment into a quick lookup table (HashMap)
        // We only map variables that are toggled "enabled"
        let mut lookup = HashMap::new();
        for var in &env.variables {
            if var.enabled {
                lookup.insert(var.key.as_str(), var.value.as_str());
            }
        }

        // Search and replace!
        let parsed = self
            .matcher
            .replace_all(raw_text, |caps: &regex::Captures| {
                // cap[0] is the full match: "{{base_url}}"
                // cap[1] is the inner group: "base_url"
                let key = caps[1].trim();

                // If the key exists in our environment. inject the value.
                // If it doesn't exist. leave it as {{key}} so the user knows it's missing.
                match lookup.get(key) {
                    Some(val) => val.to_string(),
                    None => caps[0].to_string(),
                }
            });

        parsed.into_owned()
    }
}
