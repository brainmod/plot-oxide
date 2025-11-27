//! Error types for PlotOxide
//!
//! This module provides structured error handling using thiserror,
//! replacing ad-hoc String-based errors with proper typed errors.

use thiserror::Error;

/// Main error type for PlotOxide operations
#[derive(Error, Debug)]
pub enum PlotError {
    /// File I/O error
    #[error("Failed to access file: {0}")]
    FileIo(#[from] std::io::Error),

    /// Polars data processing error
    #[error("Data processing error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    /// Configuration file error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Unsupported file format
    #[error("Unsupported file format: {extension}")]
    UnsupportedFormat { extension: String },

    /// Column not found in data
    #[error("Column '{column}' not found in dataset")]
    ColumnNotFound { column: String },

    /// Invalid column index
    #[error("Invalid column index: {index} (available: {max})")]
    InvalidColumnIndex { index: usize, max: usize },

    /// Empty dataset error
    #[error("Dataset is empty or has no rows")]
    EmptyDataset,

    /// Data validation error
    #[error("Data validation failed: {0}")]
    Validation(String),

    /// Insufficient data for operation
    #[error("Insufficient data: {operation} requires at least {required} points, but got {actual}")]
    InsufficientData {
        operation: String,
        required: usize,
        actual: usize,
    },

    /// Type conversion error
    #[error("Type conversion error: {0}")]
    TypeConversion(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic error with custom message
    #[error("{0}")]
    Custom(String),
}

/// Result type alias for PlotOxide operations
pub type Result<T> = std::result::Result<T, PlotError>;

/// UI-friendly error message formatting
impl PlotError {
    /// Get a user-friendly error message suitable for displaying in UI
    pub fn user_message(&self) -> String {
        match self {
            PlotError::FileIo(e) => format!("File error: {}", e),
            PlotError::Polars(e) => format!("Data error: {}", e),
            PlotError::Config(msg) => format!("Config error: {}", msg),
            PlotError::UnsupportedFormat { extension } => {
                format!("Unsupported file format: '.{}'", extension)
            }
            PlotError::ColumnNotFound { column } => {
                format!("Column '{}' not found", column)
            }
            PlotError::InvalidColumnIndex { index, max } => {
                format!("Column index {} out of range (max: {})", index, max)
            }
            PlotError::EmptyDataset => "Dataset is empty".to_string(),
            PlotError::Validation(msg) => format!("Validation error: {}", msg),
            PlotError::InsufficientData {
                operation,
                required,
                actual,
            } => {
                format!(
                    "{} requires {} points, but only {} available",
                    operation, required, actual
                )
            }
            PlotError::TypeConversion(msg) => format!("Type error: {}", msg),
            PlotError::Json(e) => format!("JSON error: {}", e),
            PlotError::Custom(msg) => msg.clone(),
        }
    }

    /// Get a short title for the error (for toast notifications)
    pub fn title(&self) -> &'static str {
        match self {
            PlotError::FileIo(_) => "File Error",
            PlotError::Polars(_) => "Data Error",
            PlotError::Config(_) => "Configuration Error",
            PlotError::UnsupportedFormat { .. } => "Unsupported Format",
            PlotError::ColumnNotFound { .. } => "Column Not Found",
            PlotError::InvalidColumnIndex { .. } => "Invalid Column",
            PlotError::EmptyDataset => "Empty Dataset",
            PlotError::Validation(_) => "Validation Error",
            PlotError::InsufficientData { .. } => "Insufficient Data",
            PlotError::TypeConversion(_) => "Type Error",
            PlotError::Json(_) => "JSON Error",
            PlotError::Custom(_) => "Error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages() {
        let err = PlotError::ColumnNotFound {
            column: "Temperature".to_string(),
        };
        assert_eq!(err.user_message(), "Column 'Temperature' not found");
        assert_eq!(err.title(), "Column Not Found");

        let err = PlotError::InsufficientData {
            operation: "Moving average".to_string(),
            required: 10,
            actual: 5,
        };
        assert_eq!(
            err.user_message(),
            "Moving average requires 10 points, but only 5 available"
        );
    }

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let plot_err: PlotError = io_err.into();
        assert!(matches!(plot_err, PlotError::FileIo(_)));
    }
}
