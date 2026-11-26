//! Integration tests against real Publisher `.pub` fixtures.
//!
//! Fixtures live in the repository's `reverse/test-files/` directory and are
//! read-only. Tests that depend on a fixture skip gracefully when it is
//! absent.

use std::path::{Path, PathBuf};

use repub_format_pub::compat::RoundtripTest;

/// Locate the repository's `reverse/test-files` directory relative to this
/// crate, if it exists.
fn fixtures_dir() -> Option<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // crates/repub-format-pub -> repo root
    let root = manifest.parent()?.parent()?;
    let dir = root.join("reverse/test-files");
    if dir.is_dir() {
        Some(dir)
    } else {
        None
    }
}

fn fixture(name: &str) -> Option<PathBuf> {
    let path = fixtures_dir()?.join(name);
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

#[test]
fn imports_a_simple_publisher_file() {
    let Some(path) = fixture("Simple.pub") else {
        eprintln!("skipping: Simple.pub fixture not available");
        return;
    };

    let doc = repub_format_pub::import(&path).expect("import Simple.pub");
    assert!(doc.page_count() >= 1, "expected at least one page");
}

#[test]
fn imports_a_multipage_publisher_file() {
    let Some(path) = fixture("example_multipage.pub") else {
        eprintln!("skipping: example_multipage.pub fixture not available");
        return;
    };

    let doc = repub_format_pub::import(&path).expect("import example_multipage.pub");
    assert!(doc.page_count() >= 1);
}

#[test]
fn imports_several_fixtures_without_panicking() {
    let Some(dir) = fixtures_dir() else {
        eprintln!("skipping: reverse/test-files not available");
        return;
    };

    // A representative selection of well-formed sample files. Import should
    // succeed and produce at least one page for each.
    let samples = [
        "Sample.pub",
        "Sample2.pub",
        "Sample3.pub",
        "MSPublisher2000-Sample.pub",
    ];

    let mut tested = 0;
    for name in samples {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        let doc = repub_format_pub::import(&path)
            .unwrap_or_else(|e| panic!("import {name}: {e}"));
        assert!(doc.page_count() >= 1, "{name}: expected at least one page");
        tested += 1;
    }

    if tested == 0 {
        eprintln!("skipping: no sample fixtures available");
    }
}

#[test]
fn import_bytes_matches_import_path() {
    let Some(path) = fixture("Simple.pub") else {
        eprintln!("skipping: Simple.pub fixture not available");
        return;
    };

    let bytes = std::fs::read(&path).expect("read fixture bytes");
    let from_path = repub_format_pub::import(&path).expect("import from path");
    let from_bytes = repub_format_pub::import_bytes(&bytes).expect("import from bytes");
    assert_eq!(from_path.page_count(), from_bytes.page_count());
    assert_eq!(
        from_path.total_shape_count(),
        from_bytes.total_shape_count()
    );
}

/// Strict import never silently drops data: whenever a fixture imports cleanly
/// with the strict default, the lenient path must recover the same document.
/// Whenever strict refuses a file, it must be because of unread items, and the
/// lenient path must still succeed.
#[test]
fn strict_and_lenient_agree() {
    let Some(dir) = fixtures_dir() else {
        eprintln!("skipping: reverse/test-files not available");
        return;
    };

    let samples = [
        "Simple.pub",
        "Sample.pub",
        "Sample2.pub",
        "Sample3.pub",
        "MSPublisher2000-Sample.pub",
        "example_multipage.pub",
    ];

    let mut tested = 0;
    for name in samples {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        tested += 1;

        // Lenient always succeeds (best-effort recovery).
        let lenient = repub_format_pub::import_lenient(&path)
            .unwrap_or_else(|e| panic!("lenient import {name}: {e}"));

        match repub_format_pub::import(&path) {
            Ok(strict) => {
                // Clean file: strict and lenient recover the same document.
                assert_eq!(
                    strict.total_shape_count(),
                    lenient.total_shape_count(),
                    "{name}: strict and lenient shape counts differ"
                );
            }
            Err(repub_format_pub::FormatError::UnreadItems(items)) => {
                // Strict refused: it must name at least one unread item, and
                // lenient must still have produced a usable document.
                assert!(!items.is_empty(), "{name}: UnreadItems with no items");
                assert!(lenient.page_count() >= 1, "{name}: lenient produced no pages");
            }
            Err(e) => panic!("{name}: unexpected strict error: {e}"),
        }
    }

    if tested == 0 {
        eprintln!("skipping: no sample fixtures available");
    }
}

/// Every well-formed `.pub` fixture must import cleanly under the strict
/// default — no `UnreadItems`. Intentionally not imported shapes (ALTSHAPE
/// alternates, confirmed-invisible shapes) are not data loss and must not fail
/// strict import. Deliberately-corrupt fuzzer testcases are skipped: they are
/// expected to fail at the OLE layer, not strict import.
#[test]
fn every_well_formed_fixture_imports_strictly() {
    let Some(dir) = fixtures_dir() else {
        eprintln!("skipping: reverse/test-files not available");
        return;
    };

    let mut tested = 0;
    for entry in std::fs::read_dir(&dir).expect("read fixtures dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().map(|e| e == "pub").unwrap_or(false) {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            // Skip known-corrupt fuzzer inputs (they fail at the OLE layer).
            if name.contains("clusterfuzz") || name.contains("Fuzzer") {
                continue;
            }
            match repub_format_pub::import(&path) {
                Ok(_) => tested += 1,
                Err(repub_format_pub::FormatError::UnreadItems(items)) => {
                    panic!("{name}: strict import left {} item(s) unread:\n  {}", items.len(), items.join("\n  "));
                }
                // A malformed container that isn't a fuzzer input is still a
                // real failure to surface.
                Err(e) => panic!("{name}: import failed: {e}"),
            }
        }
    }

    if tested == 0 {
        eprintln!("skipping: no well-formed fixtures available");
    }
}

#[test]
fn roundtrip_report_runs_on_a_fixture() {
    let Some(path) = fixture("Simple.pub") else {
        eprintln!("skipping: Simple.pub fixture not available");
        return;
    };

    let test = RoundtripTest::new();
    let result = test.test(&path);
    // Import must succeed; export/re-import fidelity is tracked but not yet
    // complete (see docs/TODO.md), so we only assert the pipeline ran.
    assert!(result.import_success, "import step should succeed");
    let _ = result.summary();
}
