/// Fast line-based scanner for Python files without docstrings.
///
/// Extracts imports and top-level class/function names without
/// tree-sitter parsing. Used as a fast path when a file has no
/// triple-quoted strings (and thus no docstrings to check).
use crate::graph::{Import, Module, Symbol, SymbolKind};
use std::collections::HashMap;
use std::path::Path;

/// Check if source bytes contain a triple-quoted string.
pub fn has_docstrings(source: &[u8]) -> bool {
    source.windows(3).any(|w| w == b"\"\"\"" || w == b"'''")
}

/// Fast-scan a Python file to extract imports and top-level definitions.
/// Does NOT extract docstrings, class members, bases, or attributes.
pub fn fast_scan(source: &[u8], file_path: &Path, dotted_path: &str) -> Module {
    let is_package = file_path.file_name().is_some_and(|f| f == "__init__.py");

    let mut module = Module {
        path: dotted_path.to_string(),
        file_path: file_path.display().to_string(),
        is_package,
        definitions: HashMap::new(),
        imports: Vec::new(),
        all: None,
        docstrings: Vec::new(),
    };

    // Process line by line. We only handle unindented (top-level) statements.
    for line in source.split(|&b| b == b'\n') {
        let trimmed = strip_leading_spaces(line);
        if trimmed.is_empty() || trimmed[0] == b'#' {
            continue;
        }

        // Top-level only: line must start at column 0 (no indentation).
        if line.first().is_some_and(|b| *b == b' ' || *b == b'\t') {
            continue;
        }

        if trimmed.starts_with(b"from ") {
            scan_from_import(trimmed, &mut module);
        } else if trimmed.starts_with(b"import ") {
            scan_import(trimmed, &mut module);
        } else if trimmed.starts_with(b"class ") {
            if let Some(name) = extract_def_name(trimmed, b"class ") {
                module.definitions.insert(
                    name.clone(),
                    Symbol {
                        name,
                        kind: SymbolKind::Class,
                        members: HashMap::new(),
                        bases: vec![],
                        location: None,
                    },
                );
            }
        } else if trimmed.starts_with(b"def ") {
            if let Some(name) = extract_def_name(trimmed, b"def ") {
                module.definitions.insert(
                    name.clone(),
                    Symbol {
                        name,
                        kind: SymbolKind::Function,
                        members: HashMap::new(),
                        bases: vec![],
                        location: None,
                    },
                );
            }
        } else if trimmed.starts_with(b"async def ") {
            if let Some(name) = extract_def_name(trimmed, b"async def ") {
                module.definitions.insert(
                    name.clone(),
                    Symbol {
                        name,
                        kind: SymbolKind::Function,
                        members: HashMap::new(),
                        bases: vec![],
                        location: None,
                    },
                );
            }
        }
    }

    module
}

fn strip_leading_spaces(line: &[u8]) -> &[u8] {
    let start = line
        .iter()
        .position(|b| *b != b' ' && *b != b'\t')
        .unwrap_or(line.len());
    &line[start..]
}

/// Extract a class/function name from a line like `class Foo(Bar):` or `def baz(x):`.
fn extract_def_name(line: &[u8], prefix: &[u8]) -> Option<String> {
    let rest = &line[prefix.len()..];
    let end = rest
        .iter()
        .position(|b| !b.is_ascii_alphanumeric() && *b != b'_')?;
    if end == 0 {
        return None;
    }
    std::str::from_utf8(&rest[..end]).ok().map(String::from)
}

/// Parse `from <module> import <name1>, <name2>` (single line only).
fn scan_from_import(line: &[u8], module: &mut Module) {
    let line_str = match std::str::from_utf8(line) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Match: from <source> import <names>
    let Some(rest) = line_str.strip_prefix("from ") else {
        return;
    };
    let Some((source_raw, rest)) = rest.split_once(" import ") else {
        return;
    };
    let source_raw = source_raw.trim();

    // Resolve relative imports.
    let source_path = if source_raw.starts_with('.') {
        crate::parse::resolve_relative_import(&module.path, source_raw, module.is_package)
    } else {
        source_raw.to_string()
    };

    // Parse comma-separated names, handling `as` aliases.
    for part in rest.split(',') {
        let part = part.trim();
        if part.is_empty() || part.starts_with('#') {
            break;
        }
        // Handle trailing backslash continuation or parens — we only do single line.
        let part = part.trim_end_matches(['(', ')', '\\']);
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((name, alias)) = part.split_once(" as ") {
            let name = name.trim().to_string();
            let alias = alias.trim().to_string();
            module.imports.push(Import {
                source: source_path.clone(),
                name,
                alias: Some(alias),
            });
        } else {
            let name = part.trim().to_string();
            if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                module.imports.push(Import {
                    source: source_path.clone(),
                    name,
                    alias: None,
                });
            }
        }
    }
}

/// Parse `import <module>` (single line only).
fn scan_import(line: &[u8], module: &mut Module) {
    let line_str = match std::str::from_utf8(line) {
        Ok(s) => s,
        Err(_) => return,
    };

    let rest = match line_str.strip_prefix("import ") {
        Some(r) => r,
        None => return,
    };

    for part in rest.split(',') {
        let part = part.trim();
        if part.is_empty() || part.starts_with('#') {
            break;
        }

        if let Some((dotted, alias)) = part.split_once(" as ") {
            let dotted = dotted.trim();
            let alias = alias.trim();
            let name = dotted.rsplit('.').next().unwrap_or(dotted).to_string();
            module.imports.push(Import {
                source: dotted
                    .rsplit_once('.')
                    .map(|(p, _)| p.to_string())
                    .unwrap_or_default(),
                name,
                alias: Some(alias.to_string()),
            });
        } else {
            let dotted = part.trim();
            if dotted
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
            {
                let name = dotted.rsplit('.').next().unwrap_or(dotted).to_string();
                module.imports.push(Import {
                    source: dotted
                        .rsplit_once('.')
                        .map(|(p, _)| p.to_string())
                        .unwrap_or_default(),
                    name,
                    alias: None,
                });
            }
        }
    }
}

// Make resolve_relative_import accessible from fast_scan.
// It's defined in parse.rs and we call it through crate::parse::.
