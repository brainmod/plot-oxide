//! Application state management
//!
//! This module organizes the PlotOxide application state into logical components,
//! replacing the monolithic struct with focused, maintainable modules.

mod view;
mod spc;
mod filters;
mod ui;

pub use view::ViewState;
pub use spc::SpcConfig;
pub use filters::FilterConfig;
pub use ui::UiState;

use crate::data::DataSource;
use std::collections::HashMap;
use std::path::PathBuf;

/// Main application state container
///
/// Replaces the previous mega-struct by organizing related fields into
/// focused state modules.
#[derive(Default)]
pub struct AppState {
    /// Current data source (CSV or Parquet)
    pub data: Option<DataSource>,

    /// View and visualization state
    pub view: ViewState,

    /// Statistical Process Control configuration
    pub spc: SpcConfig,

    /// Data filtering configuration
    pub filters: FilterConfig,

    /// UI interaction state
    pub ui: UiState,

    /// Currently loaded file path
    pub current_file: Option<PathBuf>,

    /// Recently opened files
    pub recent_files: Vec<PathBuf>,

    /// Performance cache for outlier statistics (column_idx -> (mean, std_dev))
    pub outlier_stats_cache: HashMap<usize, (f64, f64)>,
}

impl AppState {
    /// Create a new application state with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all state (useful for resetting the application)
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Check if data is loaded
    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    /// Get the number of columns in the current dataset
    pub fn column_count(&self) -> usize {
        self.data
            .as_ref()
            .map(|ds| ds.dataframe().width())
            .unwrap_or(0)
    }

    /// Get the number of rows in the current dataset
    pub fn row_count(&self) -> usize {
        self.data
            .as_ref()
            .map(|ds| ds.dataframe().height())
            .unwrap_or(0)
    }

    /// Get column names
    pub fn column_names(&self) -> Vec<String> {
        self.data
            .as_ref()
            .map(|ds| {
                ds.dataframe()
                    .get_column_names()
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}
