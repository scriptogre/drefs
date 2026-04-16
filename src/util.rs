//! Shared utility functions.

/// Levenshtein edit distance between two strings.
///
/// Used for "did you mean?" suggestions on unresolved references.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();
    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for (i, row) in matrix.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in matrix[0].iter_mut().enumerate() {
        *cell = j;
    }

    for (i, a_byte) in a.bytes().enumerate() {
        for (j, b_byte) in b.bytes().enumerate() {
            let cost = if a_byte == b_byte { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }

    matrix[a_len][b_len]
}

/// Resolve a relative import like `.foo` or `..bar` against the current module path.
///
/// For `__init__.py` modules (packages), `.foo` means "submodule foo of this package".
/// For regular modules, `.foo` means "sibling module foo".
pub fn resolve_relative_import(current_module: &str, relative: &str, is_package: bool) -> String {
    let dots = relative.chars().take_while(|c| *c == '.').count();
    let remainder = &relative[dots..];

    let parts: Vec<&str> = current_module.split('.').collect();

    // For __init__.py, the module path IS the package, so 1 dot = this package.
    // For regular modules, 1 dot = parent package (go up 1 from the module).
    let effective_dots = if is_package { dots - 1 } else { dots };
    let up = effective_dots.min(parts.len());
    let base: Vec<&str> = parts[..parts.len() - up].to_vec();

    if remainder.is_empty() {
        base.join(".")
    } else {
        let mut result = base.join(".");
        if !result.is_empty() {
            result.push('.');
        }
        result.push_str(remainder);
        result
    }
}
