use chrono::{DateTime, NaiveDateTime, Utc};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::data;
use crate::error::PlotError;
use crate::state::{self, WEViolation};

#[derive(Serialize, Deserialize)]
pub struct ViewConfig {
    pub show_grid: bool,
    pub show_legend: bool,
    pub line_style: state::LineStyle,
    pub show_spc_limits: bool,
    pub sigma_multiplier: f64,
    pub show_sigma_zones: bool,
    pub show_outliers: bool,
    pub outlier_threshold: f64,
    pub show_moving_avg: bool,
    pub ma_window: usize,
    pub show_ewma: bool,
    pub ewma_lambda: f64,
    pub show_regression: bool,
    pub regression_order: usize,
    pub show_histogram: bool,
    pub histogram_bins: usize,
    pub show_boxplot: bool,
    pub show_capability: bool,
    pub spec_lower: f64,
    pub spec_upper: f64,
    pub show_we_rules: bool,
    pub dark_mode: bool,
}

pub struct PlotOxide {
    // Application state (Phase 2 refactoring)
    pub state: state::AppState,
}

impl Default for PlotOxide {
    fn default() -> Self {
        Self {
            state: state::AppState::default(),
        }
    }
}

impl PlotOxide {
    // Helper methods to access data through DataSource
    pub fn headers(&self) -> Vec<String> {
        self.state.column_names()
    }

    pub fn raw_data(&self) -> Vec<Vec<String>> {
        self.state.data
            .as_ref()
            .map(|ds| ds.as_row_major_string())
            .unwrap_or_default()
    }

    pub fn data(&self) -> Vec<Vec<f64>> {
        self.state.data
            .as_ref()
            .map(|ds| ds.as_row_major_f64())
            .unwrap_or_default()
    }

    pub fn get_series_color(index: usize) -> eframe::egui::Color32 {
        let colors = [
            eframe::egui::Color32::from_rgb(31, 119, 180),   // Blue
            eframe::egui::Color32::from_rgb(255, 127, 14),   // Orange
            eframe::egui::Color32::from_rgb(44, 160, 44),    // Green
            eframe::egui::Color32::from_rgb(214, 39, 40),    // Red
            eframe::egui::Color32::from_rgb(148, 103, 189),  // Purple
            eframe::egui::Color32::from_rgb(140, 86, 75),    // Brown
            eframe::egui::Color32::from_rgb(227, 119, 194),  // Pink
            eframe::egui::Color32::from_rgb(127, 127, 127),  // Gray
            eframe::egui::Color32::from_rgb(188, 189, 34),   // Yellow
            eframe::egui::Color32::from_rgb(23, 190, 207),   // Cyan
        ];
        colors[index % colors.len()]
    }

    /// Parse a value from string, detecting dates/times and converting to Unix timestamp with millisecond precision
    /// Returns (f64 value, is_timestamp, parse_info)
    pub fn parse_value(s: &str) -> (f64, bool) {
        let trimmed = s.trim();

        // Try to parse as number first
        if let Ok(num) = trimmed.parse::<f64>() {
            // Check if this looks like a Unix timestamp (seconds since epoch)
            // Valid range: 1970-01-01 (0) to 2106-02-07 (4294967295)
            // For millisecond timestamps: multiply by 1000
            if num >= 946684800.0 && num <= 2147483647.0 {
                // Likely Unix timestamp in seconds (2000-01-01 to 2038-01-19)
                return (num, true);
            } else if num >= 946684800000.0 && num <= 2147483647000.0 {
                // Likely Unix timestamp in milliseconds - convert to seconds with decimal
                return (num / 1000.0, true);
            }
            return (num, false);
        }

        // Try compact format: YYYYMMDD HHMMSS
        if trimmed.len() >= 15 && trimmed.chars().all(|c| c.is_ascii_digit() || c.is_ascii_whitespace()) {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() == 2 && parts[0].len() == 8 && parts[1].len() == 6 {
                let date_str = parts[0];
                let time_str = parts[1];
                let formatted = format!(
                    "{}-{}-{} {}:{}:{}",
                    &date_str[0..4], &date_str[4..6], &date_str[6..8],
                    &time_str[0..2], &time_str[2..4], &time_str[4..6]
                );
                if let Ok(dt) = NaiveDateTime::parse_from_str(&formatted, "%Y-%m-%d %H:%M:%S") {
                    return (dt.and_utc().timestamp() as f64, true);
                }
            }
        }

        // Try ISO 8601 formats with 'T' separator (most common for logs/APIs)
        let iso_formats = [
            "%Y-%m-%dT%H:%M:%S%.fZ",      // 2024-01-15T14:30:00.123Z
            "%Y-%m-%dT%H:%M:%SZ",         // 2024-01-15T14:30:00Z
            "%Y-%m-%dT%H:%M:%S%.f",       // 2024-01-15T14:30:00.123
            "%Y-%m-%dT%H:%M:%S",          // 2024-01-15T14:30:00
            "%Y-%m-%dT%H:%M",             // 2024-01-15T14:30
        ];

        for format in &iso_formats {
            if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, format) {
                // Preserve millisecond precision by using timestamp_millis
                let timestamp_ms = dt.and_utc().timestamp_millis() as f64;
                return (timestamp_ms / 1000.0, true);
            }
        }

