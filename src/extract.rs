/// Extract cross-references from docstring content.
///
/// Supports MkDocs/mkdocstrings Markdown syntax and Sphinx RST syntax.
use crate::config::DocStyle;
use regex::Regex;
use std::sync::LazyLock;

/// A cross-reference found in a docstring.
#[derive(Debug, Clone)]
pub struct Reference {
    /// The dotted path being referenced (e.g. `my_pkg.foo.Bar`).
    pub target: String,
    /// Byte offset within the docstring where this reference starts.
    pub offset: usize,
}

// ---------------------------------------------------------------------------
// MkDocs patterns
// ---------------------------------------------------------------------------

// [display text][identifier]
static MKDOCS_EXPLICIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[[^\]]*\]\[([a-zA-Z_][\w.]*)\]").unwrap()
});

// [identifier][]  (autoref shorthand)
static MKDOCS_AUTOREF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[([a-zA-Z_][\w.]*)\]\[\]").unwrap()
});

// ---------------------------------------------------------------------------
// Sphinx patterns
// ---------------------------------------------------------------------------

// :role:`~optional.dotted.path`
static SPHINX_XREF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r":(class|func|meth|mod|attr|exc|data|obj|const|type):`~?([^`]+)`").unwrap()
});

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check if a reference looks like a fully-qualified Python path.
///
/// Must contain a dot and start with a lowercase letter or underscore
/// (package names are lowercase; `ClassName.method` is a scoped ref, not FQN).
fn is_fully_qualified(s: &str) -> bool {
    s.contains('.') && s.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
}

/// Extract all cross-references from a docstring's content.
pub fn extract_references(content: &str, style: &DocStyle) -> Vec<Reference> {
    match style {
        DocStyle::Mkdocs => extract_mkdocs(content),
        DocStyle::Sphinx => extract_sphinx(content),
        DocStyle::Auto => {
            let mut refs = extract_mkdocs(content);
            refs.extend(extract_sphinx(content));
            refs
        }
    }
}

fn extract_mkdocs(content: &str) -> Vec<Reference> {
    let mut refs = Vec::new();

    for cap in MKDOCS_EXPLICIT.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            if is_fully_qualified(m.as_str()) {
                refs.push(Reference {
                    target: m.as_str().to_string(),
                    offset: m.start(),
                });
            }
        }
    }

    for cap in MKDOCS_AUTOREF.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            if is_fully_qualified(m.as_str()) {
                refs.push(Reference {
                    target: m.as_str().to_string(),
                    offset: m.start(),
                });
            }
        }
    }

    refs
}

fn extract_sphinx(content: &str) -> Vec<Reference> {
    let mut refs = Vec::new();

    for cap in SPHINX_XREF.captures_iter(content) {
        if let Some(m) = cap.get(2) {
            let target = m.as_str().trim();
            // Strip leading `~` if present (display hint, not part of path).
            let target = target.strip_prefix('~').unwrap_or(target);
            if is_fully_qualified(target) {
                refs.push(Reference {
                    target: target.to_string(),
                    offset: m.start(),
                });
            }
        }
    }

    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mkdocs_explicit() {
        let content = r#"See [MyClass][my_pkg.foo.MyClass] for details."#;
        let refs = extract_references(content, &DocStyle::Mkdocs);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target, "my_pkg.foo.MyClass");
    }

    #[test]
    fn test_mkdocs_autoref() {
        let content = r#"See [my_pkg.foo.MyClass][] for details."#;
        let refs = extract_references(content, &DocStyle::Mkdocs);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target, "my_pkg.foo.MyClass");
    }

    #[test]
    fn test_sphinx_class() {
        let content = r#"See :class:`my_pkg.foo.MyClass` for details."#;
        let refs = extract_references(content, &DocStyle::Sphinx);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target, "my_pkg.foo.MyClass");
    }

    #[test]
    fn test_sphinx_tilde() {
        let content = r#"See :func:`~my_pkg.foo.bar` for details."#;
        let refs = extract_references(content, &DocStyle::Sphinx);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target, "my_pkg.foo.bar");
    }

    #[test]
    fn test_auto_finds_both() {
        let content = r#"
        See [my_pkg.foo.A][] and :class:`my_pkg.bar.B`.
        "#;
        let refs = extract_references(content, &DocStyle::Auto);
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_ignores_non_dotted_mkdocs() {
        let content = r#"See [details][section-link] for more."#;
        let refs = extract_references(content, &DocStyle::Mkdocs);
        assert_eq!(refs.len(), 0);
    }
}
