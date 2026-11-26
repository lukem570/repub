//! Test that importing a .pub file twice produces identical results.

use std::env;
use repub_format_native::{save_bytes, SaveOptions};
use repub_model::reset_id_counter;

fn main() -> anyhow::Result<()> {
    let pub_file = env::args().nth(1).expect("Usage: test_import_stability <file.pub>");
    
    println!("Testing import stability for: {}", pub_file);
    
    // Reset ID counter before first import for deterministic IDs
    reset_id_counter(1);
    let doc1 = repub_format_pub::import(&pub_file)?;
    
    // Reset ID counter again before second import
    reset_id_counter(1);
    let doc2 = repub_format_pub::import(&pub_file)?;
    
    // Serialize both to JSON (via the native-format DTO layer) for comparison
    let json1 = String::from_utf8(save_bytes(&doc1, &SaveOptions::default())?)?;
    let json2 = String::from_utf8(save_bytes(&doc2, &SaveOptions::default())?)?;
    
    if json1 == json2 {
        println!("✓ Import is stable - both imports are identical");
        println!("  Document size: {} bytes JSON", json1.len());
    } else {
        println!("✗ Import is NOT stable - documents differ!");
        
        // Find differences
        let lines1: Vec<&str> = json1.lines().collect();
        let lines2: Vec<&str> = json2.lines().collect();
        
        let mut diff_count = 0;
        for (i, (l1, l2)) in lines1.iter().zip(lines2.iter()).enumerate() {
            if l1 != l2 {
                if diff_count < 10 {
                    println!("  Line {}: ", i + 1);
                    println!("    Import 1: {}", l1);
                    println!("    Import 2: {}", l2);
                }
                diff_count += 1;
            }
        }
        
        if lines1.len() != lines2.len() {
            println!("  Line count differs: {} vs {}", lines1.len(), lines2.len());
        }
        
        if diff_count > 10 {
            println!("  ... and {} more differences", diff_count - 10);
        }
        
        std::process::exit(1);
    }
    
    Ok(())
}
