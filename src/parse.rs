/// Parse a Python file using tree-sitter and extract definitions, imports,
/// docstrings, and `__all__`.
use crate::graph::{Docstring, Import, Module, SourceLocation, Symbol, SymbolKind};
use anyhow::{Context, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, Parser};

thread_local! {
    static PARSER: RefCell<Parser> = RefCell::new({
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
        parser
    });
}

/// Create a SourceLocation from a tree-sitter node.
fn node_location(node: Node, file_path: &str) -> SourceLocation {
    let pos = node.start_position();
    SourceLocation {
        file: file_path.to_string(),
        line: pos.row + 1,
        col: pos.column + 1,
    }
}

/// Parse a single `.py` file and return a [`Module`].
pub fn parse_file(file_path: &Path, dotted_path: &str) -> Result<Module> {
    let source = std::fs::read(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;
    parse_bytes(&source, file_path, dotted_path)
}

/// Parse Python source bytes and return a [`Module`].
pub fn parse_bytes(source: &[u8], file_path: &Path, dotted_path: &str) -> Result<Module> {
    let is_package = file_path.file_name().is_some_and(|f| f == "__init__.py");

    let tree = PARSER
        .with_borrow_mut(|parser| parser.parse(source, None))
        .context("tree-sitter failed to parse")?;

    let root = tree.root_node();

    let file_path_str = file_path.display().to_string();
    let mut module = Module {
        path: dotted_path.to_string(),
        file_path: file_path_str,
        is_package,
        definitions: HashMap::new(),
        imports: Vec::new(),
        all: None,
        docstrings: Vec::new(),
    };

    // Walk top-level statements.
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        process_node(child, source, &mut module);
    }

    Ok(module)
}

/// Process a single AST node, extracting definitions/imports/docstrings.
fn process_node(node: Node, src: &[u8], module: &mut Module) {
    match node.kind() {
        "class_definition" => {
            if let Some(sym) = extract_class(node, src, module) {
                module.definitions.insert(sym.name.clone(), sym);
            }
        }
        "decorated_definition" => {
            // Could be a decorated class or a decorated function.
            if let Some(inner) = node.child_by_field_name("definition") {
                match inner.kind() {
                    "class_definition" => {
                        if let Some(sym) = extract_class(inner, src, module) {
                            module.definitions.insert(sym.name.clone(), sym);
                        }
                    }
                    _ => {
                        if let Some(sym) = extract_function(node, src, module) {
                            module.definitions.insert(sym.name.clone(), sym);
                        }
                    }
                }
            }
        }
        "function_definition" => {
            if let Some(sym) = extract_function(node, src, module) {
                module.definitions.insert(sym.name.clone(), sym);
            }
        }
        "import_from_statement" => {
            extract_from_import(node, src, module);
        }
        "import_statement" => {
            extract_import(node, src, module);
        }
        "expression_statement" => {
            handle_expression_statement(node, src, module);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Definition extractors
// ---------------------------------------------------------------------------

fn extract_class(node: Node, src: &[u8], module: &mut Module) -> Option<Symbol> {
    let name = node.child_by_field_name("name")?;
    let name_text = name.utf8_text(src).ok()?.to_string();

    let mut members = HashMap::new();
    let bases = extract_bases(node, src);

    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "function_definition" | "decorated_definition" => {
                    if let Some(sym) = extract_function(child, src, module) {
                        // Extract self.X assignments from __init__.
                        if sym.name == "__init__" {
                            let func_node = if child.kind() == "decorated_definition" {
                                child.child_by_field_name("definition")
                            } else {
                                Some(child)
                            };
                            if let Some(func_node) = func_node {
                                extract_self_attributes(func_node, src, &mut members);
                            }
                        }
                        members.insert(sym.name.clone(), sym);
                    }
                }
                "expression_statement" => {
                    // Class-level docstring or attribute assignment.
                    if let Some(ds) = try_extract_docstring(child, src) {
                        module.docstrings.push(ds);
                    } else {
                        extract_attribute(child, src, &mut members);
                    }
                }
                _ => {}
            }
        }
    }

    Some(Symbol {
        name: name_text,
        kind: SymbolKind::Class,
        members,
        bases,
        location: Some(node_location(name, &module.file_path)),
    })
}

