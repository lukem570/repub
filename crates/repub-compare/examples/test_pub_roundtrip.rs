//! Test round-trip: import .pub, export to .pub, re-import, and compare.

use std::env;
use std::io::Cursor;

fn main() -> anyhow::Result<()> {
    let pub_file = env::args()
        .nth(1)
        .expect("Usage: test_pub_roundtrip <file.pub>");

    println!("Testing round-trip for: {}", pub_file);

    // Import the original .pub file
    println!("1. Importing original file...");
    let doc1 = repub_format_pub::import(&pub_file)?;
    println!("   Imported {} pages", doc1.pages().len());

    // Export to a new .pub file (in memory)
    println!("2. Exporting to .pub format...");
    let mut exported_data = Cursor::new(Vec::new());
    repub_format_pub::pub_export::export_to_writer(
        &doc1,
        &mut exported_data,
        &repub_format_pub::pub_export::PubExportOptions::default(),
    )?;
    let exported_bytes = exported_data.into_inner();
    println!("   Exported {} bytes", exported_bytes.len());

    // Try to re-import the exported file
    println!("3. Re-importing exported file...");

    // Write to temp file for import (pub_import needs a path)
    let temp_path = "/tmp/roundtrip_test_export.pub";
    std::fs::write(temp_path, &exported_bytes)?;

    match repub_format_pub::import(temp_path) {
        Ok(doc2) => {
            println!("   Re-imported {} pages", doc2.pages().len());

            // Compare key properties
            println!("\n4. Comparing documents...");

            if doc1.pages().len() == doc2.pages().len() {
                println!("   ✓ Page count matches: {}", doc1.pages().len());
            } else {
                println!(
                    "   ✗ Page count differs: {} vs {}",
                    doc1.pages().len(),
                    doc2.pages().len()
                );
            }

            // Compare page sizes
            for (i, (p1, p2)) in doc1.pages().iter().zip(doc2.pages().iter()).enumerate() {
                if (p1.size.width - p2.size.width).abs() < 0.1
                    && (p1.size.height - p2.size.height).abs() < 0.1
                {
                    println!("   ✓ Page {} size matches", i + 1);
                } else {
                    println!(
                        "   ✗ Page {} size differs: {:?} vs {:?}",
                        i + 1,
                        p1.size,
                        p2.size
                    );
                }
            }

            println!("\n✓ Round-trip completed (note: fidelity may vary)");
        }
        Err(e) => {
            println!("   ✗ Failed to re-import: {}", e);
            println!("\nNote: Our .pub export is a basic implementation.");
            println!("Full Publisher compatibility requires more work on:");
            println!("  - Contents stream format");
            println!("  - Quill text formatting");
            println!("  - Escher shape records");
        }
    }

    // Clean up
    let _ = std::fs::remove_file(temp_path);

    Ok(())
}
