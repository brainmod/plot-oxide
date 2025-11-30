//! Data filtering configuration

#![allow(dead_code)]

use crate::constants::filters::*;

/// Filter configuration for data selection and outlier detection
#[derive(Debug, Clone)]
pub struct FilterConfig {
    /// Exclude empty cells from the dataset
    pub filter_empty: bool,

    /// Y axis minimum filter value
    pub filter_y_min: Option<f64>,

    /// Y axis maximum filter value
    pub filter_y_max: Option<f64>,

    /// X axis minimum filter value
    pub filter_x_min: Option<f64>,

    /// X axis maximum filter value
    pub filter_x_max: Option<f64>,

    /// Apply outlier filtering
    pub filter_outliers: bool,

    /// Outlier threshold (sigma, default: 3.0)
    pub filter_outlier_sigma: f64,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            filter_empty: false,
            filter_y_min: None,
            filter_y_max: None,
            filter_x_min: None,
            filter_x_max: None,
            filter_outliers: false,
            filter_outlier_sigma: DEFAULT_OUTLIER_SIGMA,
        }
    }
}

impl FilterConfig {
    /// Create a new FilterConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all filters
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Check if any filters are active
    pub fn has_active_filters(&self) -> bool {
        self.filter_empty
            || self.filter_y_min.is_some()
            || self.filter_y_max.is_some()
            || self.filter_x_min.is_some()
            || self.filter_x_max.is_some()
            || self.filter_outliers
    }

    /// Get the Y range filter as a tuple if both min and max are set
    pub fn y_range(&self) -> Option<(f64, f64)> {
        match (self.filter_y_min, self.filter_y_max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    /// Get the X range filter as a tuple if both min and max are set
    pub fn x_range(&self) -> Option<(f64, f64)> {
        match (self.filter_x_min, self.filter_x_max) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    /// Set Y range filter
    pub fn set_y_range(&mut self, min: f64, max: f64) {
        self.filter_y_min = Some(min);
        self.filter_y_max = Some(max);
    }

    /// Set X range filter
    pub fn set_x_range(&mut self, min: f64, max: f64) {
        self.filter_x_min = Some(min);
        self.filter_x_max = Some(max);
    }

    /// Clear Y range filter
    pub fn clear_y_range(&mut self) {
        self.filter_y_min = None;
        self.filter_y_max = None;
    }

    /// Clear X range filter
    pub fn clear_x_range(&mut self) {
        self.filter_x_min = None;
        self.filter_x_max = None;
    }

    /// Validate and fix any invalid filter values
    pub fn validate(&mut self) {
        // Ensure min < max for Y range
        if let (Some(min), Some(max)) = (self.filter_y_min, self.filter_y_max) {
            if min > max {
                std::mem::swap(&mut self.filter_y_min, &mut self.filter_y_max);
            }
        }

        // Ensure min < max for X range
        if let (Some(min), Some(max)) = (self.filter_x_min, self.filter_x_max) {
            if min > max {
                std::mem::swap(&mut self.filter_x_min, &mut self.filter_x_max);
            }
        }

        // Clamp outlier sigma to reasonable range
        self.filter_outlier_sigma = self.filter_outlier_sigma.clamp(1.0, 6.0);
    }
}
