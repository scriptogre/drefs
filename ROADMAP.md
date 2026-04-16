# Roadmap

## v0.1.0 (first release)

- [x] CLI with MkDocs, Sphinx, and Rust-style `[Symbol]` syntax
- [x] Zero-config auto-detection of src layout and doc style
- [x] Symbol resolution: re-exports, inheritance, `self.x` attributes
- [x] External symbol validation via `objects.inv` inventories
- [x] Ruff-compatible output format
- [x] PyCharm plugin with Ctrl+Click, highlighting, and squiggles
- [x] Benchmarks
- [ ] Publish to PyPI (`uvx drefs .` works)
- [ ] GitHub Action for CI integration

## Future

- [ ] VS Code extension (LSP server already exists)
- [ ] Performance: sub-100ms on 500+ file projects
- [ ] Rename-aware refactoring in editor plugins
- [ ] `.pyi` stub file resolution for third-party symbols
- [ ] Benchmark SVG chart for README (like ruff/uv)
