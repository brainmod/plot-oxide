// Utility to create test Parquet files from CSV
// Run with: rustc create_parquet.rs && ./create_parquet

use std::fs::File;
use std::io::Write;

fn main() {
    // This is a simple script - we'll use polars directly from the main crate
    println!("To create Parquet files, use the following Polars code:");
    println!();
    println!("use polars::prelude::*;");
    println!();
    println!("let df = CsvReader::from_path(\"test-data/sample_timeseries.csv\")?");
    println!("    .has_header(true)");
    println!("    .finish()?;");
    println!();
    println!("let mut file = File::create(\"test-data/sample_timeseries.parquet\")?;");
    println!("ParquetWriter::new(&mut file).finish(&mut df)?;");
}
