/// Configuration loaded from `pyproject.toml` under `[tool.doxr]`.
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocStyle {
    Mkdocs,
    Sphinx,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct DoxrConfig {
    /// Source directories to scan (e.g. `["src/my_package"]`).
    #[serde(default)]
    pub src: Vec<PathBuf>,

    /// Documentation style for cross-reference syntax.
    #[serde(default)]
    pub style: DocStyle,

    /// Glob patterns to exclude.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// External module roots to skip validation for.
    #[serde(default)]
    pub known_modules: Vec<String>,

    /// Inventory files (objects.inv) to load for external reference validation.
    /// Can be URLs or local file paths.
    #[serde(default)]
    pub inventories: Vec<String>,
}

/// Wrapper to deserialize `[tool.doxr]` from pyproject.toml.
#[derive(Debug, Deserialize)]
struct PyProject {
    tool: Option<ToolTable>,
}

#[derive(Debug, Deserialize)]
struct ToolTable {
    doxr: Option<DoxrConfig>,
}

impl DoxrConfig {
    /// Load config from `pyproject.toml` in the given directory.
    /// Returns default config if the file doesn't exist or has no `[tool.doxr]`.
    pub fn load(project_root: &Path) -> Result<Self> {
        let pyproject_path = project_root.join("pyproject.toml");

        if !pyproject_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&pyproject_path)
            .with_context(|| format!("Failed to read {}", pyproject_path.display()))?;

        let pyproject: PyProject = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", pyproject_path.display()))?;

        Ok(pyproject.tool.and_then(|t| t.doxr).unwrap_or_default())
    }

    /// Return effective source directories, auto-detecting if not configured.
    ///
    /// Detection order:
    /// 1. Explicit `src` config → use as-is
    /// 2. `src/` directory exists with Python packages inside → `["src"]`
    /// 3. Any top-level directory containing `__init__.py` → `["."]`
    /// 4. Fallback → `["."]`
    pub fn effective_src(&self, project_root: &Path) -> Vec<PathBuf> {
        if !self.src.is_empty() {
            return self
                .src
                .iter()
                .map(|s| {
                    if s.is_absolute() {
                        s.clone()
                    } else {
                        project_root.join(s)
                    }
                })
                .collect();
        }

        // Auto-detect: check for src layout first.
        let src_dir = project_root.join("src");
        if src_dir.is_dir() && has_python_packages(&src_dir) {
            return vec![src_dir];
        }

        // Flat layout: project root itself.
        vec![project_root.to_path_buf()]
    }

    /// Auto-detect documentation style if set to Auto.
    ///
    /// - `mkdocs.yml` or `mkdocs.yaml` exists → MkDocs
    /// - `conf.py` exists (Sphinx) → Sphinx
    /// - Otherwise → Auto (check both)
    pub fn effective_style(&self, project_root: &Path) -> DocStyle {
        if self.style != DocStyle::Auto {
            return self.style.clone();
        }

        if project_root.join("mkdocs.yml").exists() || project_root.join("mkdocs.yaml").exists() {
            return DocStyle::Mkdocs;
        }

        // Check for Sphinx conf.py in common locations.
        if project_root.join("conf.py").exists()
            || project_root.join("docs/conf.py").exists()
            || project_root.join("doc/conf.py").exists()
        {
            return DocStyle::Sphinx;
        }

        DocStyle::Auto
    }
}

/// Check if a directory contains at least one Python package (dir with __init__.py).
fn has_python_packages(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries
        .filter_map(|e| e.ok())
        .any(|entry| entry.path().is_dir() && entry.path().join("__init__.py").exists())
}
