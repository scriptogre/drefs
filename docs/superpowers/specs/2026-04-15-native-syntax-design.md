# doxr Native Syntax: Rust-style Intra-doc Links for Python

## Problem

Python has no standard way to cross-reference symbols in docstrings that is:
- Validated at lint time (not just at docs build time)
- IDE-aware (Ctrl+Click, squiggles, rename refactoring)
- Syntax-agnostic (works regardless of MkDocs vs Sphinx)

doxr already supports MkDocs (`[text][path]`, `[path][]`) and Sphinx (`:role:`path``) syntax. This design adds a **doxr-native syntax** modeled on Rust's intra-doc links that works alongside existing patterns.

## Design

### New Syntax

Inspired by Rust's `[Item]` / `` [`Item`] `` intra-doc links. Brackets delimit the reference. Backticks inside brackets are optional formatting.

| Syntax | Category | Resolution |
|---|---|---|
| `[User]` | Short name | Via imports/definitions in current module |
| `` [`User`] `` | Short name (code-styled) | Same |
| `[pkg.models.User]` | Fully qualified | Direct symbol graph lookup |
| `` [`pkg.models.User`] `` | Fully qualified (code-styled) | Same |

Existing syntax continues to work unchanged:

| Syntax | Style |
|---|---|
| `[text][pkg.models.User]` | MkDocs explicit |
| `` [`User`][pkg.models.User] `` | MkDocs explicit (code-styled) |
| `[pkg.models.User][]` | MkDocs autoref |
| `:class:`pkg.models.User`` | Sphinx |

The native syntax is always active regardless of the configured `style` (MkDocs/Sphinx/Auto).

### Escaping

`\[User\]` suppresses link detection, following Rust's convention. If `[optional]` happens to match a symbol in scope and you didn't mean it as a ref, escape it.

### Heuristic Filter

The regex for doxr-native refs:

```
(?<!\]|\\)\[`?([a-zA-Z_][\w.]*)`?\](?!\[)
```

- `(?<!\]|\\)` — not preceded by `]` or `\` (avoids matching the `[path]` part of MkDocs `[text][path]`, and supports escaping)
- `\[` — opening bracket
- `` `? `` — optional opening backtick
- `([a-zA-Z_][\w.]*)` — capture: starts with letter/underscore, then word chars + dots
- `` `? `` — optional closing backtick
- `\]` — closing bracket
- `(?!\[)` — NOT followed by `[` (avoids colliding with MkDocs `[text][path]` / `[path][]`)
- Must also not match the second `[path]` of an MkDocs explicit ref `[text][path]`. Add negative lookbehind for `]`: `(?<!\])` at the start, OR extract native refs last and deduplicate by offset.

Anything with spaces, slashes, special characters, or starting with a digit is silently skipped by the regex. This naturally filters out prose like `[see above]`, `[RFC 7231]`, `[1]`.

### Categorization

After the regex matches, the captured text is categorized:

- **Contains a dot** → `FullyQualified` — resolve directly against the symbol graph
- **No dot** → `ShortName` — expand via current module's imports/definitions first, then resolve

Both categories produce an **error if unresolved**. This is the Rust model: if it passes the heuristic filter, it's treated as a cross-reference. No silent "link if possible" for things that look like symbols.

### Resolution: Two-Pass Model

**Pass 1 — Pattern matching** (identical regex in CLI and plugin):

All patterns (MkDocs, Sphinx, doxr-native) extract references. Each reference carries:
- `target`: the captured text
- `offset`: byte position in docstring
- `kind`: `FullyQualified` or `ShortName`

Existing MkDocs/Sphinx refs are always `FullyQualified` (they already require dotted paths).

**Pass 2 — Resolution** (platform-specific backends, same logic):

- `FullyQualified` → resolve directly (current behavior, unchanged)
- `ShortName` → look up in current module context:
  1. Check imports — if `from pkg.models import User` exists, expand to `pkg.models.User`
  2. Check local definitions — if `User` is defined in this file, expand to `{module_path}.User`
  3. Neither matches → error: unresolved reference

Resolution order (imports first, then local definitions) matches Python's own name resolution.

### CLI Implementation

**`src/extract.rs`**:

- Add `ReferenceKind` enum: `FullyQualified`, `ShortName`
- Add `kind` field to `Reference` struct
- Add `DOXR_NATIVE` static regex
- New function `extract_native()` called in all styles
- Existing `is_fully_qualified()` used to categorize: dot → `FullyQualified`, no dot → `ShortName`
- Remove the `is_fully_qualified` gate from existing MkDocs/Sphinx extractors for the native pattern (the native pattern handles both dotted and non-dotted)

**`src/diagnostic.rs`**:

- When a `ShortName` ref is encountered, look up the name in the module's `imports` and `definitions` to expand it to an FQN before resolving
- `FullyQualified` refs follow the existing path unchanged

### PyCharm Plugin Implementation

**`DoxrReferenceProvider.kt`**:

- Add `DOXR_NATIVE` pattern (same regex, Java `Pattern.compile`)
- In `findRefs()`, handle group 1 from native pattern
- For dotted paths: create per-segment `DoxrPythonReference` objects (existing behavior)
- For short names: create a single `DoxrPythonReference` that resolves via `PyPsiFacade` using the file's scope context

**`DoxrAnnotator.kt`**:

- Add `DOXR_NATIVE` pattern
- For dotted paths: highlight each segment as identifier, dots as punctuation (existing behavior)
- For short names: highlight the name as an identifier

**`DoxrPythonReference.kt`**:

- For short names, resolve by looking up imports in the containing file, then checking definitions. This mirrors the CLI's resolution order.

### Symmetry Between CLI and Plugin

The implementations read symmetrically:

| Step | CLI (Rust) | Plugin (Kotlin) |
|---|---|---|
| Pattern matching | `extract.rs`: regex → `Vec<Reference>` | `DoxrReferenceProvider.kt`: regex → `Array<PsiReference>` |
| Heuristic filter | Same regex | Same regex |
| Categorize | `is_fully_qualified()` → `ReferenceKind` | `path.contains('.')` → branch |
| FQ resolution | `graph.resolve(&target)` | `PyPsiFacade.resolveQualifiedName(qName, ctx)` |
| Short name expansion | `module.imports` + `module.definitions` → FQN | File's imports via PSI → FQN |
| Highlighting | N/A | `DoxrAnnotator.kt`: identifier + dot attrs |
| Error reporting | `Diagnostic` with DXR001 | `soft = false` → red squiggles |

### Test Fixture

New fixture: `tests/fixtures/native_syntax/`

Structure:
```
tests/fixtures/native_syntax/
  pyproject.toml
  src/pkg/
    __init__.py          # re-exports User
    models.py            # defines User, Admin
    services.py          # docstrings using native syntax
```

`services.py` contains docstrings mixing all syntax forms:

**Must resolve (no false positives):**
- `[pkg.models.User]` — FQ bare brackets
- `` [`pkg.models.User`] `` — FQ with backticks
- `[User]` — short name via import
- `` [`User`] `` — short name with backticks via import
- `[Admin]` — short name via import
- `[helper_func]` — short name function via import
- Mixed alongside `[text][pkg.models.User]` and `:class:`pkg.models.User``

**Must error (broken refs):**
- `[Nonexistent]` — not in scope
- `[pkg.models.Fake]` — FQ, doesn't exist
- `` [`AlsoFake`] `` — not in scope, with backticks

**Must be ignored (not a ref):**
- `\[User\]` — escaped
- `[see above]` — contains space, filtered by regex
- `[1]` — starts with digit, filtered by regex
- `[some/path]` — contains slash, filtered by regex