        // Try common date/time formats
        let common_formats = [
            "%Y-%m-%d %H:%M:%S%.f",       // 2024-01-15 14:30:00.123
            "%Y-%m-%d %H:%M:%S",          // 2024-01-15 14:30:00
            "%Y-%m-%d %H:%M",             // 2024-01-15 14:30
            "%Y-%m-%d",                   // 2024-01-15
            "%Y/%m/%d %H:%M:%S",          // 2024/01/15 14:30:00
            "%Y/%m/%d",                   // 2024/01/15
            "%d/%m/%Y %H:%M:%S",          // 15/01/2024 14:30:00
            "%d/%m/%Y",                   // 15/01/2024
            "%m/%d/%Y %H:%M:%S",          // 01/15/2024 14:30:00
            "%m/%d/%Y",                   // 01/15/2024
            "%d-%m-%Y %H:%M:%S",          // 15-01-2024 14:30:00
            "%d-%m-%Y",                   // 15-01-2024
            "%b %d, %Y %H:%M:%S",         // Jan 15, 2024 14:30:00
            "%b %d, %Y",                  // Jan 15, 2024
            "%d %b %Y %H:%M:%S",          // 15 Jan 2024 14:30:00
            "%d %b %Y",                   // 15 Jan 2024
        ];

        for format in &common_formats {
            if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, format) {
                let timestamp_ms = dt.and_utc().timestamp_millis() as f64;
                return (timestamp_ms / 1000.0, true);
            }
        }

        // Return NaN instead of 0.0 for failed parsing to distinguish from actual 0.0 values
        (f64::NAN, false)
    }

    /// Detect if a column contains timestamp data
    /// Uses Polars' automatic date/datetime detection for accuracy
    pub fn is_column_timestamp(&self, col_index: usize) -> bool {
        self.state.data
            .as_ref()
            .map(|ds| ds.is_datetime_column(col_index))
            .unwrap_or(false)
    }

    // Check if a data point passes all active filters
    pub fn passes_filters(&self, row_idx: usize, x_val: f64, y_val: f64, y_idx: usize) -> bool {
        // Check empty data filter (only for selected Y columns)
        if self.state.filters.filter_empty && self.state.view.y_indices.contains(&y_idx) {
            let raw_data = self.raw_data();
            if row_idx < raw_data.len() && y_idx < raw_data[row_idx].len() {
                let raw_val = &raw_data[row_idx][y_idx];
                if raw_val.trim().is_empty() || raw_val == "NaN" || raw_val == "nan" {
                    return false;
                }
            }
        }

        // Check X range filter
        if let Some(min) = self.state.filters.filter_x_min {
            if x_val < min {
                return false;
            }
        }
        if let Some(max) = self.state.filters.filter_x_max {
            if x_val > max {
                return false;
            }
        }

        // Check Y range filter
        if let Some(min) = self.state.filters.filter_y_min {
            if y_val < min {
                return false;
            }
        }
        if let Some(max) = self.state.filters.filter_y_max {
            if y_val > max {
                return false;
            }
        }

        // Check outlier filter (using cached statistics)
        if self.state.filters.filter_outliers {
            if let Some(&(mean, std_dev)) = self.state.outlier_stats_cache.get(&y_idx) {
                let z_score = ((y_val - mean) / std_dev).abs();
                if z_score > self.state.filters.filter_outlier_sigma {
                    return false;
                }
            }
        }

        true
    }

    pub fn load_csv(&mut self, path: PathBuf) -> Result<(), PlotError> {
        // Use new DataSource for loading
        let data_source = data::DataSource::load(&path)?;

        // Extract data for validation (using DataSource methods)
        let headers = data_source.column_names();
        let data = data_source.as_row_major_f64();

        // Validate parsed data and report issues
        let mut nan_counts = vec![0usize; headers.len()];
        let total_rows = data.len();

        for row in &data {
            for (col_idx, &value) in row.iter().enumerate() {
                if value.is_nan() {
                    nan_counts[col_idx] += 1;
                }
            }
        }

        // Report parsing warnings for columns with significant NaN values
        let mut warnings = Vec::new();
        for (col_idx, &nan_count) in nan_counts.iter().enumerate() {
            if nan_count > 0 && col_idx < headers.len() {
                let pct = (nan_count as f64 / total_rows as f64) * 100.0;
                if pct > 5.0 {  // Warn if >5% of values failed to parse
                    warnings.push(format!(
                        "Column '{}': {}/{} values ({:.1}%) failed to parse",
                        headers[col_idx], nan_count, total_rows, pct
                    ));
                }
            }
        }

        // Store data source
        let num_cols = headers.len();
        self.state.data = Some(data_source);
        self.state.view.x_index = 0;
        self.state.view.y_indices = if num_cols > 1 { vec![1] } else { vec![] };

        // Update recent files list
        if !self.state.recent_files.contains(&path) {
            self.state.recent_files.insert(0, path.clone());
            if self.state.recent_files.len() > 5 {
                self.state.recent_files.truncate(5);
            }
        } else {
            // Move to front
            self.state.recent_files.retain(|p| p != &path);
            self.state.recent_files.insert(0, path.clone());
        }

        self.state.current_file = Some(path);
        self.state.view.reset_bounds = true; // Reset to auto-fit when loading new data

        // Detect if X column is timestamp
        self.state.view.x_is_timestamp = self.is_column_timestamp(self.state.view.x_index);

        // Show warnings if any parsing issues detected
        if !warnings.is_empty() {
            let warning_msg = format!(
                "⚠ Data parsing warnings:\n{}",
                warnings.join("\n")
            );
            self.state.ui.set_error(warning_msg);
        }

        // Invalidate caches
        self.state.outlier_stats_cache.clear();

        Ok(())
    }

    pub fn reset_view(&mut self) {
        self.state.view.reset_bounds = true;
    }

    pub fn save_config(&mut self) {
        let config = ViewConfig {
            show_grid: self.state.view.show_grid,
            show_legend: self.state.view.show_legend,
            line_style: self.state.view.line_style,
            show_spc_limits: self.state.spc.show_spc_limits,
            sigma_multiplier: self.state.spc.sigma_multiplier,
            show_sigma_zones: self.state.spc.show_sigma_zones,
            show_outliers: self.state.spc.show_outliers,
            outlier_threshold: self.state.spc.outlier_threshold,
            show_moving_avg: self.state.spc.show_moving_avg,
            ma_window: self.state.spc.ma_window,
            show_ewma: self.state.spc.show_ewma,
            ewma_lambda: self.state.spc.ewma_lambda,
            show_regression: self.state.spc.show_regression,
            regression_order: self.state.spc.regression_order,
            show_histogram: self.state.view.show_histogram,
            histogram_bins: self.state.view.histogram_bins,
            show_boxplot: self.state.view.show_boxplot,
            show_capability: self.state.spc.show_capability,
            spec_lower: self.state.spc.spec_lower,
            spec_upper: self.state.spc.spec_upper,
            show_we_rules: self.state.spc.show_we_rules,
            dark_mode: self.state.view.dark_mode,
        };

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name("view_config.json")
            .save_file()
        {
            match serde_json::to_string_pretty(&config) {
                Ok(json) => {
                    if let Err(e) = std::fs::write(&path, json) {
                        self.state.ui.set_error(format!("Failed to save config: {}", e));
                    }
                }
                Err(e) => {
                    self.state.ui.set_error(format!("Failed to serialize config: {}", e));
                }
            }
        }
    }

    pub fn load_config(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(contents) => {
                    match serde_json::from_str::<ViewConfig>(&contents) {
                        Ok(config) => {
                            self.state.view.show_grid = config.show_grid;
                            self.state.view.show_legend = config.show_legend;
                            self.state.view.line_style = config.line_style;
                            self.state.spc.show_spc_limits = config.show_spc_limits;
                            self.state.spc.sigma_multiplier = config.sigma_multiplier;
                            self.state.spc.show_sigma_zones = config.show_sigma_zones;
                            self.state.spc.show_outliers = config.show_outliers;
                            self.state.spc.outlier_threshold = config.outlier_threshold;
                            self.state.spc.show_moving_avg = config.show_moving_avg;
                            self.state.spc.ma_window = config.ma_window;
                            self.state.spc.show_ewma = config.show_ewma;
                            self.state.spc.ewma_lambda = config.ewma_lambda;
                            self.state.spc.show_regression = config.show_regression;
                            self.state.spc.regression_order = config.regression_order;
                            self.state.view.show_histogram = config.show_histogram;
                            self.state.view.histogram_bins = config.histogram_bins;
                            self.state.view.show_boxplot = config.show_boxplot;
                            self.state.spc.show_capability = config.show_capability;
                            self.state.spc.spec_lower = config.spec_lower;
                            self.state.spc.spec_upper = config.spec_upper;
                            self.state.spc.show_we_rules = config.show_we_rules;
                            self.state.view.dark_mode = config.dark_mode;
                        }
                        Err(e) => {
                            self.state.ui.set_error(format!("Failed to parse config file: {}", e));
                        }
                    }
                }
                Err(e) => {
                    self.state.ui.set_error(format!("Failed to read config file: {}", e));
                }
            }
        }
    }

    pub fn calculate_statistics(values: &[f64]) -> (f64, f64) {
        if values.is_empty() {
            return (0.0, 0.0);
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        (mean, std_dev)
    }

    pub fn calculate_median(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }

    pub fn detect_outliers(values: &[f64], threshold: f64) -> Vec<usize> {
        let (mean, std_dev) = Self::calculate_statistics(values);
        values.iter()
            .enumerate()
            .filter(|&(_, v)| ((v - mean) / std_dev).abs() > threshold)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn calculate_sma(values: &[f64], window: usize) -> Vec<[f64; 2]> {
        if values.is_empty() || window == 0 {
            return vec![];
        }
        let mut result = Vec::new();
        for i in 0..values.len() {
            if i + 1 >= window {
                let sum: f64 = values[i + 1 - window..=i].iter().sum();
                let avg = sum / window as f64;
                result.push([i as f64, avg]);
            }
        }
        result
    }

    pub fn calculate_ewma(values: &[f64], lambda: f64) -> Vec<[f64; 2]> {
        if values.is_empty() {
            return vec![];
        }
        let mut result = Vec::new();
        let mut ewma = values[0]; // Initialize with first value
        result.push([0.0, ewma]);

        for i in 1..values.len() {
            ewma = lambda * values[i] + (1.0 - lambda) * ewma;
            result.push([i as f64, ewma]);
        }
        result
    }

    pub fn downsample_lttb(data: &[[f64; 2]], threshold: usize) -> Vec<[f64; 2]> {
        // Largest-Triangle-Three-Buckets algorithm
        if data.len() <= threshold || threshold < 3 {
            return data.to_vec();
        }

        let mut sampled = Vec::with_capacity(threshold);
        sampled.push(data[0]); // Always keep first point

        let bucket_size = (data.len() - 2) as f64 / (threshold - 2) as f64;

        let mut a = 0;
        for i in 0..(threshold - 2) {
            let avg_range_start = ((i + 1) as f64 * bucket_size).floor() as usize + 1;
            let avg_range_end = ((i + 2) as f64 * bucket_size).floor() as usize + 1;
            let avg_range_end = avg_range_end.min(data.len());

            let avg_x = data[avg_range_start..avg_range_end]
                .iter()
                .map(|p| p[0])
                .sum::<f64>() / (avg_range_end - avg_range_start) as f64;
            let avg_y = data[avg_range_start..avg_range_end]
                .iter()
                .map(|p| p[1])
                .sum::<f64>() / (avg_range_end - avg_range_start) as f64;

            let range_start = (i as f64 * bucket_size).floor() as usize + 1;
            let range_end = ((i + 1) as f64 * bucket_size).floor() as usize + 1;

            let point_a = data[a];
            let mut max_area = 0.0;
            let mut max_area_point = range_start;

            for j in range_start..range_end {
                let area = ((point_a[0] - avg_x) * (data[j][1] - point_a[1])
                    - (point_a[0] - data[j][0]) * (avg_y - point_a[1]))
                    .abs();
                if area > max_area {
                    max_area = area;
                    max_area_point = j;
                }
            }

            sampled.push(data[max_area_point]);
            a = max_area_point;
        }

        sampled.push(data[data.len() - 1]); // Always keep last point
        sampled
    }

    pub fn calculate_boxplot_stats(values: &[f64]) -> Option<(f64, f64, f64, f64, f64)> {
        if values.is_empty() {
            return None;
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let len = sorted.len();
        let q1 = sorted[len / 4];
        let median = if len % 2 == 0 {
            (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
        } else {
            sorted[len / 2]
        };
        let q3 = sorted[(3 * len) / 4];
        let iqr = q3 - q1;

        let lower_whisker = sorted.iter()
            .cloned()
            .find(|&v| v >= q1 - 1.5 * iqr)
            .unwrap_or(sorted[0]);
        let upper_whisker = sorted.iter()
            .cloned()
            .rev()
            .find(|&v| v <= q3 + 1.5 * iqr)
            .unwrap_or(sorted[len - 1]);

        Some((lower_whisker, q1, median, q3, upper_whisker))
    }

    // X-bar and R chart constants (A2, D3, D4) for subgroup sizes 2-10
    pub fn get_xbarr_constants(n: usize) -> Option<(f64, f64, f64)> {
        // (A2, D3, D4) constants
        match n {
            2 => Some((1.880, 0.000, 3.267)),
            3 => Some((1.023, 0.000, 2.574)),
            4 => Some((0.729, 0.000, 2.282)),
            5 => Some((0.577, 0.000, 2.114)),
            6 => Some((0.483, 0.000, 2.004)),
            7 => Some((0.419, 0.076, 1.924)),
            8 => Some((0.373, 0.136, 1.864)),
            9 => Some((0.337, 0.184, 1.816)),
            10 => Some((0.308, 0.223, 1.777)),
            _ => None,
        }
    }

    pub fn calculate_xbarr(values: &[f64], subgroup_size: usize) -> (Vec<[f64; 2]>, Vec<[f64; 2]>, f64, f64, f64, f64, f64, f64) {
        if values.is_empty() || subgroup_size < 2 {
            return (vec![], vec![], 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        }

        let num_subgroups = values.len() / subgroup_size;
        if num_subgroups == 0 {
            return (vec![], vec![], 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        }

        let mut xbar_points = Vec::new();
        let mut r_points = Vec::new();

        // Calculate X-bar and R for each subgroup
        for i in 0..num_subgroups {
            let start = i * subgroup_size;
            let end = start + subgroup_size;
            let subgroup = &values[start..end];

            // Calculate subgroup mean (X-bar)
            let mean: f64 = subgroup.iter().sum::<f64>() / subgroup.len() as f64;
            xbar_points.push([i as f64, mean]);

            // Calculate subgroup range (R)
            let min = subgroup.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = subgroup.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let range = max - min;
            r_points.push([i as f64, range]);
        }

        // Calculate overall mean of X-bars (X-double-bar)
        let xbar_mean: f64 = xbar_points.iter().map(|p| p[1]).sum::<f64>() / xbar_points.len() as f64;

        // Calculate mean range (R-bar)
        let r_mean: f64 = r_points.iter().map(|p| p[1]).sum::<f64>() / r_points.len() as f64;

        // Get constants for control limits
        let (a2, d3, d4) = Self::get_xbarr_constants(subgroup_size).unwrap_or((0.577, 0.0, 2.114));

        // X-bar chart control limits
        let xbar_ucl = xbar_mean + a2 * r_mean;
        let xbar_lcl = xbar_mean - a2 * r_mean;

        // R chart control limits
        let r_ucl = d4 * r_mean;
        let r_lcl = d3 * r_mean;

        (xbar_points, r_points, xbar_mean, xbar_ucl, xbar_lcl, r_mean, r_ucl, r_lcl)
    }

    // p-chart for attribute data (proportion defective)
    pub fn calculate_pchart(defects: &[f64], sample_size: usize) -> (Vec<[f64; 2]>, f64, f64, f64) {
        if defects.is_empty() || sample_size == 0 {
            return (vec![], 0.0, 0.0, 0.0);
        }

        // Calculate proportions
        let proportions: Vec<[f64; 2]> = defects.iter()
            .enumerate()
            .map(|(i, &d)| {
                let p = d / sample_size as f64;
                [i as f64, p]
            })
            .collect();

        // Calculate p-bar (average proportion)
        let total_defects: f64 = defects.iter().sum();
        let total_inspected = defects.len() * sample_size;
        let p_bar = total_defects / total_inspected as f64;

        // Calculate control limits
        // UCL = p-bar + 3 * sqrt(p-bar * (1 - p-bar) / n)
        // LCL = p-bar - 3 * sqrt(p-bar * (1 - p-bar) / n)
        let std_dev = (p_bar * (1.0 - p_bar) / sample_size as f64).sqrt();
        let ucl = p_bar + 3.0 * std_dev;
        let lcl = (p_bar - 3.0 * std_dev).max(0.0); // LCL can't be negative

        (proportions, p_bar, ucl, lcl)
    }

    pub fn calculate_pareto(values: &[f64]) -> (Vec<(f64, usize)>, Vec<f64>) {
        // Create frequency map
        use std::collections::HashMap;
        let mut freq_map: HashMap<i64, usize> = HashMap::new();

        // Round values to 2 decimal places for grouping
        for &v in values {
            if v.is_finite() {
                let rounded = (v * 100.0).round() as i64;
                *freq_map.entry(rounded).or_insert(0) += 1;
            }
        }

        // Convert to vector and sort by frequency (descending)
        let mut freq_vec: Vec<(f64, usize)> = freq_map
            .into_iter()
            .map(|(k, v)| (k as f64 / 100.0, v))
            .collect();
        freq_vec.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.partial_cmp(&b.0).unwrap()));

        // Calculate cumulative percentages
        let total: usize = freq_vec.iter().map(|(_, count)| count).sum();
        let mut cumulative = 0;
        let cumulative_pct: Vec<f64> = freq_vec
            .iter()
            .map(|(_, count)| {
                cumulative += count;
                (cumulative as f64 / total as f64) * 100.0
            })
            .collect();

        (freq_vec, cumulative_pct)
    }

    pub fn calculate_histogram(values: &[f64], bins: usize) -> (Vec<[f64; 2]>, f64, f64) {
        if values.is_empty() || bins == 0 {
            return (vec![], 0.0, 0.0);
        }

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;

        if range == 0.0 {
            return (vec![], min, max);
        }

        let bin_width = range / bins as f64;
        let mut counts = vec![0; bins];

        for &v in values {
            let bin_idx = ((v - min) / bin_width).floor() as usize;
            let bin_idx = bin_idx.min(bins - 1);
            counts[bin_idx] += 1;
        }

        let hist_data = counts.iter()
            .enumerate()
            .map(|(i, &count)| {
                // Use bin left edge as X position
                let x = min + i as f64 * bin_width;
                [x, count as f64]
            })
            .collect();

        (hist_data, min, bin_width)
    }

    pub fn detect_western_electric_violations(values: &[f64]) -> Vec<usize> {
        Self::detect_western_electric_violations_detailed(values)
            .iter()
            .map(|v| v.point_index)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detect_western_electric_violations_detailed(values: &[f64]) -> Vec<WEViolation> {
        if values.len() < 8 {
            return vec![];
        }

        let (mean, std_dev) = Self::calculate_statistics(values);
        let mut violation_map: std::collections::HashMap<usize, Vec<String>> = std::collections::HashMap::new();

        // Rule 1: One point beyond 3σ (handled by outlier detection)
        // Rule 2: 2 out of 3 consecutive points beyond 2σ on same side
        // Rule 3: 4 out of 5 consecutive points beyond 1σ on same side
        // Rule 4: 8 consecutive points on same side of mean
        // Rule 5: 6 consecutive points increasing or decreasing (trend)
        // Rule 6: 14 consecutive points alternating up and down

        // Rule 4: 8 consecutive on same side
        for i in 7..values.len() {
            let slice = &values[i-7..=i];
            let above = slice.iter().all(|&v| v > mean);
            let below = slice.iter().all(|&v| v < mean);
            if above || below {
                for j in (i-7)..=i {
                    violation_map.entry(j).or_insert_with(Vec::new).push("Rule 4: 8 consecutive on same side".to_string());
                }
            }
        }

        // Rule 2: 2 out of 3 beyond 2σ
        for i in 2..values.len() {
            let slice = &values[i-2..=i];
            let above_2s = slice.iter().filter(|&&v| v > mean + 2.0 * std_dev).count();
            let below_2s = slice.iter().filter(|&&v| v < mean - 2.0 * std_dev).count();
            if above_2s >= 2 || below_2s >= 2 {
                for j in (i-2)..=i {
                    violation_map.entry(j).or_insert_with(Vec::new).push("Rule 2: 2/3 beyond 2σ".to_string());
                }
            }
        }

        // Rule 3: 4 out of 5 beyond 1σ
        for i in 4..values.len() {
            let slice = &values[i-4..=i];
            let above_1s = slice.iter().filter(|&&v| v > mean + 1.0 * std_dev).count();
            let below_1s = slice.iter().filter(|&&v| v < mean - 1.0 * std_dev).count();
            if above_1s >= 4 || below_1s >= 4 {
                for j in (i-4)..=i {
                    violation_map.entry(j).or_insert_with(Vec::new).push("Rule 3: 4/5 beyond 1σ".to_string());
                }
            }
        }

        // Rule 5: 6 consecutive increasing or decreasing
        for i in 5..values.len() {
            let slice = &values[i-5..=i];
            let increasing = slice.windows(2).all(|w| w[1] > w[0]);
            let decreasing = slice.windows(2).all(|w| w[1] < w[0]);
            if increasing || decreasing {
                for j in (i-5)..=i {
                    violation_map.entry(j).or_insert_with(Vec::new).push("Rule 5: 6 trending".to_string());
                }
            }
        }

        // Rule 6: 14 consecutive alternating
        if values.len() >= 14 {
            for i in 13..values.len() {
                let slice = &values[i-13..=i];
                let alternating = slice.windows(3).all(|w| {
                    (w[1] > w[0] && w[1] > w[2]) || (w[1] < w[0] && w[1] < w[2])
                });
                if alternating {
                    for j in (i-13)..=i {
                        violation_map.entry(j).or_insert_with(Vec::new).push("Rule 6: 14 alternating".to_string());
                    }
                }
            }
        }

        // Convert to vector of violations
        let mut violations: Vec<WEViolation> = violation_map.into_iter()
            .map(|(point_index, rules)| WEViolation { point_index, rules })
            .collect();
        violations.sort_by_key(|v| v.point_index);
        violations
    }

    pub fn calculate_process_capability(values: &[f64], lsl: f64, usl: f64) -> (f64, f64) {
        if values.is_empty() || usl <= lsl {
            return (0.0, 0.0);
        }

        let (mean, std_dev) = Self::calculate_statistics(values);

        // Cp = (USL - LSL) / (6 * sigma)
        let cp = (usl - lsl) / (6.0 * std_dev);

        // Cpk = min((USL - mean) / (3 * sigma), (mean - LSL) / (3 * sigma))
        let cpu = (usl - mean) / (3.0 * std_dev);
        let cpl = (mean - lsl) / (3.0 * std_dev);
        let cpk = cpu.min(cpl);

        (cp, cpk)
    }

    pub fn export_csv(&mut self) {
        if !self.state.has_data() {
            return;
        }

        // Open save dialog
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV Files", &["csv"])
            .set_file_name("exported_data.csv")
            .save_file()
        {
            use std::io::Write;

            let mut writer = match std::fs::File::create(&path) {
                Ok(f) => std::io::BufWriter::new(f),
                Err(e) => {
                    self.state.ui.set_error(format!("Failed to create file: {}", e));
                    return;
                }
            };

            // Get data from DataSource
            let headers = self.headers();
            let raw_data = self.raw_data();

            // Write header
            let header_line = headers.join(",");
            if let Err(e) = writeln!(writer, "{}", header_line) {
                self.state.ui.set_error(format!("Failed to write header: {}", e));
                return;
            }

            // Write data rows
            for row in &raw_data {
                let row_line = row.join(",");
                if let Err(e) = writeln!(writer, "{}", row_line) {
                    self.state.ui.set_error(format!("Failed to write row: {}", e));
                    return;
                }
            }

            if let Err(e) = writer.flush() {
                self.state.ui.set_error(format!("Failed to flush writer: {}", e));
            }
        }
    }

    pub fn linear_regression(points: &[[f64; 2]]) -> Option<(f64, f64, f64)> {
        if points.len() < 2 {
            return None;
        }

        let n = points.len() as f64;
        let sum_x: f64 = points.iter().map(|p| p[0]).sum();
        let sum_y: f64 = points.iter().map(|p| p[1]).sum();
        let sum_xy: f64 = points.iter().map(|p| p[0] * p[1]).sum();
        let sum_x2: f64 = points.iter().map(|p| p[0] * p[0]).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        // Calculate R²
        let mean_y = sum_y / n;
        let ss_tot: f64 = points.iter().map(|p| (p[1] - mean_y).powi(2)).sum();
        let ss_res: f64 = points.iter().map(|p| (p[1] - (slope * p[0] + intercept)).powi(2)).sum();
        let r_squared = 1.0 - (ss_res / ss_tot);

        Some((slope, intercept, r_squared))
    }

    pub fn polynomial_regression(points: &[[f64; 2]], order: usize) -> Option<(Vec<f64>, f64)> {
        if points.len() < order + 1 {
            return None;
        }

        let n = points.len();
        let size = order + 1;

        // Build design matrix and response vector
        let mut matrix = vec![vec![0.0; size]; size];
        let mut vector = vec![0.0; size];

        for i in 0..size {
            for j in 0..size {
                matrix[i][j] = points.iter().map(|p| p[0].powi((i + j) as i32)).sum();
            }
            vector[i] = points.iter().map(|p| p[1] * p[0].powi(i as i32)).sum();
        }

        // Gaussian elimination
        for i in 0..size {
            let mut max_row = i;
            for k in (i + 1)..size {
                if matrix[k][i].abs() > matrix[max_row][i].abs() {
                    max_row = k;
                }
            }
            matrix.swap(i, max_row);
            let temp = vector[i];
            vector[i] = vector[max_row];
            vector[max_row] = temp;

            if matrix[i][i].abs() < 1e-10 {
                return None;
            }

            for k in (i + 1)..size {
                let factor = matrix[k][i] / matrix[i][i];
                for j in i..size {
                    matrix[k][j] -= factor * matrix[i][j];
                }
                vector[k] -= factor * vector[i];
            }
        }

        // Back substitution
        let mut coeffs = vec![0.0; size];
        for i in (0..size).rev() {
            coeffs[i] = vector[i];
            for j in (i + 1)..size {
                coeffs[i] -= matrix[i][j] * coeffs[j];
            }
            coeffs[i] /= matrix[i][i];
        }

        // Calculate R²
        let mean_y: f64 = points.iter().map(|p| p[1]).sum::<f64>() / n as f64;
        let ss_tot: f64 = points.iter().map(|p| (p[1] - mean_y).powi(2)).sum();
        let ss_res: f64 = points.iter().map(|p| {
            let y_pred: f64 = coeffs.iter().enumerate()
                .map(|(i, &c)| c * p[0].powi(i as i32))
                .sum();
            (p[1] - y_pred).powi(2)
        }).sum();
        let r_squared = 1.0 - (ss_res / ss_tot);

        Some((coeffs, r_squared))
    }
}
