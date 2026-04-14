use std::process::Command;

fn run_doxr(fixture: &str) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_doxr"))
        .arg(format!("tests/fixtures/{fixture}"))
        .output()
        .expect("Failed to run doxr");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

fn extract_unresolved(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| l.contains("DXR001"))
        .filter_map(|l| {
            let start = l.find('`')? + 1;
            let end = l[start..].find('`')? + start;
            Some(l[start..end].to_string())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Edge cases fixture
// ---------------------------------------------------------------------------

#[test]
fn edge_cases_catches_all_broken_refs() {
    let (stdout, _stderr, code) = run_doxr("edge_cases");
    let errors = extract_unresolved(&stdout);

    // These should all be flagged.
    let expected_errors = vec![
        "pkg.mdoels.User",
        "pkg.models.NonExistent",
        "pkg.models.User.nonexistent_method",
        "pkg.totally_fake.Thing",
        "pkg.deep.DeepClass",
        "pkg.sub.deep.DeepClass.nope",
        "pkg.sub.helpers.nonexistent_func",
        "pkg.models.FakeClass",
        "pkg.nonexistent.bad_func",
    ];

    for expected in &expected_errors {
        assert!(
            errors.contains(&expected.to_string()),
            "Expected error for `{expected}` but it was not flagged.\nAll errors: {errors:?}"
        );
    }

    assert_ne!(code, 0, "Should exit non-zero when errors found");
}

#[test]
fn edge_cases_no_false_positives() {
    let (stdout, _stderr, _code) = run_doxr("edge_cases");
    let errors = extract_unresolved(&stdout);

    // These should all resolve successfully (NOT appear in errors).
    let must_resolve = vec![
        // Direct definitions
        "pkg.models.User",
        "pkg.models.Admin",
        "pkg.models.User.greet",
        "pkg.sub.deep.DeepClass",
        "pkg.sub.deep.DeepClass.process",
        "pkg.sub.helpers.helper_func",
        "pkg.sub.helpers.another_helper",
        "pkg.reexport.layer_c.ChainedClass",
        // Re-exports through __init__.py
        "pkg.User",
        "pkg.Admin",
        "pkg.DeepClass",
        "pkg.reexport.ChainedClass",
        "pkg.reexport.layer_b.ChainedClass",
        "pkg.sub.DeepClass",
        // Inherited methods (via mixin)
        "pkg.models.User.to_json",
        "pkg.models.Admin.to_json",
        "pkg.sub.deep.DeepClass.to_json",
        // self.X attributes from __init__
        "pkg.models.User.name",
        "pkg.models.Admin.name",
        // Class-level attributes
        "pkg.models.User.role",
        // Module reference
        "pkg.sub",
    ];

    for valid in &must_resolve {
        assert!(
            !errors.contains(&valid.to_string()),
            "False positive: `{valid}` was flagged but should resolve.\nAll errors: {errors:?}"
        );
    }
}

#[test]
fn edge_cases_error_count() {
    let (stdout, _stderr, _code) = run_doxr("edge_cases");
    let errors = extract_unresolved(&stdout);
    // 10 total: 4 from models.py, 3 from sphinx_style.py, 2 from deep.py, 1 from helpers.py
    // (nonexistent_method appears twice: once in models.py mkdocs, once in sphinx_style.py sphinx)
    assert_eq!(
        errors.len(),
        10,
        "Expected exactly 10 errors, got {}.\nErrors: {errors:?}",
        errors.len()
    );
}

// ---------------------------------------------------------------------------
// Decorated classes, subscript bases, relative __init__.py imports
// ---------------------------------------------------------------------------

#[test]
fn decorated_classes_catches_broken_refs() {
    let (stdout, _stderr, code) = run_doxr("decorated_classes");
    let errors = extract_unresolved(&stdout);

    let expected = vec![
        "pkg.sub.base.AbstractBase.nope",
        "pkg.sub.child.ConcreteChild.fake_method",
        "pkg.sub.config.Config.nonexistent",
    ];
    for e in &expected {
        assert!(errors.contains(&e.to_string()), "Missing expected error: {e}");
    }
    assert_eq!(errors.len(), 3);
    assert_ne!(code, 0);
}

#[test]
fn decorated_classes_no_false_positives() {
    let (stdout, _stderr, _code) = run_doxr("decorated_classes");
    let errors = extract_unresolved(&stdout);

    let must_resolve = vec![
        // Direct methods
        "pkg.sub.base.AbstractBase.process",
        "pkg.sub.base.AbstractBase.class_create",
        "pkg.sub.base.AbstractBase.async_method",
        // Own + inherited methods on child with subscript base
        "pkg.sub.child.ConcreteChild.do_stuff",
        "pkg.sub.child.ConcreteChild.process",
        "pkg.sub.child.ConcreteChild.class_create",
        "pkg.sub.child.ConcreteChild.async_method",
        // Re-exports through __init__.py with relative imports
        "pkg.sub.ConcreteChild.do_stuff",
        "pkg.sub.ConcreteChild.process",
        "pkg.ConcreteChild.do_stuff",
        // Decorated (@dataclass) class attributes
        "pkg.sub.config.Config.name",
        "pkg.sub.config.Config.value",
        "pkg.Config.name",
    ];

    for valid in &must_resolve {
        assert!(
            !errors.contains(&valid.to_string()),
            "False positive: `{valid}` was flagged.\nAll errors: {errors:?}"
        );
    }
}
