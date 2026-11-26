//! Following hyperlinks: finding the link under the pointer in laid-out text,
//! and where a link goes (a web or mail address, or a page of the publication).

use repub_model::shape::{Shape, ShapeKind};
use repub_model::text::{Hyperlink, InlineElement};

use crate::sdf_text;
use crate::text_edit::{element_len, to_frame_space};

/// The link drawn at document point (`x`, `y`) in text box `tf` (as laid out
/// and shown), if any. Only clicks on the link's characters count.
pub fn link_at(tf: &Shape, x: f32, y: f32) -> Option<Hyperlink> {
    let ShapeKind::TextFrame(data) = &tf.kind else {
        return None;
    };
    let (fx, fy) = to_frame_space(tf, x, y);
    let layout = sdf_text::layout_frame(tf);
    let line = layout
        .lines
        .iter()
        .find(|l| fy >= l.top && fy <= l.bottom)?;
    // The character whose box holds x
    let i = line
        .xs
        .windows(2)
        .position(|w| fx >= w[0].min(w[1]) && fx < w[0].max(w[1]))?;
    let offset = line.start + i;
    let para = data.content.paragraphs.get(line.para)?;
    let mut at = 0;
    for element in &para.elements {
        let len = element_len(element);
        if offset < at + len {
            return match element {
                InlineElement::Link(link) => Some(link.clone()),
                _ => None,
            };
        }
        at += len;
    }
    None
}

/// Where a link goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// A web, mail or FTP address, opened by the system
    Address(String),
    /// A page of this publication (0-based)
    Page(usize),
    /// Nothing that can be opened
    Unusable(String),
}

/// What following `link` does. Addresses without a scheme ("www.example.com")
/// are taken as web addresses; internal links name a page ("3", "Page 3",
/// "page:3", "#3").
pub fn destination(link: &Hyperlink) -> Destination {
    let target = link.target.trim();
    let lower = target.to_ascii_lowercase();
    if !link.external {
        let digits: String = lower
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return match digits.parse::<usize>() {
            Ok(n) if n >= 1 => Destination::Page(n - 1),
            _ => Destination::Unusable(target.to_owned()),
        };
    }
    if ["http://", "https://", "mailto:", "ftp://"]
        .iter()
        .any(|s| lower.starts_with(s))
    {
        return Destination::Address(target.to_owned());
    }
    // Anything else with a scheme (file:, javascript:, ...) isn't opened
    if lower.contains("://")
        || lower
            .split_once(':')
            .is_some_and(|(s, _)| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic()))
    {
        return Destination::Unusable(target.to_owned());
    }
    if target.contains('@') && !target.contains('/') {
        return Destination::Address(format!("mailto:{target}"));
    }
    if target.contains('.') && !target.contains(' ') {
        return Destination::Address(format!("https://{target}"));
    }
    Destination::Unusable(target.to_owned())
}

/// Open `address` in the system's browser or mail app.
pub fn open_address(address: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");
    command.arg(address).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(target: &str, external: bool) -> Hyperlink {
        let mut l = Hyperlink::external(target, "x");
        l.external = external;
        l
    }

    #[test]
    fn destinations() {
        assert_eq!(
            destination(&link("https://example.com/a", true)),
            Destination::Address("https://example.com/a".into())
        );
        assert_eq!(
            destination(&link("www.example.com", true)),
            Destination::Address("https://www.example.com".into())
        );
        assert_eq!(
            destination(&link("me@example.com", true)),
            Destination::Address("mailto:me@example.com".into())
        );
        assert_eq!(destination(&link("Page 3", false)), Destination::Page(2));
        assert!(matches!(
            destination(&link("file:///etc/passwd", true)),
            Destination::Unusable(_)
        ));
        assert!(matches!(
            destination(&link("javascript:alert(1)", true)),
            Destination::Unusable(_)
        ));
    }
}
