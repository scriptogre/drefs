/// Symbol graph: a lightweight representation of a Python project's namespace.
///
/// The graph is built by parsing every `.py` file in the project and extracting
/// definitions (classes, functions, attributes) and imports. References found in
/// docstrings are later validated against this graph.
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// A dotted Python path like `my_pkg.foo.Bar`.
pub type DottedPath = String;

/// The kind of symbol defined in Python source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SymbolKind {
    #[default]
    Function,
    Class,
    Attribute,
}

/// Source location of a symbol definition.
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize, // 1-indexed
    pub col: usize,  // 1-indexed
}

/// A single Python symbol (class, function, variable, …).
#[derive(Debug, Clone, Default)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    /// Nested members -- e.g. methods inside a class.
    pub members: HashMap<String, Symbol>,
    /// Base class names (for classes only), as written in the source.
    pub bases: Vec<String>,
    /// Where this symbol is defined.
    pub location: Option<SourceLocation>,
}

impl Symbol {
    /// Create a new symbol with the given name and kind, defaulting everything else.
    pub fn new(name: String, kind: SymbolKind) -> Self {
        Self {
            name,
            kind,
            ..Default::default()
        }
    }
}

/// A recorded `import` / `from … import …` statement.
#[derive(Debug, Clone)]
pub struct Import {
    /// The module being imported from (e.g. `my_pkg.utils`).
    pub source: DottedPath,
    /// The name being imported (e.g. `MyClass`).
    pub name: String,
    /// Optional alias (`as …`).
    pub alias: Option<String>,
}

/// A docstring occurrence we need to check.
#[derive(Debug, Clone)]
pub struct Docstring {
    pub line: usize,
    pub col: usize,
    pub content: String,
}

/// A parsed Python module.
#[derive(Debug, Clone)]
pub struct Module {
    /// Dotted module path (e.g. `my_pkg.foo.bar`).
    pub path: DottedPath,
    /// File path on disk.
    pub file_path: String,
    /// Whether this module is a package (`__init__.py`).
    pub is_package: bool,
    /// Top-level definitions in this module.
    pub definitions: HashMap<String, Symbol>,
    /// Import statements.
    pub imports: Vec<Import>,
    /// Source modules of `from X import *` statements.
    pub wildcard_imports: Vec<DottedPath>,
    /// `__all__` if defined (explicit public API).
    pub all: Option<Vec<String>>,
    /// Docstrings found in this module (module-level + all nested).
    pub docstrings: Vec<Docstring>,
}

// ---------------------------------------------------------------------------
// The graph itself
// ---------------------------------------------------------------------------

/// Project-wide symbol graph.
#[derive(Debug, Default)]
pub struct SymbolGraph {
    pub modules: HashMap<DottedPath, Module>,
    /// Precomputed set of root package names (e.g. `{"pkg", "my_lib"}`).
    /// Built by [`SymbolGraph::compute_roots`].
    root_packages: HashSet<String>,
}

