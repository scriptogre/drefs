# Benchmarks

Benchmarks comparing doxr against `mkdocs build --strict` for validating cross-references.

doxr checks cross-references only. `mkdocs build --strict` renders the entire documentation site (Markdown processing, HTML generation, search index, etc.), which includes cross-reference validation as a side effect.

## Setup

- Apple M1 Pro, 16 GB RAM
- macOS 15.7.1
- Rust 1.94.0 (release build)
- Measured with [hyperfine](https://github.com/sharkdp/hyperfine), 3 warmup runs

## Results

### httpx (60 Python files)

Validating cross-references in [encode/httpx](https://github.com/encode/httpx):

| Command | Mean | Min | Max |
|:---|---:|---:|---:|
| `doxr .` | 19 ms | 17 ms | 31 ms |
| `mkdocs build --strict` | 876 ms | 854 ms | 962 ms |

doxr is **~45x faster**.

### pydantic (401 Python files)

Checking cross-references in [pydantic/pydantic](https://github.com/pydantic/pydantic):

| Command | Mean |
|:---|---:|
| `doxr .` | 114 ms |

### pydantic-ai (503 Python files)

Checking cross-references in [pydantic/pydantic-ai](https://github.com/pydantic/pydantic-ai):

| Command | Mean |
|:---|---:|
| `doxr .` | 169 ms |

`mkdocs build --strict` could not be benchmarked on pydantic and pydantic-ai due to private dependencies required by their documentation configs. doxr works on any Python project with no setup.

## Methodology

```bash
# doxr (release build)
cargo build --release
hyperfine --warmup 3 --ignore-failure './target/release/doxr .'

# mkdocs build --strict
hyperfine --warmup 3 '.venv/bin/mkdocs build --strict'
```

Note: doxr and `mkdocs build --strict` are not doing identical work. doxr validates cross-references against source code symbols. `mkdocs build --strict` renders an entire documentation site, which happens to catch some broken references along the way. The comparison shows how fast you can get cross-reference validation without waiting for a full docs build.
