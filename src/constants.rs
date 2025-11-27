//! Application-wide constants and default values
//!
//! This module centralizes all magic numbers and default values used throughout
//! the application, making them easier to maintain and configure.

/// Statistical Process Control (SPC) defaults
pub mod spc {
    /// Default sigma multiplier for control limits (±3σ)
    pub const DEFAULT_SIGMA: f64 = 3.0;

    /// Default outlier detection threshold (Z-score)
    pub const DEFAULT_OUTLIER_THRESHOLD: f64 = 3.0;

    /// Default moving average window size
    pub const DEFAULT_MA_WINDOW: usize = 10;

    /// Default EWMA lambda (smoothing constant)
    pub const DEFAULT_EWMA_LAMBDA: f64 = 0.2;

    /// Default regression order (1=linear, 2=quadratic)
    pub const DEFAULT_REGRESSION_ORDER: usize = 1;

    /// Default X-bar R chart subgroup size
    pub const DEFAULT_XBARR_SUBGROUP: usize = 5;

    /// Default p-chart sample size
    pub const DEFAULT_PCHART_SAMPLE: usize = 50;

    /// Default lower specification limit
    pub const DEFAULT_SPEC_LOWER: f64 = 0.0;

    /// Default upper specification limit
    pub const DEFAULT_SPEC_UPPER: f64 = 100.0;
}

/// Filtering defaults
pub mod filters {
    /// Default outlier filter sigma threshold
    pub const DEFAULT_OUTLIER_SIGMA: f64 = 3.0;
}

/// Performance and optimization constants
pub mod performance {
    /// Point threshold before applying LTTB downsampling
    pub const DOWNSAMPLE_THRESHOLD: usize = 5000;

    /// Maximum number of recent files to track
    pub const MAX_RECENT_FILES: usize = 10;
}

/// Plotting and visualization defaults
pub mod plot {
    /// Default number of histogram bins
    pub const DEFAULT_HISTOGRAM_BINS: usize = 20;

    /// Tolerance for point selection (in normalized plot coordinates)
    pub const POINT_SELECT_TOLERANCE: f64 = 0.0004;
}

/// UI layout defaults
pub mod layout {
    /// Left panel (series selector) default width
    pub const SERIES_PANEL_WIDTH: f32 = 200.0;

    /// Bottom panel (statistics) default height
    pub const STATS_PANEL_HEIGHT: f32 = 120.0;

    /// Right panel (data table) default width
    pub const DATA_PANEL_WIDTH: f32 = 400.0;

    /// Standard UI element padding
    pub const STANDARD_PADDING: f32 = 10.0;

    /// Table header row height
    pub const TABLE_HEADER_HEIGHT: f32 = 20.0;

    /// Minimum panel width before collapsing
    pub const MIN_PANEL_WIDTH: f32 = 150.0;

    /// Responsive layout breakpoints
    pub const COMPACT_BREAKPOINT: f32 = 800.0;
    pub const NORMAL_BREAKPOINT: f32 = 1200.0;
}

/// Date/time parsing constants
pub mod datetime {
    /// Minimum string length for timestamp parsing
    pub const MIN_TIMESTAMP_LENGTH: usize = 15;

    /// YYYYMMDD format length
    pub const DATE_FORMAT_LENGTH: usize = 8;

    /// HHMMSS format length
    pub const TIME_FORMAT_LENGTH: usize = 6;
}

/// Numeric precision constants
pub mod numeric {
    /// Matrix singularity check tolerance
    pub const SINGULARITY_TOLERANCE: f64 = 1e-10;

    /// Floating point comparison epsilon
    pub const EPSILON: f64 = 1e-12;
}

/// Configuration file paths
pub mod config {
    /// Configuration file name
    pub const CONFIG_FILE: &str = "plot-oxide.toml";
}
