/// Configuration loaded from `pyproject.toml` under `[tool.doxr]`.
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocStyle {
    Mkdocs,
    Sphinx,
    Auto,
}

impl Default for DocStyle {
    fn default() -> Self {
        Self::Auto
    }
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

        Ok(pyproject
            .tool
            .and_then(|t| t.doxr)
            .unwrap_or_default())
    }

    /// Return effective source directories (defaults to `["."]` if empty).
    pub fn effective_src(&self, project_root: &Path) -> Vec<PathBuf> {
        if self.src.is_empty() {
            vec![project_root.to_path_buf()]
        } else {
            self.src
                .iter()
                .map(|s| {
                    if s.is_absolute() {
                        s.clone()
                    } else {
                        project_root.join(s)
                    }
                })
                .collect()
        }
    }
}