impl SymbolGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a parsed module into the graph.
    pub fn add_module(&mut self, module: Module) {
        self.modules.insert(module.path.clone(), module);
    }

    /// Precompute the set of root package names from all modules.
    /// Call after all modules are added and wildcards are expanded.
    pub fn compute_roots(&mut self) {
        self.root_packages = self
            .modules
            .keys()
            .filter_map(|path| path.split('.').next().map(String::from))
            .collect();
    }

    /// Check if a dotted reference's root module is part of this project.
    pub fn is_internal(&self, target: &str) -> bool {
        let root = target.split('.').next().unwrap_or("");
        self.root_packages.contains(root)
    }

    /// Expand `from X import *` statements by copying exported symbols
    /// from the source module into the importing module's imports.
    ///
    /// If the source module defines `__all__`, only those names are exported.
    /// Otherwise, all names not starting with `_` are exported.
    pub fn expand_wildcards(&mut self) {
        // Collect the work to do: (importing_module_path, source_module_path).
        let work: Vec<(String, String)> = self
            .modules
            .values()
            .flat_map(|m| {
                m.wildcard_imports
                    .iter()
                    .map(move |src| (m.path.clone(), src.clone()))
            })
            .collect();

        for (importer_path, source_path) in work {
            // Look up the source module and collect names to export.
            let exported: Vec<(String, String)> = match self.modules.get(&source_path) {
                Some(source) => {
                    if let Some(ref all) = source.all {
                        // __all__ defined: export only those names.
                        all.iter()
                            .map(|name| (source_path.clone(), name.clone()))
                            .collect()
                    } else {
                        // No __all__: export all public definitions (no leading _).
                        source
                            .definitions
                            .keys()
                            .filter(|name| !name.starts_with('_'))
                            .map(|name| (source_path.clone(), name.clone()))
                            .collect()
                    }
                }
                None => continue,
            };

            // Add synthetic imports to the importing module.
            if let Some(importer) = self.modules.get_mut(&importer_path) {
                for (source, name) in exported {
                    // Don't duplicate if already explicitly imported.
                    let already_imported = importer
                        .imports
                        .iter()
                        .any(|imp| imp.source == source && imp.name == name);
                    if !already_imported {
                        importer.imports.push(Import {
                            source,
                            name,
                            alias: None,
                        });
                    }
                }
            }
        }
    }

    /// Resolve a dotted reference like `my_pkg.foo.Bar` to a [`Symbol`].
    ///
    /// Returns `true` if the reference resolves to a known symbol.
    pub fn resolve(&self, reference: &str) -> bool {
        self.resolve_inner(reference, 0)
    }

    fn resolve_inner(&self, reference: &str, depth: usize) -> bool {
        if depth > 20 {
            return false; // prevent infinite recursion
        }

        let segments: Vec<&str> = reference.split('.').collect();
        if segments.is_empty() {
            return false;
        }

        // Try progressively longer module prefixes.
        // e.g. for `a.b.C.method`, try `a.b.C`, then `a.b`, then `a` as the
        // module path, and walk the remainder through definitions.
        for split in (1..=segments.len()).rev() {
            let module_path = segments[..split].join(".");
            if let Some(module) = self.modules.get(&module_path) {
                // If the entire reference *is* the module, it resolves.
                if split == segments.len() {
                    return true;
                }
                // Walk remaining segments through definitions.
                let remaining = &segments[split..];
                if resolve_in_definitions(&module.definitions, remaining) {
                    return true;
                }
                // If the first remaining segment is a class and the second
                // segment doesn't resolve, try base class inheritance.
                if remaining.len() >= 2
                    && let Some(sym) = module.definitions.get(remaining[0])
                    && sym.kind == SymbolKind::Class
                {
                    let class_fqn = format!("{}.{}", module_path, remaining[0]);
                    // Try resolving each remaining member via bases.
                    if self.resolve_via_bases(&class_fqn, remaining[1], depth + 1) {
                        // If there are more segments (method on inherited class),
                        // we only handle one level of member lookup for now.
                        if remaining.len() == 2 {
                            return true;
                        }
                    }
                }
                // Try following imports.
                if self.resolve_via_imports(module, remaining, depth) {
                    return true;
                }
            }
        }

        false
    }

    /// Attempt to resolve `remaining` segments by following an import in `module`.
    fn resolve_via_imports(&self, module: &Module, remaining: &[&str], depth: usize) -> bool {
        if remaining.is_empty() {
            return false;
        }

        let first = remaining[0];

        for imp in &module.imports {
            let local_name = imp.alias.as_deref().unwrap_or(&imp.name);
            if local_name == first {
                // Reconstruct the fully-qualified path through the import.
                let mut target = format!("{}.{}", imp.source, imp.name);
                for seg in &remaining[1..] {
                    target.push('.');
                    target.push_str(seg);
                }
                return self.resolve_inner(&target, depth + 1);
            }
        }

        false
    }

    /// Find a class Symbol by its fully-qualified path (e.g. `pkg.mod.ClassName`).
    fn find_class(&self, fqn: &str) -> Option<&Symbol> {
        let segments: Vec<&str> = fqn.split('.').collect();
        // Try progressively longer module prefixes.
        for split in (1..segments.len()).rev() {
            let module_path = segments[..split].join(".");
            if let Some(module) = self.modules.get(&module_path) {
                let class_name = segments[split];
                if let Some(sym) = module.definitions.get(class_name)
                    && sym.kind == SymbolKind::Class
                {
                    return Some(sym);
                }
                // Also check via imports.
                for imp in &module.imports {
                    let local_name = imp.alias.as_deref().unwrap_or(&imp.name);
                    if local_name == class_name {
                        let target_fqn = format!("{}.{}", imp.source, imp.name);
                        return self.find_class(&target_fqn);
                    }
                }
            }
        }
        None
    }

    /// Resolve a member by walking base classes (simple MRO).
    /// `class_fqn` is the fully-qualified class name, `member` is the attribute name.
    fn resolve_via_bases(&self, class_fqn: &str, member: &str, depth: usize) -> bool {
        if depth > 20 {
            return false;
        }

        // Find the module containing this class.
        let segments: Vec<&str> = class_fqn.split('.').collect();
        for split in (1..segments.len()).rev() {
            let module_path = segments[..split].join(".");
            if let Some(module) = self.modules.get(&module_path) {
                let class_name = segments[split..].join(".");
                // Look at the AST to find base class names.
                if let Some(bases) = self.get_class_bases(&module_path, &class_name) {
                    for base in bases {
                        // Try resolving the base class name.
                        let base_fqn = self.resolve_class_name(&base, module);
                        if let Some(base_fqn) = base_fqn {
                            // Check if the base class has this member.
                            if let Some(base_sym) = self.find_class(&base_fqn)
                                && base_sym.members.contains_key(member)
                            {
                                return true;
                            }
                            // Recurse up the hierarchy.
                            if self.resolve_via_bases(&base_fqn, member, depth + 1) {
                                return true;
                            }
                        }
                    }
                }
                break;
            }
        }
        false
    }

    /// Get base class names for a class in a given module.
    fn get_class_bases(&self, module_path: &str, class_name: &str) -> Option<Vec<String>> {
        let module = self.modules.get(module_path)?;
        let sym = module.definitions.get(class_name)?;
        if sym.kind == SymbolKind::Class && !sym.bases.is_empty() {
            Some(sym.bases.clone())
        } else {
            None
        }
    }

    /// Resolve a class name to its fully-qualified path using the module's imports.
    fn resolve_class_name(&self, name: &str, module: &Module) -> Option<String> {
        // If it contains dots, it might already be qualified.
        if name.contains('.') {
            // Check if it resolves as-is.
            let segments: Vec<&str> = name.split('.').collect();
            for split in (1..=segments.len()).rev() {
                let module_path = segments[..split].join(".");
                if self.modules.contains_key(&module_path) {
                    return Some(name.to_string());
                }
            }
        }

        // Check module's imports.
        for imp in &module.imports {
            let local_name = imp.alias.as_deref().unwrap_or(&imp.name);
            if local_name == name {
                return Some(format!("{}.{}", imp.source, imp.name));
            }
        }

        // Check module's own definitions.
        let module_path = &module.path;
        if module.definitions.contains_key(name) {
            return Some(format!("{module_path}.{name}"));
        }

        None
    }
}

/// Walk through nested definitions (`HashMap<String, Symbol>`) using the
/// given path segments.
fn resolve_in_definitions(defs: &HashMap<String, Symbol>, segments: &[&str]) -> bool {
    if segments.is_empty() {
        return true; // nothing left to resolve
    }
    if let Some(sym) = defs.get(segments[0]) {
        if segments.len() == 1 {
            return true;
        }
        return resolve_in_definitions(&sym.members, &segments[1..]);
    }
    false
}
