//! Discuss command — print a leetcode.cn discuss post in the terminal.
use crate::{Error, Result, plugins::LeetCode};
use clap::Args;
use colored::Colorize;
use serde_json::Value;

/// Discuss command arguments
#[derive(Args)]
pub struct DiscussArgs {
    /// Discuss post uuid or full URL
    ///
    /// Examples:
    ///   leetcode discuss RvFUtj
    ///   leetcode discuss https://leetcode.cn/discuss/post/RvFUtj
    #[arg(value_name = "UUID_OR_URL")]
    pub target: String,
}

impl DiscussArgs {
    /// `discuss` handler
    pub async fn run(&self) -> Result<()> {
        let uuid = extract_uuid(&self.target).ok_or_else(|| {
            Error::Anyhow(anyhow::anyhow!(
                "cannot parse discuss uuid from `{}`\n  try: leetcode discuss RvFUtj",
                self.target
            ))
        })?;

        let lc = LeetCode::new()?;
        let resp = lc.get_discuss_post(&uuid).await?;
        let json: Value = resp.json().await?;

        let q = json
            .pointer("/data/qaQuestion")
            .cloned()
            .unwrap_or(Value::Null);
        if q.is_null() {
            if let Some(errs) = json.get("errors") {
                return Err(Error::Anyhow(anyhow::anyhow!(
                    "discuss API error: {}",
                    errs
                )));
            }
            return Err(Error::Anyhow(anyhow::anyhow!(
                "discuss post `{}` not found",
                uuid
            )));
        }

        let title = q
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("(no title)");
        let content = q
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let slug = q.get("slug").and_then(|v| v.as_str()).unwrap_or("");

        println!(
            "\n{} {}\n",
            title.bold().underline(),
            if slug.is_empty() {
                String::new()
            } else {
                format!("({})", slug).dimmed().to_string()
            }
        );

        let rendered = crate::helper::render_markdown(content);
        crate::helper::print_desc_with_images(&rendered.text, &rendered.images);

        // Always print the canonical link at the end.
        let base = crate::config::Config::locate()
            .map(|c| c.sys.urls.base)
            .unwrap_or_else(|_| "https://leetcode.cn".into());
        println!(
            "\n{} {}/discuss/post/{}",
            "链接:".dimmed(),
            base.trim_end_matches('/'),
            uuid
        );

        Ok(())
    }
}

/// Accept raw uuid, path, or full URL.
fn extract_uuid(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    // Full / partial URL: .../discuss/post/<uuid>[/...]
    if let Some(idx) = s.find("/discuss/post/") {
        let rest = &s[idx + "/discuss/post/".len()..];
        let uuid = rest
            .split(|c| c == '/' || c == '?' || c == '#' || c == ' ')
            .next()
            .unwrap_or("")
            .trim();
        if !uuid.is_empty() {
            return Some(uuid.to_string());
        }
    }
    // Bare uuid (alphanumeric, typically 6 chars but don't hardcode length)
    let bare = s
        .trim_matches(|c| c == '/' || c == '?' || c == '#')
        .split(|c| c == '/' || c == '?' || c == '#' || c == ' ')
        .next()
        .unwrap_or("")
        .trim();
    if !bare.is_empty()
        && bare
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Some(bare.to_string());
    }
    None
}
