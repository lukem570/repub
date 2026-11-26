//! Publisher format compatibility testing and validation.
//!
//! This module provides tools to measure and improve .pub file format compatibility:
//! - Import validation: Does the importer correctly read all data?
//! - Export validation: Does the exporter produce readable files?
//! - Round-trip testing: Import → Export → Re-import = identical?
//! - Block-level analysis: Which block types are correctly handled?

#[allow(missing_docs)]
mod block_coverage;
#[allow(missing_docs)]
mod export_validator;
#[allow(missing_docs)]
mod import_validator;
#[allow(missing_docs)]
mod report;
#[allow(missing_docs)]
mod roundtrip;

pub use block_coverage::{BlockCoverage, BlockStats};
pub use export_validator::{ExportValidationResult, ExportValidator};
pub use import_validator::{ImportValidationResult, ImportValidator};
pub use report::{check_file, generate_report, CompatibilityReport};
pub use roundtrip::{RoundtripResult, RoundtripTest};
