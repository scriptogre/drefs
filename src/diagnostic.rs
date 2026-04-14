/// Diagnostics: validate references and emit Ruff-format output.
use crate::config::DoxrConfig;
use crate::extract::{extract_references, Reference};
use crate::graph::SymbolGraph;
use crate::inventory::Inventory;
use std::path::Path;

/// A single diagnostic (error) to report.
#[derive(Debug)]
pub struct Diagnostic {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {} {}",
            self.file, self.line, self.col, self.code, self.message
        )
    }
}

/// Check all docstrings in the symbol graph and return diagnostics.
pub fn check(
    graph: &SymbolGraph,
    config: &DoxrConfig,
    inventory: &Inventory,
    file_map: &[(String, String)], // (dotted_path, file_path)
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (dotted_path, file_path) in file_map {
        let module = match graph.modules.get(dotted_path) {
            Some(m) => m,
            None => continue,
        };

        for docstring in &module.docstrings {
            let refs = extract_references(&docstring.content, &config.style);
            for r in refs {
                if is_explicitly_skipped(&r, config) {
                    continue;
                }

                let internal = is_internal(&r, graph);

                if internal {
                    // Internal ref: must resolve in our symbol graph.
                    if !graph.resolve(&r.target) {
                        diagnostics.push(Diagnostic {
                            file: file_path.clone(),
                            line: docstring.line,
                            col: docstring.col,
                            code: "DXR001",
                            message: format!("Unresolved reference `{}`", r.target),
                        });
                    }
                } else if inventory.covers_root(&r.target) {
                    // External ref whose root module is covered by an inventory:
                    // check it. If the root isn't covered, silently skip.
                    if !inventory.contains(&r.target) {
                        diagnostics.push(Diagnostic {
                            file: file_path.clone(),
                            line: docstring.line,
                            col: docstring.col,
                            code: "DXR001",
                            message: format!("Unresolved reference `{}`", r.target),
                        });
                    }
                }
                // External ref not covered by any inventory: silently skip.
            }
        }
    }

    // Sort by file, then line, then column for stable output.
    diagnostics.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.col.cmp(&b.col))
    });

    diagnostics
}

/// Check if a reference should be skipped entirely (explicit config).
fn is_explicitly_skipped(reference: &Reference, config: &DoxrConfig) -> bool {
    let root = reference.target.split('.').next().unwrap_or("");
    config.known_modules.iter().any(|km| {
        let km_root = km.split('.').next().unwrap_or("");
        root == km_root
    })
}

/// Check if a reference's root module is part of the project we're checking.
fn is_internal(reference: &Reference, graph: &SymbolGraph) -> bool {
    let root = reference.target.split('.').next().unwrap_or("");
    graph.modules.keys().any(|path| {
        path == root || path.starts_with(&format!("{root}."))
    })
}

/// Format diagnostics as a summary line.
pub fn summary(diagnostics: &[Diagnostic]) -> String {
    match diagnostics.len() {
        0 => "All references OK.".to_string(),
        1 => "Found 1 error.".to_string(),
        n => format!("Found {n} errors."),
    }
}

/// Relativize a file path for display.
pub fn display_path(path: &str, project_root: &Path) -> String {
    Path::new(path)
        .strip_prefix(project_root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.to_string())
}
