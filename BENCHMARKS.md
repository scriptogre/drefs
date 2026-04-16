# Benchmarks

Benchmarks comparing drefs against `mkdocs build --strict` for validating cross-references.

drefs checks cross-references only. `mkdocs build --strict` renders the entire documentation site (Markdown processing, HTML generation, search index, etc.), which includes cross-reference validation as a side effect.

## Setup

- Apple M1 Pro, 16 GB RAM
- macOS 15.7.1
- Rust 1.94.0 (release build)
- Measured with [hyperfine](https://github.com/sharkdp/hyperfine)

## Results

### tinygrad (697 Python files, 32k stars)

Validating cross-references in [tinygrad/tinygrad](https://github.com/tinygrad/tinygrad):

| Command | Mean | Min | Max |
|:---|---:|---:|---:|
| `drefs .` | 110 ms | 98 ms | 125 ms |
| `mkdocs build --strict` | 51.0 s | 48.3 s | 52.5 s |

**~460x faster.**

### httpx (60 Python files, 13k stars)

Validating cross-references in [encode/httpx](https://github.com/encode/httpx):

| Command | Mean | Min | Max |
|:---|---:|---:|---:|
| `drefs .` | 19 ms | 16 ms | 50 ms |
| `mkdocs build --strict` | 872 ms | 845 ms | 921 ms |

**~45x faster.**

## Methodology

```bash
# drefs (release build)
cargo build --release
hyperfine --warmup 3 --ignore-failure './target/release/drefs .'

# mkdocs build --strict
hyperfine --warmup 3 '.venv/bin/mkdocs build --strict'
```

drefs and `mkdocs build --strict` are not doing identical work. drefs validates cross-references against source code symbols. `mkdocs build --strict` renders an entire documentation site, which happens to catch some broken references along the way. The comparison shows how fast you can get cross-reference validation without waiting for a full docs build.
