// Temporarily allow dead code during migration phase
#![allow(dead_code)]

use polars::prelude::*;
use std::path::{Path, PathBuf};

/// Errors that can occur during data operations
#[derive(Debug)]
pub enum DataError {
    PolarsError(PolarsError),
    IoError(std::io::Error),
    UnsupportedFormat(String),
    ColumnNotFound(String),
}

impl From<PolarsError> for DataError {
    fn from(err: PolarsError) -> Self {
        DataError::PolarsError(err)
    }
}

impl From<std::io::Error> for DataError {
    fn from(err: std::io::Error) -> Self {
        DataError::IoError(err)
    }
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataError::PolarsError(e) => write!(f, "Polars error: {}", e),
            DataError::IoError(e) => write!(f, "IO error: {}", e),
            DataError::UnsupportedFormat(ext) => write!(f, "Unsupported file format: {}", ext),
            DataError::ColumnNotFound(col) => write!(f, "Column not found: {}", col),
        }
    }
}

impl std::error::Error for DataError {}

/// DataSource wraps a Polars DataFrame with both lazy and materialized views
pub struct DataSource {
    /// Lazy frame for efficient filtering and transformations
    df: LazyFrame,
    /// Materialized DataFrame for display and immediate access
    materialized: DataFrame,
    /// Original file path
    file_path: Option<PathBuf>,
}

impl DataSource {
    /// Load data from a file (CSV or Parquet)
    pub fn load(path: &Path) -> Result<Self, DataError> {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| DataError::UnsupportedFormat("No file extension".to_string()))?;

        let df = match extension.to_lowercase().as_str() {
            "parquet" => LazyFrame::scan_parquet(path, Default::default())?,
            "csv" => LazyCsvReader::new(path)
                .with_has_header(true)
                .with_infer_schema_length(Some(100))
                .finish()?,
            ext => return Err(DataError::UnsupportedFormat(ext.to_string())),
        };

        let materialized = df.clone().collect()?;

        Ok(Self {
            df,
            materialized,
            file_path: Some(path.to_path_buf()),
        })
    }

    /// Get a reference to the materialized DataFrame
    pub fn dataframe(&self) -> &DataFrame {
        &self.materialized
    }

    /// Get column values as a Series
    pub fn column_values(&self, col: &str) -> Result<Series, DataError> {
        self.materialized
            .column(col)
            .map(|c| c.as_materialized_series().clone())
            .map_err(|_| DataError::ColumnNotFound(col.to_string()))
    }

    /// Get all column names
    pub fn column_names(&self) -> Vec<String> {
        self.materialized
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get the number of rows
    pub fn height(&self) -> usize {
        self.materialized.height()
    }

    /// Get the number of columns
    pub fn width(&self) -> usize {
        self.materialized.width()
    }

    /// Get the file path
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Apply filters to the data
    pub fn apply_filters(&mut self, filters: impl Fn() -> Expr) -> Result<(), DataError> {
        let filter_expr = filters();
        self.materialized = self.df.clone().filter(filter_expr).collect()?;
        Ok(())
    }

    /// Re-materialize the DataFrame (useful after lazy operations)
    pub fn refresh(&mut self) -> Result<(), DataError> {
        self.materialized = self.df.clone().collect()?;
        Ok(())
    }
}
