/// Module discovery: walk the filesystem and map `.py` files to dotted module paths.
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// A discovered Python file and its dotted module path.
#[derive(Debug)]
pub struct DiscoveredModule {
    pub file_path: PathBuf,
    pub dotted_path: String,
}

/// Walk `src_dirs`, find all `.py` files, and compute their dotted module paths.
///
/// `exclude` patterns are applied as gitignore-style globs.
pub fn discover_modules(src_dirs: &[PathBuf], exclude: &[String]) -> Vec<DiscoveredModule> {
    let mut modules = Vec::new();

    for src_dir in src_dirs {
        if !src_dir.exists() {
            continue;
        }

        let mut builder = WalkBuilder::new(src_dir);
        builder.hidden(false); // don't skip hidden files by default

        // Add exclude patterns as overrides.
        let mut overrides = ignore::overrides::OverrideBuilder::new(src_dir);
        for pattern in exclude {
            // Negate the pattern so matching files are excluded.
            let negated = format!("!{pattern}");
            let _ = overrides.add(&negated);
        }
        if let Ok(ov) = overrides.build() {
            builder.overrides(ov);
        }

        for entry in builder.build().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "py") {
                if let Some(dotted) = path_to_dotted(path, src_dir) {
                    modules.push(DiscoveredModule {
                        file_path: path.to_path_buf(),
                        dotted_path: dotted,
                    });
                }
            }
        }
    }

    modules
}

/// Convert a filesystem path to a dotted Python module path.
///
/// `src/my_pkg/foo/bar.py`  → `my_pkg.foo.bar`
/// `src/my_pkg/foo/__init__.py` → `my_pkg.foo`
fn path_to_dotted(file_path: &Path, src_root: &Path) -> Option<String> {
    let relative = file_path.strip_prefix(src_root).ok()?;

    let mut components: Vec<&str> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if components.is_empty() {
        return None;
    }

    // Strip the `.py` extension from the last component.
    let last = components.last_mut()?;
    *last = last.strip_suffix(".py")?;

    // If the file is `__init__.py`, drop the `__init__` segment —
    // the package path is the directory itself.
    if *last == "__init__" {
        components.pop();
        if components.is_empty() {
            return None;
        }
    }

    Some(components.join("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_regular_module() {
        let path = Path::new("/src/my_pkg/foo/bar.py");
        let root = Path::new("/src");
        assert_eq!(
            path_to_dotted(path, root),
            Some("my_pkg.foo.bar".to_string())
        );
    }

    #[test]
    fn test_init_module() {
        let path = Path::new("/src/my_pkg/foo/__init__.py");
        let root = Path::new("/src");
        assert_eq!(path_to_dotted(path, root), Some("my_pkg.foo".to_string()));
    }

    #[test]
    fn test_top_level_init() {
        let path = Path::new("/src/__init__.py");
        let root = Path::new("/src");
        assert_eq!(path_to_dotted(path, root), None);
    }
}
