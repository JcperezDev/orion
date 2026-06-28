use crate::tools::{ApprovalRequest, ApprovalResponse, PermissionKind, Tool, ToolContext, ToolResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Node, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_TIMEOUT_SECS: u64 = 120;
const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

pub struct WebFetchTool {
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct WebFetchArgs {
    url: String,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

fn default_format() -> String {
    "markdown".to_string()
}

impl WebFetchTool {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(MAX_TIMEOUT_SECS + 10))
            .user_agent(BROWSER_UA)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap_or_default();
        Self { client }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "webfetch"
    }

    fn description(&self) -> &str {
        "Fetch content from an HTTP or HTTPS URL and return it as text, markdown, or HTML. Markdown is the default."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The HTTP or HTTPS URL to fetch content from"
                },
                "format": {
                    "type": "string",
                    "enum": ["text", "markdown", "html"],
                    "description": "The format to return content in. Defaults to markdown."
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Optional timeout in seconds (maximum: 120)"
                }
            },
            "required": ["url"]
        })
    }

    fn requires_permission(&self) -> PermissionKind {
        PermissionKind::Network
    }

    fn action_summary(&self, args: &serde_json::Value) -> String {
        args.get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("webfetch")
            .to_string()
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<ToolResult> {
        let args: WebFetchArgs =
            serde_json::from_value(args).context("invalid args for webfetch tool")?;

        let request = ApprovalRequest {
            tool_name: "webfetch".into(),
            action: format!("fetch {}", args.url),
            matched_pattern: None,
            arguments: serde_json::to_value(&args).unwrap_or_default(),
        };
        match ctx.ask(request).await {
            ApprovalResponse::Deny => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: "denied by user".into(),
                    is_error: true,
                });
            }
            ApprovalResponse::Allow | ApprovalResponse::AllowAlways => {}
        }

        let parsed = url::Url::parse(&args.url).context("invalid URL")?;
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                content: "URL must use http:// or https://".into(),
                is_error: true,
            });
        }

        let timeout =
            Duration::from_secs(args.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS).min(MAX_TIMEOUT_SECS));
        let format = args.format.as_str();

        let result = tokio::time::timeout(timeout + Duration::from_secs(5), async {
            let response = self.client.get(&args.url).send().await?;
            let status = response.status();
            if !status.is_success() {
                return Ok::<_, anyhow::Error>(ToolResult {
                    tool_call_id: String::new(),
                    content: format!("HTTP {status}: {}", response.text().await.unwrap_or_default()),
                    is_error: true,
                });
            }

            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            let mime = content_type.split(';').next().unwrap_or("").trim().to_lowercase();

            let bytes = response.bytes().await.context("failed to read response body")?;

            if bytes.len() > MAX_RESPONSE_BYTES {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    content: format!(
                        "Response too large ({} bytes exceeds {} byte limit)",
                        bytes.len(),
                        MAX_RESPONSE_BYTES
                    ),
                    is_error: true,
                });
            }

            let body = String::from_utf8_lossy(&bytes).to_string();

            let output = if !mime.is_empty() && mime != "text/html" && !mime.starts_with("text/") && mime != "application/json" && !mime.ends_with("+json") && !mime.ends_with("+xml") {
                format!("[Fetched {} — {} bytes — use format=html to see raw content]", mime, bytes.len())
            } else if format == "html" {
                body
            } else {
                match format {
                    "text" => html_to_text(&body),
                    "markdown" | _ => html_to_markdown(&body),
                }
            };

            Ok(ToolResult {
                tool_call_id: String::new(),
                content: format!("URL: {}\nContent-Type: {}\n\n{}", args.url, content_type, output),
                is_error: false,
            })
        }).await;

        match result {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(e)) => Ok(ToolResult {
                tool_call_id: String::new(),
                content: format!("fetch error: {e}"),
                is_error: true,
            }),
            Err(_) => Ok(ToolResult {
                tool_call_id: String::new(),
                content: format!("timeout after {timeout:?}"),
                is_error: true,
            }),
        }
    }
}

fn html_to_text(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut text = String::new();
    let script_sel = Selector::parse("script, style, noscript, iframe").ok();

    for node in document.root_element().descendants() {
        let skip = script_sel.as_ref().is_some_and(|sel| {
            node.ancestors().any(|a| {
                a.value().is_element() && {
                    let _elem = a.value().as_element().unwrap();
                    sel.matches(&scraper::ElementRef::wrap(a).unwrap())
                }
            })
        });
        if skip {
            continue;
        }
        if let Some(txt) = node.value().as_text() {
            let trimmed = txt.text.trim();
            if !trimmed.is_empty() {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push(' ');
                }
                text.push_str(trimmed);
            }
        }
    }
    text
}

