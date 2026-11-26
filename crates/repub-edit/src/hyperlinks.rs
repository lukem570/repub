//! Following hyperlinks: where a link goes (a web or mail address, or a page
//! of the publication) and opening an address with the system handler.
//!
//! Finding the link under a pointer requires laid-out text, which is a
//! rendering concern; the caller supplies the resolved [`Hyperlink`] here.

use repub_model::text::Hyperlink;

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
