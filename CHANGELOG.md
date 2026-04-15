# Changelog

## 0.1.0

Initial release.

- MkDocs cross-reference checking (`[text][pkg.mod.Class]`, `[path][]`)
- Sphinx cross-reference checking (`:class:`, `:func:`, `:meth:`, etc.)
- doxr-native syntax (`[Symbol]`, `` [`Symbol`] ``) with short name resolution via imports
- Zero-config auto-detection of src layout and doc style
- Symbol graph with re-export resolution, inheritance chains, `self.x` attributes
- External symbol validation via Sphinx `objects.inv` inventories
- Ruff-compatible output format (`file:line:col: DXR001 message`)
- PyCharm plugin with Ctrl+Click navigation, syntax highlighting, and squiggles
- LSP server for editor integration