fn html_to_markdown(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut md = String::new();
    element_to_markdown(&document.root_element(), &mut md, 0);
    let result = md.trim().to_string();
    if result.is_empty() { html_to_text(html) } else { result }
}

fn element_to_markdown(el: &scraper::ElementRef, out: &mut String, depth: usize) {
    for child in el.children() {
        match child.value() {
            Node::Text(txt) => {
                let text = txt.text.trim();
                if !text.is_empty() {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push(' ');
                    }
                    out.push_str(text);
                }
            }
            Node::Element(child_el) => {
                let tag = child_el.name.local.as_ref();
                let child_ref = scraper::ElementRef::wrap(child).unwrap();
                match tag {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        let level = tag[1..].parse::<usize>().unwrap_or(1);
                        ensure_newline(out);
                        out.push_str(&format!("{} ", "#".repeat(level)));
                        element_to_markdown(&child_ref, out, depth + 1);
                        out.push('\n');
                    }
                    "p" | "div" | "section" | "article" => {
                        ensure_newline(out);
                        element_to_markdown(&child_ref, out, depth + 1);
                        out.push('\n');
                    }
                    "blockquote" => {
                        ensure_newline(out);
                        out.push_str("> ");
                        element_to_markdown(&child_ref, out, depth + 1);
                        out.push('\n');
                    }
                    "br" => out.push('\n'),
                    "hr" => {
                        ensure_newline(out);
                        out.push_str("---\n");
                    }
                    "ul" | "ol" => {
                        ensure_newline(out);
                        let is_ol = tag == "ol";
                        let mut idx = 1;
                        for li_child in child.children() {
                            if let Some(li_ref) = scraper::ElementRef::wrap(li_child) {
                                if li_ref.value().name.local.as_ref() == "li" {
                                    let bullet = if is_ol {
                                        let n = idx; idx += 1;
                                        format!("{}. ", n)
                                    } else {
                                        "- ".to_string()
                                    };
                                    out.push_str(&bullet);
                                    element_to_markdown(&li_ref, out, depth + 1);
                                    out.push('\n');
                                }
                            }
                        }
                    }
                    "a" => {
                        let href = child_el.attr("href").unwrap_or("");
                        let mut link_text = String::new();
                        element_to_markdown(&child_ref, &mut link_text, depth + 1);
                        let lt = link_text.trim();
                        if !lt.is_empty() && !href.is_empty() {
                            out.push_str(&format!("[{}]({})", lt, href));
                        } else if !lt.is_empty() {
                            out.push_str(lt);
                        } else if !href.is_empty() {
                            out.push_str(href);
                        }
                    }
                    "img" => {
                        let src = child_el.attr("src").unwrap_or("");
                        let alt = child_el.attr("alt").unwrap_or("");
                        out.push_str(&format!("![{}]({})", alt, src));
                    }
                    "code" => {
                        out.push('`');
                        element_to_markdown(&child_ref, out, depth + 1);
                        out.push('`');
                    }
                    "pre" => {
                        let code_sel = Selector::parse("code").ok();
                        let lang = code_sel
                            .and_then(|sel| child_ref.select(&sel).next())
                            .and_then(|c| c.value().attr("class"))
                            .unwrap_or("")
                            .trim_start_matches("language-");
                        out.push_str(&format!("\n```{}\n", lang));
                        element_to_markdown(&child_ref, out, depth + 1);
                        if !out.ends_with('\n') { out.push('\n'); }
                        out.push_str("```\n");
                    }
                    "strong" | "b" => {
                        out.push_str("**");
                        element_to_markdown(&child_ref, out, depth + 1);
                        out.push_str("**");
                    }
                    "em" | "i" => {
                        out.push('*');
                        element_to_markdown(&child_ref, out, depth + 1);
                        out.push('*');
                    }
                    "script" | "style" | "noscript" | "iframe" | "nav" | "footer" => {}
                    _ => element_to_markdown(&child_ref, out, depth + 1),
                }
            }
            _ => {}
        }
    }
}

fn ensure_newline(out: &mut String) {
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_markdown_simple() {
        let html = "<h1>Hello</h1><p>World</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("Hello"));
        assert!(md.contains("World"));
    }

    #[test]
    fn html_to_text_simple() {
        let html = "<p>Hello <b>World</b></p>";
        let text = html_to_text(html);
        assert!(text.contains("Hello"));
    }

    #[test]
    fn html_to_markdown_link() {
        let html = r#"<a href="https://example.com">click here</a>"#;
        let md = html_to_markdown(html);
        assert!(md.contains("[click here](https://example.com)"));
    }
}
