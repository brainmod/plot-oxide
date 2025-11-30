//! Statistical Process Control (SPC) configuration

#![allow(dead_code)]

use crate::constants::spc::*;
use serde::{Deserialize, Serialize};

/// Western Electric (WE) rule violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WEViolation {
    /// Data point index
    pub point_index: usize,
    /// Which rules were violated
    pub rules: Vec<String>,
}

/// SPC configuration manages all Statistical Process Control features
#[derive(Debug, Clone)]
pub struct SpcConfig {
    // Control limits
    /// Show SPC control limits
    pub show_spc_limits: bool,

    /// Sigma multiplier for control limits (default: 3.0)
    pub sigma_multiplier: f64,

    /// Show sigma zones (±1σ, ±2σ, ±3σ)
    pub show_sigma_zones: bool,

    // Outliers
    /// Highlight outliers
    pub show_outliers: bool,

    /// Outlier detection threshold (Z-score, default: 3.0)
    pub outlier_threshold: f64,

    // Moving averages
    /// Show moving average overlay
    pub show_moving_avg: bool,

    /// Moving average window size (default: 10)
    pub ma_window: usize,

    // EWMA (Exponentially Weighted Moving Average)
    /// Show EWMA overlay
    pub show_ewma: bool,

    /// EWMA lambda/smoothing constant (default: 0.2)
    pub ewma_lambda: f64,

    // Regression
    /// Show regression line overlay
    pub show_regression: bool,

    /// Regression polynomial order: 1=linear, 2=quadratic (default: 1)
    pub regression_order: usize,

    // Western Electric rules
    /// Show Western Electric rules violations
    pub show_we_rules: bool,

    /// Detected WE rule violations
    pub we_violations: Vec<WEViolation>,

    /// Row indices with excursions/violations
    pub excursion_rows: Vec<usize>,

    // Capability analysis
    /// Show process capability metrics (Cp, Cpk)
    pub show_capability: bool,

    /// Lower specification limit (LSL)
    pub spec_lower: f64,

    /// Upper specification limit (USL)
    pub spec_upper: f64,

    // Subgroup analysis
    /// X-bar R chart subgroup size (default: 5)
    pub xbarr_subgroup_size: usize,

    /// p-chart sample size (default: 50)
    pub pchart_sample_size: usize,
}

impl Default for SpcConfig {
    fn default() -> Self {
        Self {
            // Control limits
            show_spc_limits: false,
            sigma_multiplier: DEFAULT_SIGMA,
            show_sigma_zones: false,

            // Outliers
            show_outliers: false,
            outlier_threshold: DEFAULT_OUTLIER_THRESHOLD,

            // Moving averages
            show_moving_avg: false,
            ma_window: DEFAULT_MA_WINDOW,

            // EWMA
            show_ewma: false,
            ewma_lambda: DEFAULT_EWMA_LAMBDA,

            // Regression
            show_regression: false,
            regression_order: DEFAULT_REGRESSION_ORDER,

            // Western Electric rules
            show_we_rules: false,
            we_violations: Vec::new(),
            excursion_rows: Vec::new(),

            // Capability
            show_capability: false,
            spec_lower: DEFAULT_SPEC_LOWER,
            spec_upper: DEFAULT_SPEC_UPPER,

            // Subgroup analysis
            xbarr_subgroup_size: DEFAULT_XBARR_SUBGROUP,
            pchart_sample_size: DEFAULT_PCHART_SAMPLE,
        }
    }
}

impl SpcConfig {
    /// Create a new SpcConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all WE rule violations
    pub fn clear_violations(&mut self) {
        self.we_violations.clear();
    }

    /// Check if any SPC features are enabled
    pub fn has_active_features(&self) -> bool {
        self.show_spc_limits
            || self.show_sigma_zones
            || self.show_outliers
            || self.show_moving_avg
            || self.show_ewma
            || self.show_regression
            || self.show_we_rules
            || self.show_capability
    }

    /// Validate and clamp configuration values
    pub fn validate(&mut self) {
        // Clamp sigma multiplier to reasonable range
        self.sigma_multiplier = self.sigma_multiplier.clamp(1.0, 6.0);

        // Clamp outlier threshold
        self.outlier_threshold = self.outlier_threshold.clamp(1.0, 6.0);

        // Ensure MA window is at least 2
        if self.ma_window < 2 {
            self.ma_window = 2;
        }

        // Clamp EWMA lambda to (0, 1)
        self.ewma_lambda = self.ewma_lambda.clamp(0.01, 0.99);

        // Ensure regression order is 1 or 2
        if self.regression_order < 1 {
            self.regression_order = 1;
        } else if self.regression_order > 2 {
            self.regression_order = 2;
        }

        // Validate spec limits (LSL should be < USL)
        if self.spec_lower >= self.spec_upper {
            self.spec_upper = self.spec_lower + 1.0;
        }

        // Ensure subgroup sizes are at least 2
        if self.xbarr_subgroup_size < 2 {
            self.xbarr_subgroup_size = 2;
        }
        if self.pchart_sample_size < 2 {
            self.pchart_sample_size = 2;
        }
    }
}