/// Extract base class names from a class definition's argument list.
fn extract_bases(node: Node, src: &[u8]) -> Vec<String> {
    let mut bases = Vec::new();
    let Some(superclasses) = node.child_by_field_name("superclasses") else {
        return bases;
    };
    let mut cursor = superclasses.walk();
    for child in superclasses.children(&mut cursor) {
        match child.kind() {
            "identifier" | "dotted_name" | "attribute" => {
                if let Ok(text) = child.utf8_text(src) {
                    bases.push(text.to_string());
                }
            }
            // Handle Generic[T], ABC[T], BaseClass[Param] — extract the base name.
            "subscript" => {
                if let Some(value) = child.child_by_field_name("value") {
                    if let Ok(text) = value.utf8_text(src) {
                        bases.push(text.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    bases
}

fn extract_function(node: Node, src: &[u8], module: &mut Module) -> Option<Symbol> {
    // Handle decorated definitions by finding the inner function.
    let func_node = if node.kind() == "decorated_definition" {
        node.child_by_field_name("definition")?
    } else {
        node
    };

    let name = func_node.child_by_field_name("name")?;
    let name_text = name.utf8_text(src).ok()?.to_string();

    // Extract function docstring (first expression statement in body).
    if let Some(body) = func_node.child_by_field_name("body") {
        if let Some(first) = body.child(0) {
            if let Some(ds) = try_extract_docstring(first, src) {
                module.docstrings.push(ds);
            }
        }
    }

    Some(Symbol {
        name: name_text,
        kind: SymbolKind::Function,
        members: HashMap::new(),
        bases: vec![],
        location: Some(node_location(name, &module.file_path)),
    })
}

fn extract_attribute(node: Node, src: &[u8], defs: &mut HashMap<String, Symbol>) {
    let Some(child) = node.child(0) else { return };
    match child.kind() {
        "assignment" => {
            let Some(left) = child.child_by_field_name("left") else {
                return;
            };
            if left.kind() == "identifier" {
                let Some(name) = left.utf8_text(src).ok().map(String::from) else {
                    return;
                };
                defs.insert(
                    name.clone(),
                    Symbol {
                        name,
                        kind: SymbolKind::Attribute,
                        members: HashMap::new(),
                        bases: vec![],
                        location: None,
                    },
                );
            }
        }
        "type_alias_statement" => {
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(src) {
                    let name = name.to_string();
                    defs.insert(
                        name.clone(),
                        Symbol {
                            name,
                            kind: SymbolKind::Attribute,
                            members: HashMap::new(),
                            bases: vec![],
                            location: None,
                        },
                    );
                }
            }
        }
        _ => {}
    }
}

/// Walk a function body to find `self.X = ...` assignments and register
/// them as class-level attribute members.
fn extract_self_attributes(func_node: Node, src: &[u8], members: &mut HashMap<String, Symbol>) {
    let Some(body) = func_node.child_by_field_name("body") else {
        return;
    };
    walk_for_self_attrs(body, src, members);
}

fn walk_for_self_attrs(node: Node, src: &[u8], members: &mut HashMap<String, Symbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "expression_statement" => {
                // Look for `self.X = ...`
                if let Some(assign) = child.child(0) {
                    if assign.kind() == "assignment" {
                        if let Some(left) = assign.child_by_field_name("left") {
                            if left.kind() == "attribute" {
                                if let (Some(obj), Some(attr)) = (
                                    left.child_by_field_name("object"),
                                    left.child_by_field_name("attribute"),
                                ) {
                                    if obj.utf8_text(src).ok() == Some("self") {
                                        if let Ok(attr_name) = attr.utf8_text(src) {
                                            let name = attr_name.to_string();
                                            members.entry(name.clone()).or_insert(Symbol {
                                                name,
                                                kind: SymbolKind::Attribute,
                                                members: HashMap::new(),
                                                bases: vec![],
                                                location: None,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Recurse into if/else/try blocks inside __init__.
            "if_statement" | "else_clause" | "elif_clause" | "try_statement" | "except_clause"
            | "finally_clause" | "with_statement" | "for_statement" | "while_statement" => {
                walk_for_self_attrs(child, src, members);
            }
            "block" => {
                walk_for_self_attrs(child, src, members);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Import extractors
// ---------------------------------------------------------------------------

fn extract_from_import(node: Node, src: &[u8], module: &mut Module) {
    // `from <module_name> import <name> [as <alias>], ...`
    //
    // tree-sitter-python represents `from .foo import X` as:
    //   import_from_statement
    //     "from"
    //     import_prefix: "."      (the dots)
    //     module_name: "foo"      (without dots)
    //     "import"
    //     dotted_name: "X"
    //
    // We need to combine import_prefix + module_name for relative imports.
    let module_name_node = node.child_by_field_name("module_name");
    let prefix_node = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "import_prefix");

    let source_text = match (prefix_node, module_name_node) {
        (Some(prefix), Some(name)) => {
            let prefix_text = prefix.utf8_text(src).unwrap_or("");
            let name_text = name.utf8_text(src).unwrap_or("");
            format!("{prefix_text}{name_text}")
        }
        (Some(prefix), None) => {
            // `from . import X` — just dots, no module name.
            prefix.utf8_text(src).unwrap_or("").to_string()
        }
        (None, Some(name)) => {
            // Absolute import: `from pkg.foo import X`
            name.utf8_text(src).unwrap_or("").to_string()
        }
        (None, None) => return,
    };

    // Resolve relative imports.
    let source_path = if source_text.starts_with('.') {
        resolve_relative_import(&module.path, &source_text, module.is_package)
    } else {
        source_text
    };

    // Collect imported names. Skip the module_name node itself — it's also
    // a dotted_name but represents the source module, not an imported name.
    let module_name_node = node.child_by_field_name("module_name");
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Skip the module_name node (already handled above).
        if let Some(ref mn) = module_name_node {
            if child.id() == mn.id() {
                continue;
            }
        }
        if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
            if let Some(imp) = parse_import_name(child, src, &source_path) {
                module.imports.push(imp);
            }
        }
    }
}

fn extract_import(node: Node, src: &[u8], module: &mut Module) {
    // `import <dotted_name> [as <alias>]`
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "dotted_name" => {
                let text = match child.utf8_text(src) {
                    Ok(t) => t.to_string(),
                    Err(_) => continue,
                };
                // For `import foo.bar`, the source is `foo.bar` and name is `bar`.
                let name = text.rsplit('.').next().unwrap_or(&text).to_string();
                module.imports.push(Import {
                    source: text
                        .rsplit_once('.')
                        .map(|(prefix, _)| prefix.to_string())
                        .unwrap_or_default(),
                    name,
                    alias: None,
                });
            }
            "aliased_import" => {
                let name_node = child.child_by_field_name("name");
                let alias_node = child.child_by_field_name("alias");
                if let Some(n) = name_node {
                    let text = match n.utf8_text(src) {
                        Ok(t) => t.to_string(),
                        Err(_) => continue,
                    };
                    let alias = alias_node
                        .and_then(|a| a.utf8_text(src).ok())
                        .map(|s| s.to_string());
                    let name = text.rsplit('.').next().unwrap_or(&text).to_string();
                    module.imports.push(Import {
                        source: text
                            .rsplit_once('.')
                            .map(|(prefix, _)| prefix.to_string())
                            .unwrap_or_default(),
                        name,
                        alias,
                    });
                }
            }
            _ => {}
        }
    }
}

fn parse_import_name(node: Node, src: &[u8], source_path: &str) -> Option<Import> {
    match node.kind() {
        "dotted_name" => {
            let name = node.utf8_text(src).ok()?.to_string();
            Some(Import {
                source: source_path.to_string(),
                name,
                alias: None,
            })
        }
        "aliased_import" => {
            let name_node = node.child_by_field_name("name")?;
            let name = name_node.utf8_text(src).ok()?.to_string();
            let alias = node
                .child_by_field_name("alias")
                .and_then(|a| a.utf8_text(src).ok())
                .map(|s| s.to_string());
            Some(Import {
                source: source_path.to_string(),
                name,
                alias,
            })
        }
        _ => None,
    }
}

/// Resolve a relative import like `.foo` or `..bar` against the current module path.
///
/// For `__init__.py` modules (packages), `.foo` means "submodule foo of this package".
/// For regular modules, `.foo` means "sibling module foo".
pub fn resolve_relative_import(current_module: &str, relative: &str, is_package: bool) -> String {
    let dots = relative.chars().take_while(|c| *c == '.').count();
    let remainder = &relative[dots..];

    let parts: Vec<&str> = current_module.split('.').collect();

    // For __init__.py, the module path IS the package, so 1 dot = this package.
    // For regular modules, 1 dot = parent package (go up 1 from the module).
    let effective_dots = if is_package { dots - 1 } else { dots };
    let up = effective_dots.min(parts.len());
    let base: Vec<&str> = parts[..parts.len() - up].to_vec();

    if remainder.is_empty() {
        base.join(".")
    } else {
        let mut result = base.join(".");
        if !result.is_empty() {
            result.push('.');
        }
        result.push_str(remainder);
        result
    }
}

// ---------------------------------------------------------------------------
// Docstring / __all__ extraction
// ---------------------------------------------------------------------------

/// Try to extract a docstring from an expression_statement node.
fn try_extract_docstring(node: Node, src: &[u8]) -> Option<Docstring> {
    if node.kind() != "expression_statement" {
        return None;
    }
    let expr = node.child(0)?;
    if expr.kind() != "string" && expr.kind() != "concatenated_string" {
        return None;
    }
    let text = expr.utf8_text(src).ok()?;
    // Only triple-quoted strings count as docstrings.
    if !text.starts_with("\"\"\"") && !text.starts_with("'''") {
        return None;
    }
    Some(Docstring {
        line: expr.start_position().row + 1, // 1-indexed
        col: expr.start_position().column + 1,
        content: text.to_string(),
    })
}

fn handle_expression_statement(node: Node, src: &[u8], module: &mut Module) {
    // Check for docstring first.
    if let Some(ds) = try_extract_docstring(node, src) {
        module.docstrings.push(ds);
        return;
    }

    // Check for `__all__ = [...]`.
    let child = match node.child(0) {
        Some(c) => c,
        None => return,
    };

    if child.kind() == "assignment" {
        let left = match child.child_by_field_name("left") {
            Some(l) => l,
            None => return,
        };
        if left.kind() == "identifier" && left.utf8_text(src).ok() == Some("__all__") {
            if let Some(right) = child.child_by_field_name("right") {
                module.all = Some(extract_all_list(right, src));
            }
        }
        // Also register as an attribute definition.
        extract_attribute(node, src, &mut module.definitions);
    }
}

/// Extract string elements from a list literal (the `__all__` value).
fn extract_all_list(node: Node, src: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    if node.kind() != "list" {
        return names;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string" {
            if let Ok(text) = child.utf8_text(src) {
                // Strip quotes.
                let stripped = text
                    .trim_start_matches(['\'', '"'])
                    .trim_end_matches(['\'', '"']);
                names.push(stripped.to_string());
            }
        }
    }
    names
}
