/// Parse Sphinx objects.inv (intersphinx inventory) files.
///
/// Format (v2):
///   Line 1: `# Sphinx inventory version 2`
///   Line 2: `# Project: <name>`
///   Line 3: `# Version: <version>`
///   Line 4: `# The remainder of this file is compressed using zlib.`
///   Rest:   zlib-compressed lines, each:
///           `{name} {domain}:{role} {priority} {uri} {dispname}`
///
/// We only care about entries in the `py` domain (py:class, py:function, etc.).
use anyhow::{Context, Result, bail};
use flate2::read::ZlibDecoder;
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;

/// A set of known external Python symbols loaded from inventory files.
#[derive(Debug, Default)]
pub struct Inventory {
    /// Fully-qualified Python symbol names (e.g. `pydantic.BaseModel`).
    pub symbols: HashSet<String>,
    /// Root module names covered by loaded inventories (e.g. `{"ast", "os", "typing"}`).
    pub covered_roots: HashSet<String>,
    /// Project names that were loaded (for diagnostics).
    pub projects: Vec<String>,
}

impl Inventory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a dotted reference exists in the inventory.
    pub fn contains(&self, reference: &str) -> bool {
        self.symbols.contains(reference)
    }

    /// Check if a reference's root module is covered by any loaded inventory.
    pub fn covers_root(&self, reference: &str) -> bool {
        let root = reference.split('.').next().unwrap_or("");
        self.covered_roots.contains(root)
    }

    /// Load an inventory from a local file path.
    pub fn load_file(&mut self, path: &Path) -> Result<()> {
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read inventory: {}", path.display()))?;
        self.parse_inv(&data)
    }

    /// Load an inventory from a URL.
    pub fn load_url(&mut self, url: &str) -> Result<()> {
        let response = ureq::get(url)
            .call()
            .with_context(|| format!("Failed to fetch inventory: {url}"))?;

        let data = response
            .into_body()
            .read_to_vec()
            .with_context(|| format!("Failed to read response body from: {url}"))?;

        self.parse_inv(&data)
    }

    /// Parse raw objects.inv bytes.
    fn parse_inv(&mut self, data: &[u8]) -> Result<()> {
        // Read header lines (plain text, newline-terminated).
        let mut pos = 0;
        let mut header_lines = Vec::new();

        for _ in 0..4 {
            let line_end = data[pos..]
                .iter()
                .position(|&b| b == b'\n')
                .context("Inventory header too short")?;
            let line = std::str::from_utf8(&data[pos..pos + line_end])
                .context("Invalid UTF-8 in header")?;
            header_lines.push(line.to_string());
            pos += line_end + 1;
        }

        // Validate header.
        if !header_lines[0].contains("Sphinx inventory version 2") {
            bail!(
                "Unsupported inventory format: {}",
                header_lines[0]
            );
        }

        // Extract project name.
        let project = header_lines[1]
            .strip_prefix("# Project: ")
            .unwrap_or("unknown")
            .to_string();

        // Decompress the rest.
        let compressed = &data[pos..];
        let mut decoder = ZlibDecoder::new(compressed);
        let mut decompressed = String::new();
        decoder
            .read_to_string(&mut decompressed)
            .context("Failed to decompress inventory data")?;

        // Parse each line.
        for line in decompressed.lines() {
            // Format: `{name} {domain}:{role} {priority} {uri} {dispname}`
            // We only want `py:*` entries.
            let parts: Vec<&str> = line.splitn(5, ' ').collect();
            if parts.len() < 4 {
                continue;
            }

            let name = parts[0];
            let domain_role = parts[1];

            if domain_role.starts_with("py:") {
                self.symbols.insert(name.to_string());
                // Track root modules.
                if let Some(root) = name.split('.').next() {
                    self.covered_roots.insert(root.to_string());
                }
            }
        }

        self.projects.push(project);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inv() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        // Build a minimal objects.inv in memory.
        let header = b"# Sphinx inventory version 2\n\
                        # Project: TestProject\n\
                        # Version: 1.0\n\
                        # The remainder of this file is compressed using zlib.\n";

        let data_lines = b"my_pkg.MyClass py:class 1 api/#$ -\n\
                           my_pkg.my_func py:function 1 api/#$ -\n\
                           some_c_thing c:macro 1 api/#$ -\n";

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data_lines).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut inv_bytes = header.to_vec();
        inv_bytes.extend_from_slice(&compressed);

        let mut inv = Inventory::new();
        inv.parse_inv(&inv_bytes).unwrap();

        assert!(inv.contains("my_pkg.MyClass"));
        assert!(inv.contains("my_pkg.my_func"));
        assert!(!inv.contains("some_c_thing")); // c: domain, not py:
        assert_eq!(inv.projects, vec!["TestProject"]);
    }
}
