#![cfg(feature = "local-transport")]

use myc_core::domain::dtos::message::Message;

/// Terminal wrap width for the rendered body text.
const WIDTH: usize = 72;

/// Render a stubbed (undelivered) email as a human-friendly, bordered block
/// for the terminal. The gateway's email bodies are full HTML documents
/// (table-based layout, inline styles), which are unreadable when dumped raw
/// to a log; this converts the body to plain text, surfaces every link on its
/// own copyable line, and frames recipient/subject/body so a developer can
/// read a magic-link code straight from stdout.
///
/// Template-agnostic: works for any HTML email (magic-link, password-reset,
/// ...), no per-template special-casing.
pub fn render_stub_email_for_terminal(message: &Message) -> String {
    let rule = "═".repeat(WIDTH);
    let thin = "─".repeat(WIDTH);

    let mut out = String::from("\n");
    out.push_str(&format!("{rule}\n"));
    out.push_str("  STUB EMAIL — not actually delivered\n");
    out.push_str(&format!("{thin}\n"));
    out.push_str(&format!("  To:      {}\n", message.to.email()));
    out.push_str(&format!("  Subject: {}\n", message.subject));
    out.push_str(&render_links_section(&extract_links(&message.body)));
    out.push_str(&format!("{thin}\n"));
    out.push_str(&render_body_section(&message.body));
    out.push_str(&format!("{rule}\n"));

    out
}

/// Render the "Links:" section, or nothing when the body carried no URLs.
fn render_links_section(links: &[String]) -> String {
    let Some(_) = links.first() else {
        return String::new();
    };

    let mut section = String::from("  Links:\n");
    for link in links {
        section.push_str(&format!("    - {link}\n"));
    }

    section
}

/// Render the HTML body as indented plain-text lines. `html2text` right-pads
/// lines to the wrap width; trim that trailing padding for a cleaner block.
fn render_body_section(html: &str) -> String {
    let mut section = String::new();
    for line in html_body_to_text(html).lines() {
        section.push_str(&format!("  {}\n", line.trim_end()));
    }

    section
}

/// Convert an HTML email body into wrapped plain text, falling back to the raw
/// body if the HTML cannot be rendered.
fn html_body_to_text(html: &str) -> String {
    html2text::from_read(html.as_bytes(), WIDTH)
        .unwrap_or_else(|_| html.to_string())
        .trim()
        .to_string()
}

/// Extract every `http(s)` URL from the raw HTML (both `href` attribute values
/// and visible URLs), deduped with insertion order preserved. Hand-rolled to
/// avoid pulling a regex dependency into the notifier.
fn extract_links(html: &str) -> Vec<String> {
    let mut links: Vec<String> = Vec::new();
    let mut cursor = 0;

    while let Some((start, end)) = next_url_span(html, cursor) {
        push_unique(&mut links, &html[start..end]);
        cursor = end.max(start + 1);
    }

    links
}

/// Locate the next `http(s)` URL span at or after `from`, bounded by the first
/// character that cannot belong to a URL.
fn next_url_span(html: &str, from: usize) -> Option<(usize, usize)> {
    let rest = html.get(from..)?;
    // Take the left-most of the two schemes -- preferring one over the other
    // would skip a URL that happens to appear earlier under the other scheme.
    let relative = [rest.find("http://"), rest.find("https://")]
        .into_iter()
        .flatten()
        .min()?;

    let start = from + relative;
    let end = html[start..]
        .find(|c: char| {
            c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | ']')
        })
        .map(|pos| start + pos)
        .unwrap_or(html.len());

    Some((start, end))
}

/// Push a URL only if it is non-empty and not already collected.
fn push_unique(links: &mut Vec<String>, link: &str) {
    let owned = link.to_string();
    if owned.is_empty() || links.contains(&owned) {
        return;
    }

    links.push(owned);
}

#[cfg(test)]
mod tests {
    use super::*;
    use myc_core::domain::dtos::{
        email::Email,
        message::{FromEmail, Message},
    };

    fn sample_message(body: &str) -> Message {
        Message {
            from: FromEmail::Email(
                Email::from_string("noreply@mycelium.com".to_string()).unwrap(),
            ),
            to: Email::from_string("user@mycelium.com".to_string()).unwrap(),
            cc: None,
            subject: "Your Login Link".to_string(),
            body: body.to_string(),
        }
    }

    #[test]
    fn renders_recipient_subject_and_link() {
        let html = r#"<html><body>
            <p>Here's your login link. Click the button below.</p>
            <a href="https://app.mycelium.com/magic-link/abc123">View Login Code</a>
            </body></html>"#;

        let rendered = render_stub_email_for_terminal(&sample_message(html));

        assert!(rendered.contains("user@mycelium.com"));
        assert!(rendered.contains("Your Login Link"));
        assert!(rendered.contains("https://app.mycelium.com/magic-link/abc123"));
        assert!(rendered.contains("not actually delivered"));
    }

    #[test]
    fn strips_html_tags_from_body() {
        let html = "<html><body><p>Plain readable text here.</p></body></html>";

        let rendered = render_stub_email_for_terminal(&sample_message(html));

        assert!(rendered.contains("Plain readable text here."));
        assert!(!rendered.contains("<p>"));
        assert!(!rendered.contains("</body>"));
    }

    #[test]
    fn dedupes_links_preserving_order() {
        let html = r#"<a href="https://a.example/one">x</a>
            <a href="https://a.example/one">y</a>
            <span>https://a.example/two</span>"#;

        let links = extract_links(html);

        assert_eq!(
            links,
            vec![
                "https://a.example/one".to_string(),
                "https://a.example/two".to_string(),
            ]
        );
    }

    #[test]
    fn extracts_links_in_document_order_across_schemes() {
        let html = "visit https://secure.example/a then http://plain.example/b";

        let links = extract_links(html);

        assert_eq!(
            links,
            vec![
                "https://secure.example/a".to_string(),
                "http://plain.example/b".to_string(),
            ]
        );
    }
}
