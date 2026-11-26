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
mod import_validator;
#[allow(missing_docs)]
mod export_validator;
#[allow(missing_docs)]
mod roundtrip;
#[allow(missing_docs)]
mod report;

pub use block_coverage::{BlockCoverage, BlockStats};
pub use import_validator::{ImportValidator, ImportValidationResult};
pub use export_validator::{ExportValidator, ExportValidationResult};
pub use roundtrip::{RoundtripTest, RoundtripResult};
pub use report::{CompatibilityReport, generate_report, check_file};
