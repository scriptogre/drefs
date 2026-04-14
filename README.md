# doxr

A hyper-fast Python docstring cross-reference checker.

Catches broken `[text][pkg.mod.Class]` (MkDocs) and `:class:`pkg.mod.Class`` (Sphinx) references in your docstrings **without running a full docs build**.

## Usage

```bash
# Check the current project
uvx doxr .

# With external symbol validation via objects.inv
uvx doxr . -i https://docs.python.org/3/objects.inv
```

## Configuration

Add to your `pyproject.toml`:

```toml
[tool.doxr]
src = ["src"]
style = "auto"  # "mkdocs" | "sphinx" | "auto"
inventories = [
    "https://docs.python.org/3/objects.inv",
]
```

## Output

Matches Ruff's format for seamless CI integration:

```
src/my_pkg/models.py:12:5: DXR001 Unresolved reference `my_pkg.old_module.Foo`
```
