#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{DateTime, NaiveDateTime, Utc};
use eframe::App;
use eframe::egui::{self, CentralPanel, ComboBox};
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use egui_plot::{Bar, BarChart, BoxElem, BoxPlot, BoxSpread, HLine, Line, Plot, Points};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

// Data module for Polars-based data handling
mod data;

// Application constants
mod constants;

// Error handling
mod error;

// Application state modules
mod state;

// Import types from state modules
use state::{LineStyle, PlotMode, WEViolation};

// Import error types
use error::PlotError;

#[derive(Serialize, Deserialize)]
struct ViewConfig {
    show_grid: bool,
    show_legend: bool,
    line_style: LineStyle,
    show_spc_limits: bool,
    sigma_multiplier: f64,
    show_sigma_zones: bool,
    show_outliers: bool,
    outlier_threshold: f64,
    show_moving_avg: bool,
    ma_window: usize,
    show_ewma: bool,
    ewma_lambda: f64,
    show_regression: bool,
    regression_order: usize,
    show_histogram: bool,
    histogram_bins: usize,
    show_boxplot: bool,
    show_capability: bool,
    spec_lower: f64,
    spec_upper: f64,
    show_we_rules: bool,
    dark_mode: bool,
}

struct PlotOxide {
    // Application state (Phase 2 refactoring)
    state: state::AppState,

    // Legacy fields for backward compatibility (to be gradually removed)
    headers: Vec<String>,
    raw_data: Vec<Vec<String>>,  // Store raw string data
    data: Vec<Vec<f64>>,         // Numeric data for plotting
}

impl Default for PlotOxide {
    fn default() -> Self {
        Self {
            state: state::AppState::default(),
            headers: Vec::new(),
            raw_data: Vec::new(),
            data: Vec::new(),
        }
    }
}

impl PlotOxide {
    fn get_series_color(index: usize) -> egui::Color32 {
        let colors = [
            egui::Color32::from_rgb(31, 119, 180),   // Blue
            egui::Color32::from_rgb(255, 127, 14),   // Orange
            egui::Color32::from_rgb(44, 160, 44),    // Green
            egui::Color32::from_rgb(214, 39, 40),    // Red
            egui::Color32::from_rgb(148, 103, 189),  // Purple
            egui::Color32::from_rgb(140, 86, 75),    // Brown
            egui::Color32::from_rgb(227, 119, 194),  // Pink
            egui::Color32::from_rgb(127, 127, 127),  // Gray
            egui::Color32::from_rgb(188, 189, 34),   // Yellow
            egui::Color32::from_rgb(23, 190, 207),   // Cyan
        ];
        colors[index % colors.len()]
    }

    fn parse_value(s: &str) -> (f64, bool) {
        // Try to parse as number first
        if let Ok(num) = s.parse::<f64>() {
            return (num, false);
        }

        // Try compact Unix timestamp format: YYYYMMDD HHMMSS
        let trimmed = s.trim();
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

        // Try common date/time formats and convert to Unix timestamp
        let date_formats = [
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d %H:%M",
            "%Y-%m-%d",
            "%d/%m/%Y %H:%M:%S",
            "%d/%m/%Y",
            "%m/%d/%Y",
            "%Y/%m/%d",
        ];

        for format in &date_formats {
            if let Ok(dt) = NaiveDateTime::parse_from_str(s, format) {
                return (dt.and_utc().timestamp() as f64, true);
            }
        }

        (0.0, false) // Default if parsing fails
    }

    fn is_column_timestamp(&self, col_index: usize) -> bool {
        if self.raw_data.is_empty() || col_index >= self.raw_data[0].len() {
            return false;
        }

        // Check first few rows to see if this column contains dates
        let sample_size = self.raw_data.len().min(10);
        let mut timestamp_count = 0;

        for row in self.raw_data.iter().take(sample_size) {
            if col_index < row.len() {
                let (_, is_timestamp) = Self::parse_value(&row[col_index]);
                if is_timestamp {
                    timestamp_count += 1;
                }
            }
        }

        // If more than half of sampled values are timestamps, consider the column as timestamp
        timestamp_count > sample_size / 2
    }

    // Check if a data point passes all active filters
    fn passes_filters(&self, row_idx: usize, x_val: f64, y_val: f64, y_idx: usize) -> bool {
        // Check empty data filter (only for selected Y columns)
        if self.state.filters.filter_empty && self.state.view.y_indices.contains(&y_idx) {
            if row_idx < self.raw_data.len() && y_idx < self.raw_data[row_idx].len() {
                let raw_val = &self.raw_data[row_idx][y_idx];
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

    fn load_csv(&mut self, path: PathBuf) -> Result<(), PlotError> {
        // Use new DataSource for loading
        let data_source = data::DataSource::load(&path)?;

        // Extract data for legacy compatibility
        let headers = data_source.column_names();
        let raw_data = data_source.as_row_major_string();
        let data = data_source.as_row_major_f64();

        // Store both new and legacy representations
        self.state.data = Some(data_source);
        self.headers = headers;
        self.raw_data = raw_data;
        self.data = data;
        self.state.view.x_index = 0;
        self.state.view.y_indices = if self.headers.len() > 1 { vec![1] } else { vec![] };

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

        // Invalidate caches
        self.state.outlier_stats_cache.clear();

        Ok(())
    }

    fn reset_view(&mut self) {
        self.state.view.reset_bounds = true;
    }

    fn save_config(&mut self) {
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

    fn load_config(&mut self) {
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

    fn calculate_statistics(values: &[f64]) -> (f64, f64) {
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

    fn calculate_median(values: &[f64]) -> f64 {
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

    fn detect_outliers(values: &[f64], threshold: f64) -> Vec<usize> {
        let (mean, std_dev) = Self::calculate_statistics(values);
        values.iter()
            .enumerate()
            .filter(|&(_, v)| ((v - mean) / std_dev).abs() > threshold)
            .map(|(i, _)| i)
            .collect()
    }

    fn calculate_sma(values: &[f64], window: usize) -> Vec<[f64; 2]> {
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

    fn calculate_ewma(values: &[f64], lambda: f64) -> Vec<[f64; 2]> {
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

    fn downsample_lttb(data: &[[f64; 2]], threshold: usize) -> Vec<[f64; 2]> {
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

    fn calculate_boxplot_stats(values: &[f64]) -> Option<(f64, f64, f64, f64, f64)> {
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
    fn get_xbarr_constants(n: usize) -> Option<(f64, f64, f64)> {
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

    fn calculate_xbarr(values: &[f64], subgroup_size: usize) -> (Vec<[f64; 2]>, Vec<[f64; 2]>, f64, f64, f64, f64, f64, f64) {
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
    fn calculate_pchart(defects: &[f64], sample_size: usize) -> (Vec<[f64; 2]>, f64, f64, f64) {
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

    fn calculate_pareto(values: &[f64]) -> (Vec<(f64, usize)>, Vec<f64>) {
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

    fn calculate_histogram(values: &[f64], bins: usize) -> (Vec<[f64; 2]>, f64, f64) {
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

    fn detect_western_electric_violations(values: &[f64]) -> Vec<usize> {
        Self::detect_western_electric_violations_detailed(values)
            .iter()
            .map(|v| v.point_index)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    fn detect_western_electric_violations_detailed(values: &[f64]) -> Vec<WEViolation> {
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

    fn calculate_process_capability(values: &[f64], lsl: f64, usl: f64) -> (f64, f64) {
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

    fn export_csv(&mut self) {
        if self.data.is_empty() {
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

            // Write header
            let header_line = self.headers.join(",");
            if let Err(e) = writeln!(writer, "{}", header_line) {
                self.state.ui.set_error(format!("Failed to write header: {}", e));
                return;
            }

            // Write data rows
            for row in &self.raw_data {
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

    fn linear_regression(points: &[[f64; 2]]) -> Option<(f64, f64, f64)> {
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

    fn polynomial_regression(points: &[[f64; 2]], order: usize) -> Option<(Vec<f64>, f64)> {
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

    /// Render the Y series selection panel (left sidebar)
    fn render_series_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let ctrl_held = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);
        let shift_held = ctx.input(|i| i.modifiers.shift);

        ui.heading("Y Series");
        ui.separator();

        let old_y_indices = self.state.view.y_indices.clone();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, header) in self.headers.iter().enumerate() {
                let is_selected = self.state.view.y_indices.contains(&i);

                // Check for sigma violations for ALL columns (independent of selection)
                let violation_color = if !self.data.is_empty() {
                    let y_values: Vec<f64> = self.data.iter().map(|row| row[i]).collect();
                    let (mean, std_dev) = Self::calculate_statistics(&y_values);
                    let mut has_3sigma = false;
                    let mut has_2sigma = false;

                    for &v in &y_values {
                        let z = ((v - mean) / std_dev).abs();
                        if z > 3.0 {
                            has_3sigma = true;
                            break;
                        } else if z > 2.0 {
                            has_2sigma = true;
                        }
                    }

                    if has_3sigma {
                        Some(egui::Color32::from_rgb(255, 50, 50))
                    } else if has_2sigma {
                        Some(egui::Color32::from_rgb(255, 165, 0))
                    } else {
                        None
                    }
                } else {
                    None
                };

                let color = if is_selected {
                    Self::get_series_color(self.state.view.y_indices.iter().position(|&x| x == i).unwrap_or(0))
                } else {
                    egui::Color32::GRAY
                };

                ui.horizontal(|ui| {
                    let response = ui.selectable_label(is_selected, header);
                    if is_selected {
                        ui.painter().circle_filled(
                            response.rect.left_center() - egui::vec2(10.0, 0.0),
                            4.0,
                            color,
                        );
                    }

                    if let Some(warn_color) = violation_color {
                        ui.colored_label(warn_color, "⚠");
                    }

                    if response.clicked() {
                        if shift_held {
                            // Range select
                            if let Some(last) = self.state.view.last_selected_series {
                                let start = last.min(i);
                                let end = last.max(i);
                                if ctrl_held {
                                    // Add range to existing selection
                                    for idx in start..=end {
                                        if !self.state.view.y_indices.contains(&idx) {
                                            self.state.view.y_indices.push(idx);
                                        }
                                    }
                                } else {
                                    // Replace with range
                                    self.state.view.y_indices = (start..=end).collect();
                                }
                            } else {
                                self.state.view.y_indices = vec![i];
                            }
                            self.state.view.last_selected_series = Some(i);
                        } else if ctrl_held {
                            // Toggle individual item
                            if is_selected {
                                self.state.view.y_indices.retain(|&x| x != i);
                            } else {
                                self.state.view.y_indices.push(i);
                            }
                            self.state.view.last_selected_series = Some(i);
                        } else {
                            // Single-select mode (replace)
                            self.state.view.y_indices = vec![i];
                            self.state.view.last_selected_series = Some(i);
                        }
                    }
                });
            }
        });

        // Clear point selection if series changed
        if self.state.view.y_indices != old_y_indices {
            self.state.view.selected_point = None;
            self.state.view.reset_bounds = true;
        }
    }

    /// Render the statistics summary panel (bottom panel)
    fn render_stats_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Statistics Summary");
        ui.separator();

        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                // Calculate all series data
                let all_series: Vec<Vec<[f64; 2]>> = self.state.view.y_indices.iter()
                    .map(|&y_idx| {
                        self.data.iter()
                            .map(|row| [row[self.state.view.x_index], row[y_idx]])
                            .collect()
                    })
                    .collect();

                for (series_idx, &y_idx) in self.state.view.y_indices.iter().enumerate() {
                    let color = Self::get_series_color(series_idx);
                    let name = &self.headers[y_idx];
                    let y_values: Vec<f64> = all_series[series_idx].iter().map(|p| p[1]).collect();

                    if !y_values.is_empty() {
                        let (mean, std_dev) = Self::calculate_statistics(&y_values);
                        let median = Self::calculate_median(&y_values);
                        let min = y_values.iter().cloned().fold(f64::INFINITY, f64::min);
                        let max = y_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

                        ui.group(|ui| {
                            ui.set_min_width(280.0);
                            ui.colored_label(color, format!("● {}", name));
                            ui.label(format!("Count: {} | Range: {:.4}", y_values.len(), max - min));
                            ui.horizontal(|ui| {
                                ui.label(format!("Min: {:.4}", min));
                                ui.separator();
                                ui.label(format!("Max: {:.4}", max));
                            });
                            ui.horizontal(|ui| {
                                ui.label(format!("Mean: {:.4}", mean));
                                ui.separator();
                                ui.label(format!("Med: {:.4}", median));
                                ui.separator();
                                ui.label(format!("σ: {:.4}", std_dev));
                            });
                            if self.state.spc.show_capability {
                                let (cp, cpk) = Self::calculate_process_capability(&y_values, self.state.spc.spec_lower, self.state.spc.spec_upper);
                                ui.horizontal(|ui| {
                                    ui.label(format!("Cp: {:.3}", cp));
                                    ui.separator();
                                    ui.label(format!("Cpk: {:.3}", cpk));
                                });
                            }
                        });
                    }
                }
            });
        });
    }

    /// Render the data table panel (right sidebar)
    fn render_data_table_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Data Table");

        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.state.ui.row_filter);
            if ui.button("✖").clicked() {
                self.state.ui.row_filter.clear();
            }
        });

        ui.separator();

        // Build list of displayed columns (X + Y series)
        let mut display_cols = vec![self.state.view.x_index];
        display_cols.extend(&self.state.view.y_indices);
        display_cols.sort_unstable();
        display_cols.dedup();

        let mut table_scroll = egui::ScrollArea::vertical().id_salt("data_table_scroll");

        if let Some(row_to_scroll) = self.state.ui.scroll_to_row.take() {
            table_scroll = table_scroll.vertical_scroll_offset((row_to_scroll as f32) * 18.0);
        }

        table_scroll.show(ui, |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(40.0).resizable(false)) // Row number
            .columns(Column::initial(80.0).resizable(true), display_cols.len())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("#");
                });
                for &col_idx in &display_cols {
                    header.col(|ui| {
                        let label = &self.headers[col_idx];
                        let sort_indicator = if self.state.ui.sort_column == Some(col_idx) {
                            if self.state.ui.sort_ascending { " ↑" } else { " ↓" }
                        } else {
                            ""
                        };
                        if ui.button(format!("{}{}", label, sort_indicator)).clicked() {
                            if self.state.ui.sort_column == Some(col_idx) {
                                self.state.ui.sort_ascending = !self.state.ui.sort_ascending;
                            } else {
                                self.state.ui.sort_column = Some(col_idx);
                                self.state.ui.sort_ascending = true;
                            }
                        }
                    });
                }
            })
            .body(|mut body| {
                // Calculate row indices (filtering and sorting)
                let mut row_indices: Vec<usize> = (0..self.raw_data.len()).collect();

                // Apply filter
                if !self.state.ui.row_filter.is_empty() {
                    let filter_lower = self.state.ui.row_filter.to_lowercase();
                    row_indices.retain(|&idx| {
                        self.raw_data[idx].iter().any(|cell| cell.to_lowercase().contains(&filter_lower))
                    });
                }

                // Apply sort
                if let Some(sort_col) = self.state.ui.sort_column {
                    row_indices.sort_by(|&a, &b| {
                        let val_a = &self.data[a][sort_col];
                        let val_b = &self.data[b][sort_col];
                        if self.state.ui.sort_ascending {
                            val_a.partial_cmp(val_b).unwrap_or(std::cmp::Ordering::Equal)
                        } else {
                            val_b.partial_cmp(val_a).unwrap_or(std::cmp::Ordering::Equal)
                        }
                    });
                }

                for &row_idx in &row_indices {
                    let row_data = &self.raw_data[row_idx];
                    let is_hovered = self.state.view.hovered_point.map(|(_, pi)| pi == row_idx).unwrap_or(false);
                    let is_selected = self.state.view.selected_point.map(|(_, pi)| pi == row_idx).unwrap_or(false);
                    let is_excursion = self.state.spc.excursion_rows.contains(&row_idx);

                    body.row(18.0, |mut row_ui| {
                        if is_selected || is_hovered {
                            row_ui.set_selected(true);
                        }

                        row_ui.col(|ui| {
                            if is_excursion {
                                ui.colored_label(egui::Color32::RED, format!("⚠ {}", row_idx + 1));
                            } else {
                                ui.label(format!("{}", row_idx + 1));
                            }
                        });

                        for &col_idx in &display_cols {
                            let cell = &row_data[col_idx];
                            row_ui.col(|ui| {
                                // Highlight X column or Y series
                                if col_idx == self.state.view.x_index {
                                    ui.strong(cell);
                                } else if self.state.view.y_indices.contains(&col_idx) {
                                    ui.strong(cell);
                                } else {
                                    ui.label(cell);
                                }
                            });
                        }

                        // Detect if this row is hovered (after all columns)
                        if row_ui.response().hovered() {
                            self.state.view.table_hovered_row = Some(row_idx);
                        }
                    });
                }
            });
        });
    }

    /// Render the help dialog window
    fn render_help_dialog(&mut self, ctx: &egui::Context) {
        if self.state.view.show_help {
            egui::Window::new("⌨ Keyboard Shortcuts")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.heading("Navigation");
                    ui.label("R - Reset view");
                    ui.label("G - Toggle grid");
                    ui.label("L - Toggle legend");
                    ui.label("T - Toggle dark/light theme");
                    ui.label("H / F1 - Toggle help");
                    ui.label("ESC - Close help");

                    ui.separator();
                    ui.heading("Mouse Controls");
                    ui.label("Scroll - Zoom in/out");
                    ui.label("Shift + Scroll - Zoom X-axis only");
                    ui.label("Ctrl + Scroll - Zoom Y-axis only");
                    ui.label("Drag - Pan view");
                    ui.label("Alt + Drag - Box zoom");
                    ui.label("Click point - Select point");
                    ui.label("Right-click - Context menu");

                    ui.separator();
                    ui.heading("Series Selection");
                    ui.label("Click - Select single");
                    ui.label("Ctrl/Cmd + Click - Toggle item");
                    ui.label("Shift + Click - Select range");
                    ui.label("Ctrl + Shift + Click - Add range");

                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.state.view.show_help = false;
                    }
                });
        }
    }

    /// Render the toolbar and control panels
    /// Returns false if no Y series selected (skip plot rendering), true otherwise
    fn render_toolbar_and_controls(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
        // File selection button
        ui.horizontal(|ui| {
            if ui.button("Open CSV File").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("CSV Files", &["csv"])
                    .pick_file()
                {
                    if let Err(e) = self.load_csv(path) {
                        self.state.ui.set_error(e.user_message());
                    }
                }
            }

            // Recent files menu
            if !self.state.recent_files.is_empty() {
                egui::ComboBox::from_label("Recent")
                    .selected_text("▼")
                    .show_ui(ui, |ui| {
                        // Need to clone to avoid borrow checker issues with load_csv
                        for path in self.state.recent_files.clone().iter() {
                            if let Some(name) = path.file_name() {
                                if ui.button(name.to_string_lossy()).clicked() {
                                    let path_clone = path.clone();
                                    if let Err(e) = self.load_csv(path_clone) {
                                        self.state.ui.set_error(e.user_message());
                                    }
                                }
                            }
                        }
                    });
            }

            // Display current file using Option combinator
            self.state.current_file
                .as_ref()
                .map(|file| ui.label(format!("File: {}", file.display())));
        });

        ui.separator();

        // Handle drag and drop using Option combinators
        ctx.input(|i| {
            i.raw.dropped_files
                .first()
                .and_then(|f| f.path.as_ref())
                .map(|path| {
                    if let Err(e) = self.load_csv(path.clone()) {
                        self.state.ui.set_error(e.user_message());
                    }
                });
        });

        // Show plot only if we have data
        if !self.headers.is_empty() && !self.data.is_empty() {
            // Axis selection and controls
            ui.horizontal(|ui| {
                let old_x = self.state.view.x_index;
                let old_use_row = self.state.view.use_row_index;

                ui.checkbox(&mut self.state.view.use_row_index, "Row Index");

                if !self.state.view.use_row_index {
                    ComboBox::from_label("X Axis")
                        .selected_text(&self.headers[self.state.view.x_index])
                        .show_ui(ui, |ui| {
                            for (i, h) in self.headers.iter().enumerate() {
                                ui.selectable_value(&mut self.state.view.x_index, i, h);
                            }
                        });
                } else {
                    ui.label("X Axis: Row #");
                }

                // Update timestamp flag if X axis changed
                if old_x != self.state.view.x_index || old_use_row != self.state.view.use_row_index {
                    if !self.state.view.use_row_index {
                        self.state.view.x_is_timestamp = self.is_column_timestamp(self.state.view.x_index);
                    } else {
                        self.state.view.x_is_timestamp = false;
                    }
                    self.state.view.reset_bounds = true;
                }

                ui.separator();

                if ui.button("Reset View").clicked() {
                    self.reset_view();
                }
            });

            // Plot options
            ui.horizontal(|ui| {
                ui.label("Display:");
                ui.checkbox(&mut self.state.view.show_grid, "Grid (G)");
                ui.checkbox(&mut self.state.view.show_legend, "Legend (L)");
                ui.checkbox(&mut self.state.view.allow_zoom, "Zoom");
                ui.checkbox(&mut self.state.view.allow_drag, "Pan");
                ui.checkbox(&mut self.state.view.show_data_table, "Data Table");
                ui.checkbox(&mut self.state.view.show_stats_panel, "Statistics");

                ui.separator();
                ui.label("Export:");
                if ui.button("CSV").clicked() {
                    self.export_csv();
                }
                ui.separator();
                ui.label("Config:");
                if ui.button("Save").clicked() {
                    self.save_config();
                }
                if ui.button("Load").clicked() {
                    self.load_config();
                }

                ui.separator();
                if ui.button(if self.state.view.dark_mode { "🌙 Dark" } else { "☀ Light" }).clicked() {
                    self.state.view.dark_mode = !self.state.view.dark_mode;
                }

                ui.separator();

                ui.label("Plot Mode:");
                ui.radio_value(&mut self.state.view.plot_mode, PlotMode::Scatter, "Scatter/Line");
                ui.radio_value(&mut self.state.view.plot_mode, PlotMode::Histogram, "Histogram");
                if self.state.view.plot_mode == PlotMode::Histogram {
                    ui.label("Bins:");
                    ui.add(egui::Slider::new(&mut self.state.view.histogram_bins, 5..=50));
                }
                ui.radio_value(&mut self.state.view.plot_mode, PlotMode::BoxPlot, "Box Plot");
                ui.radio_value(&mut self.state.view.plot_mode, PlotMode::Pareto, "Pareto");
                ui.radio_value(&mut self.state.view.plot_mode, PlotMode::XbarR, "X-bar & R");
                if self.state.view.plot_mode == PlotMode::XbarR {
                    ui.label("Subgroup:");
                    ui.add(egui::Slider::new(&mut self.state.spc.xbarr_subgroup_size, 2..=10));
                }
                ui.radio_value(&mut self.state.view.plot_mode, PlotMode::PChart, "p-chart");
                if self.state.view.plot_mode == PlotMode::PChart {
                    ui.label("Sample n:");
                    ui.add(egui::Slider::new(&mut self.state.spc.pchart_sample_size, 10..=200));
                }

                if self.state.view.plot_mode == PlotMode::Scatter {
                    ui.separator();
                    ui.label("Style:");
                    ui.radio_value(&mut self.state.view.line_style, LineStyle::Line, "Line");
                    ui.radio_value(&mut self.state.view.line_style, LineStyle::Points, "Points");
                    ui.radio_value(&mut self.state.view.line_style, LineStyle::LineAndPoints, "Both");
                }

                ui.separator();
                if ui.button("? Help").clicked() {
                    self.state.view.show_help = !self.state.view.show_help;
                }
            });

            // Only show SPC/Analysis controls in Scatter mode
            if self.state.view.plot_mode == PlotMode::Scatter {
                // SPC Controls
                ui.horizontal(|ui| {
                    ui.label("SPC:");
                    ui.checkbox(&mut self.state.spc.show_spc_limits, "Control Limits");
                    if self.state.spc.show_spc_limits {
                        ui.label("σ:");
                        ui.add(egui::Slider::new(&mut self.state.spc.sigma_multiplier, 1.0..=6.0).step_by(0.5));
                    }
                    ui.checkbox(&mut self.state.spc.show_sigma_zones, "Zones");
                    ui.checkbox(&mut self.state.spc.show_we_rules, "WE Rules");
                    ui.separator();
                    ui.checkbox(&mut self.state.spc.show_capability, "Cp/Cpk");
                    if self.state.spc.show_capability {
                        ui.label("LSL:");
                        ui.add(egui::DragValue::new(&mut self.state.spc.spec_lower).speed(0.1));
                        ui.label("USL:");
                        ui.add(egui::DragValue::new(&mut self.state.spc.spec_upper).speed(0.1));
                    }
                    ui.separator();
                    ui.checkbox(&mut self.state.spc.show_outliers, "Outliers");
                    if self.state.spc.show_outliers {
                        ui.label("Z:");
                        ui.add(egui::Slider::new(&mut self.state.spc.outlier_threshold, 2.0..=6.0).step_by(0.5));
                    }
                    ui.separator();
                    ui.checkbox(&mut self.state.spc.show_moving_avg, "MA");
                    if self.state.spc.show_moving_avg {
                        ui.label("Win:");
                        ui.add(egui::Slider::new(&mut self.state.spc.ma_window, 3..=50));
                    }
                    ui.checkbox(&mut self.state.spc.show_ewma, "EWMA");
                    if self.state.spc.show_ewma {
                        ui.label("λ:");
                        ui.add(egui::Slider::new(&mut self.state.spc.ewma_lambda, 0.05..=0.5).step_by(0.05));
                    }
                    ui.separator();
                    ui.checkbox(&mut self.state.spc.show_regression, "Regression");
                    if self.state.spc.show_regression {
                        ui.label("Order:");
                        ui.add(egui::Slider::new(&mut self.state.spc.regression_order, 1..=4));
                    }
                });

                // Data Filtering Controls
                ui.horizontal(|ui| {
                    ui.label("Filters:");
                    ui.checkbox(&mut self.state.filters.filter_empty, "Empty (Y series only)");

                    ui.separator();

                    // Y min/max filter
                    ui.label("Y Range:");
                    let mut y_min_enabled = self.state.filters.filter_y_min.is_some();
                    ui.checkbox(&mut y_min_enabled, "Min");
                    if y_min_enabled {
                        let mut val = self.state.filters.filter_y_min.unwrap_or(0.0);
                        ui.add(egui::DragValue::new(&mut val).speed(0.1));
                        self.state.filters.filter_y_min = Some(val);
                    } else {
                        self.state.filters.filter_y_min = None;
                    }

                    let mut y_max_enabled = self.state.filters.filter_y_max.is_some();
                    ui.checkbox(&mut y_max_enabled, "Max");
                    if y_max_enabled {
                        let mut val = self.state.filters.filter_y_max.unwrap_or(100.0);
                        ui.add(egui::DragValue::new(&mut val).speed(0.1));
                        self.state.filters.filter_y_max = Some(val);
                    } else {
                        self.state.filters.filter_y_max = None;
                    }

                    ui.separator();

                    // X range filter
                    ui.label("X Range:");
                    let mut x_min_enabled = self.state.filters.filter_x_min.is_some();
                    ui.checkbox(&mut x_min_enabled, "Min");
                    if x_min_enabled {
                        let mut val = self.state.filters.filter_x_min.unwrap_or(0.0);
                        ui.add(egui::DragValue::new(&mut val).speed(0.1));
                        self.state.filters.filter_x_min = Some(val);
                    } else {
                        self.state.filters.filter_x_min = None;
                    }

                    let mut x_max_enabled = self.state.filters.filter_x_max.is_some();
                    ui.checkbox(&mut x_max_enabled, "Max");
                    if x_max_enabled {
                        let mut val = self.state.filters.filter_x_max.unwrap_or(100.0);
                        ui.add(egui::DragValue::new(&mut val).speed(0.1));
                        self.state.filters.filter_x_max = Some(val);
                    } else {
                        self.state.filters.filter_x_max = None;
                    }

                    ui.separator();

                    // Outlier filter
                    ui.checkbox(&mut self.state.filters.filter_outliers, "Filter Outliers");
                    if self.state.filters.filter_outliers {
                        ui.label("Z:");
                        ui.add(egui::Slider::new(&mut self.state.filters.filter_outlier_sigma, 2.0..=6.0).step_by(0.5));
                    }
                });
            }

            ui.separator();

            // Check if no Y series selected
            if self.state.view.y_indices.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.label("Select at least one Y series to plot");
                });
                return false;
            }

            true
        } else {
            false
        }
    }

    /// Render the main plot area
    fn render_plot(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Pre-calculate statistics for outlier filtering (performance optimization)
        if self.state.filters.filter_outliers {
            self.state.outlier_stats_cache.clear();
            for &y_idx in &self.state.view.y_indices {
                let y_values: Vec<f64> = self.data.iter().map(|row| row[y_idx]).collect();
                let stats = Self::calculate_statistics(&y_values);
                self.state.outlier_stats_cache.insert(y_idx, stats);
            }
        }

        // Create data for all series with filtering
        let all_series: Vec<Vec<[f64; 2]>> = self.state.view.y_indices.iter()
            .map(|&y_idx| {
                let points: Vec<[f64; 2]> = self.data.iter()
                    .enumerate()
                    .filter_map(|(row_idx, row)| {
                        let x_val = if self.state.view.use_row_index {
                            row_idx as f64
                        } else {
                            row[self.state.view.x_index]
                        };
                        let y_val = row[y_idx];

                        // Apply filters
                        if self.passes_filters(row_idx, x_val, y_val, y_idx) {
                            Some([x_val, y_val])
                        } else {
                            None
                        }
                    })
                    .collect();

                // Downsample if dataset is large
                if points.len() > self.state.view.downsample_threshold {
                    Self::downsample_lttb(&points, self.state.view.downsample_threshold)
                } else {
                    points
                }
            })
            .collect();

        // Detect modifier keys for constrained zoom
        let shift_held = ctx.input(|i| i.modifiers.shift);
        let ctrl_held = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);
        let alt_held = ctx.input(|i| i.modifiers.alt);

        let mut plot = Plot::new("plot")
            .allow_zoom(self.state.view.allow_zoom)
            .allow_drag(self.state.view.allow_drag && !alt_held)
            .allow_boxed_zoom(self.state.view.allow_zoom && alt_held)
            .allow_scroll(self.state.view.allow_zoom)
            .show_grid(self.state.view.show_grid)
            .height(ui.available_height());

        // Apply axis-locked zoom if modifiers held
        if shift_held && self.state.view.allow_zoom {
            plot = plot.allow_zoom([true, false]).allow_boxed_zoom(false); // X-only
        } else if ctrl_held && self.state.view.allow_zoom {
            plot = plot.allow_zoom([false, true]).allow_boxed_zoom(false); // Y-only
        }

        if self.state.view.reset_bounds {
            plot = plot.reset();
            self.state.view.reset_bounds = false;
        }

        if self.state.view.show_legend {
            plot = plot.legend(egui_plot::Legend::default().position(egui_plot::Corner::RightTop));
        }

        // Add custom axis formatters for timestamps
        if self.state.view.x_is_timestamp {
            plot = plot.x_axis_formatter(|mark, _range| {
                let dt = DateTime::<Utc>::from_timestamp(mark.value as i64, 0);
                if let Some(dt) = dt {
                    dt.format("%Y-%m-%d\n%H:%M").to_string()
                } else {
                    format!("{:.0}", mark.value)
                }
            });
        }

        let plot_response = plot.show(ui, |plot_ui| {
            match self.state.view.plot_mode {
                PlotMode::Scatter => {
                    // Plot each series in scatter mode
                    for (series_idx, (&y_idx, points_data)) in self.state.view.y_indices.iter().zip(&all_series).enumerate() {
                        let color = Self::get_series_color(series_idx);
                        let name = &self.headers[y_idx];

                // Draw sigma zone lines if enabled
                if self.state.spc.show_sigma_zones && !points_data.is_empty() {
                    let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                    let (mean, std_dev) = Self::calculate_statistics(&y_values);

                    // ±1σ lines (blue)
                    plot_ui.hline(HLine::new(format!("{} +1σ", name), mean + 1.0 * std_dev)
                        .color(egui::Color32::from_rgb(150, 150, 255))
                        .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                        .width(1.0));
                    plot_ui.hline(HLine::new(format!("{} -1σ", name), mean - 1.0 * std_dev)
                        .color(egui::Color32::from_rgb(150, 150, 255))
                        .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                        .width(1.0));

                    // ±2σ lines (orange)
                    plot_ui.hline(HLine::new(format!("{} +2σ", name), mean + 2.0 * std_dev)
                        .color(egui::Color32::from_rgb(255, 200, 100))
                        .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                        .width(1.0));
                    plot_ui.hline(HLine::new(format!("{} -2σ", name), mean - 2.0 * std_dev)
                        .color(egui::Color32::from_rgb(255, 200, 100))
                        .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                        .width(1.0));

                    // ±3σ lines (red)
                    plot_ui.hline(HLine::new(format!("{} +3σ", name), mean + 3.0 * std_dev)
                        .color(egui::Color32::from_rgb(255, 150, 150))
                        .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                        .width(1.0));
                    plot_ui.hline(HLine::new(format!("{} -3σ", name), mean - 3.0 * std_dev)
                        .color(egui::Color32::from_rgb(255, 150, 150))
                        .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                        .width(1.0));
                }

                // Draw specification limits if capability enabled
                if self.state.spc.show_capability {
                    plot_ui.hline(
                        HLine::new("LSL", self.state.spc.spec_lower)
                            .color(egui::Color32::from_rgb(255, 140, 0))
                            .style(egui_plot::LineStyle::Solid)
                            .width(2.0),
                    );
                    plot_ui.hline(
                        HLine::new("USL", self.state.spc.spec_upper)
                            .color(egui::Color32::from_rgb(255, 140, 0))
                            .style(egui_plot::LineStyle::Solid)
                            .width(2.0),
                    );
                }

                // Draw SPC control limits if enabled
                if self.state.spc.show_spc_limits {
                    let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                    let (mean, std_dev) = Self::calculate_statistics(&y_values);
                    let ucl = mean + self.state.spc.sigma_multiplier * std_dev;
                    let lcl = mean - self.state.spc.sigma_multiplier * std_dev;

                    // Center line (mean)
                    plot_ui.hline(
                        HLine::new(format!("{} Mean", name), mean)
                            .color(color)
                            .style(egui_plot::LineStyle::Dashed { length: 8.0 })
                            .width(1.5),
                    );

                    // Upper control limit
                    plot_ui.hline(
                        HLine::new(format!("{} UCL", name), ucl)
                            .color(egui::Color32::RED)
                            .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                            .width(2.0),
                    );

                    // Lower control limit
                    plot_ui.hline(
                        HLine::new(format!("{} LCL", name), lcl)
                            .color(egui::Color32::RED)
                            .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                            .width(2.0),
                    );
                }

                // Draw data series
                match self.state.view.line_style {
                    LineStyle::Line => {
                        plot_ui.line(Line::new(name, points_data.clone()).color(color));
                    }
                    LineStyle::Points => {
                        plot_ui.points(Points::new(name, points_data.clone()).radius(3.0).color(color));
                    }
                    LineStyle::LineAndPoints => {
                        plot_ui.line(Line::new(name, points_data.clone()).color(color));
                        plot_ui.points(Points::new(name, points_data.clone()).radius(3.0).color(color));
                    }
                }

                // Highlight outliers
                if self.state.spc.show_outliers {
                    let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                    let outlier_indices = Self::detect_outliers(&y_values, self.state.spc.outlier_threshold);
                    let outlier_points: Vec<[f64; 2]> = outlier_indices.iter()
                        .map(|&i| points_data[i])
                        .collect();

                    if !outlier_points.is_empty() {
                        plot_ui.points(
                            Points::new(format!("{} Outliers", name), outlier_points)
                                .color(egui::Color32::RED)
                                .filled(true)
                                .radius(5.0)
                                .shape(egui_plot::MarkerShape::Diamond),
                        );
                    }
                }

                // Highlight Western Electric violations
                if self.state.spc.show_we_rules {
                    let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                    let we_indices = Self::detect_western_electric_violations(&y_values);
                    let we_points: Vec<[f64; 2]> = we_indices.iter()
                        .map(|&i| points_data[i])
                        .collect();

                    if !we_points.is_empty() {
                        plot_ui.points(
                            Points::new(format!("{} WE Violations", name), we_points)
                                .color(egui::Color32::from_rgb(255, 165, 0))
                                .filled(false)
                                .radius(7.0)
                                .shape(egui_plot::MarkerShape::Square),
                        );
                    }
                }

                // Highlight selected point
                if let Some((sel_series, sel_point)) = self.state.view.selected_point {
                    if series_idx == sel_series && sel_point < points_data.len() {
                        plot_ui.points(
                            Points::new("", vec![points_data[sel_point]])
                                .color(egui::Color32::from_rgb(255, 215, 0))
                                .filled(false)
                                .radius(10.0)
                                .shape(egui_plot::MarkerShape::Circle),
                        );
                    }
                }

                // Highlight table-hovered point (use white for visibility)
                if let Some(row_idx) = self.state.view.table_hovered_row {
                    if row_idx < points_data.len() {
                        plot_ui.points(
                            Points::new("", vec![points_data[row_idx]])
                                .color(egui::Color32::WHITE)
                                .filled(true)
                                .radius(6.0)
                                .shape(egui_plot::MarkerShape::Circle),
                        );
                    }
                }

                // Draw moving average if enabled
                if self.state.spc.show_moving_avg && points_data.len() >= self.state.spc.ma_window {
                    let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                    let ma_y = Self::calculate_sma(&y_values, self.state.spc.ma_window);

                    // Map MA y-values to actual x-values
                    let ma_points: Vec<[f64; 2]> = ma_y.iter()
                        .map(|&[idx, y_val]| {
                            let actual_x = points_data[idx as usize][0];
                            [actual_x, y_val]
                        })
                        .collect();

                    if !ma_points.is_empty() {
                        let ma_color = egui::Color32::from_rgb(100, 100, 100);
                        plot_ui.line(
                            Line::new(format!("{} MA({})", name, self.state.spc.ma_window), ma_points)
                                .color(ma_color)
                                .style(egui_plot::LineStyle::Dashed { length: 5.0 })
                                .width(1.5),
                        );
                    }
                }

                // Draw EWMA if enabled
                if self.state.spc.show_ewma && !points_data.is_empty() {
                    let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                    let ewma_y = Self::calculate_ewma(&y_values, self.state.spc.ewma_lambda);

                    // Map EWMA y-values to actual x-values
                    let ewma_points: Vec<[f64; 2]> = ewma_y.iter()
                        .map(|&[idx, y_val]| {
                            let actual_x = points_data[idx as usize][0];
                            [actual_x, y_val]
                        })
                        .collect();

                    if !ewma_points.is_empty() {
                        let ewma_color = egui::Color32::from_rgb(80, 150, 80);
                        plot_ui.line(
                            Line::new(format!("{} EWMA(λ={:.2})", name, self.state.spc.ewma_lambda), ewma_points)
                                .color(ewma_color)
                                .style(egui_plot::LineStyle::Solid)
                                .width(2.0),
                        );
                    }
                }

                // Draw regression if enabled
                if self.state.spc.show_regression && points_data.len() >= self.state.spc.regression_order + 1 {
                    if self.state.spc.regression_order == 1 {
                        // Linear regression
                        if let Some((slope, intercept, r2)) = Self::linear_regression(points_data) {
                            let x_min = points_data.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                            let x_max = points_data.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);

                            let reg_line = vec![
                                [x_min, slope * x_min + intercept],
                                [x_max, slope * x_max + intercept],
                            ];

                            plot_ui.line(
                                Line::new(format!("{} Lin (R²={:.3})", name, r2), reg_line)
                                    .color(egui::Color32::from_rgb(180, 100, 180))
                                    .style(egui_plot::LineStyle::Solid)
                                    .width(2.0),
                            );
                        }
                    } else {
                        // Polynomial regression
                        if let Some((coeffs, r2)) = Self::polynomial_regression(points_data, self.state.spc.regression_order) {
                            let x_min = points_data.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                            let x_max = points_data.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);

                            let num_points = 100;
                            let step = (x_max - x_min) / num_points as f64;
                            let poly_line: Vec<[f64; 2]> = (0..=num_points)
                                .map(|i| {
                                    let x = x_min + i as f64 * step;
                                    let y: f64 = coeffs.iter().enumerate()
                                        .map(|(i, &c)| c * x.powi(i as i32))
                                        .sum();
                                    [x, y]
                                })
                                .collect();

                            plot_ui.line(
                                Line::new(format!("{} Poly{} (R²={:.3})", name, self.state.spc.regression_order, r2), poly_line)
                                    .color(egui::Color32::from_rgb(180, 100, 180))
                                    .style(egui_plot::LineStyle::Solid)
                                    .width(2.0),
                            );
                        }
                    }
                }

                    }
                }
                PlotMode::Histogram => {
                    // Histogram mode - show histogram with proper bin widths
                    // Note: We ignore the X-axis data and work only with Y-values
                    for (series_idx, &y_idx) in self.state.view.y_indices.iter().enumerate() {
                        let color = Self::get_series_color(series_idx);
                        let name = &self.headers[y_idx];

                        // Get Y values directly from data (not from all_series which has X,Y pairs)
                        let y_values: Vec<f64> = self.data.iter()
                            .map(|row| row[y_idx])
                            .filter(|&v| !v.is_nan() && v.is_finite())
                            .collect();

                        let (hist_data, _min, bin_width) = Self::calculate_histogram(&y_values, self.state.view.histogram_bins);

                        if !hist_data.is_empty() {
                            // Calculate bar width based on bin_width and number of series
                            let bar_width = bin_width * 0.9 / self.state.view.y_indices.len() as f64;
                            let offset = (series_idx as f64 - (self.state.view.y_indices.len() - 1) as f64 / 2.0) * bar_width;

                            let bars: Vec<Bar> = hist_data.iter()
                                .map(|&[x, count]| {
                                    // X is the bin left edge, center within bin and add offset for multiple series
                                    Bar::new(x + bin_width / 2.0 + offset, count).width(bar_width)
                                })
                                .collect();

                            plot_ui.bar_chart(
                                BarChart::new(name.clone(), bars)
                                    .color(color)
                            );
                        }
                    }
                }
                PlotMode::BoxPlot => {
                    // Box plot mode - only show box plots
                    for (series_idx, (&y_idx, points_data)) in self.state.view.y_indices.iter().zip(&all_series).enumerate() {
                        let color = Self::get_series_color(series_idx);
                        let name = &self.headers[y_idx];
                        let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();

                        if let Some((lower_whisker, q1, median, q3, upper_whisker)) = Self::calculate_boxplot_stats(&y_values) {
                            let x_pos = series_idx as f64;

                            let box_elem = BoxElem::new(x_pos, BoxSpread::new(lower_whisker, q1, median, q3, upper_whisker));
                            plot_ui.box_plot(
                                BoxPlot::new(format!("{}", name), vec![box_elem])
                                    .color(color)
                            );
                        }
                    }
                }
                PlotMode::Pareto => {
                    // Pareto chart mode - frequency bars + cumulative line
                    for (series_idx, &y_idx) in self.state.view.y_indices.iter().enumerate() {
                        let color = Self::get_series_color(series_idx);
                        let name = &self.headers[y_idx];

                        // Get Y values directly from data
                        let y_values: Vec<f64> = self.data.iter()
                            .map(|row| row[y_idx])
                            .filter(|&v| !v.is_nan() && v.is_finite())
                            .collect();

                        let (freq_data, cumulative_pct) = Self::calculate_pareto(&y_values);

                        if !freq_data.is_empty() {
                            // Draw frequency bars
                            let bar_width = 0.8 / self.state.view.y_indices.len() as f64;
                            let offset = (series_idx as f64 - (self.state.view.y_indices.len() - 1) as f64 / 2.0) * bar_width;

                            let bars: Vec<Bar> = freq_data.iter()
                                .enumerate()
                                .map(|(i, &(_, count))| {
                                    Bar::new(i as f64 + offset, count as f64).width(bar_width)
                                })
                                .collect();

                            plot_ui.bar_chart(
                                BarChart::new(format!("{} Frequency", name), bars)
                                    .color(color)
                            );

                            // Draw cumulative percentage line (scaled to match bar heights)
                            let max_count = freq_data.iter().map(|(_, c)| c).max().unwrap_or(&1);
                            let scale_factor = *max_count as f64 / 100.0;

                            let cumulative_line: Vec<[f64; 2]> = cumulative_pct.iter()
                                .enumerate()
                                .map(|(i, &pct)| [i as f64, pct * scale_factor])
                                .collect();

                            plot_ui.line(
                                Line::new(format!("{} Cumulative %", name), cumulative_line)
                                    .color(egui::Color32::RED)
                                    .style(egui_plot::LineStyle::Solid)
                                    .width(2.5)
                            );

                            // Draw 80% line (Pareto principle)
                            let line_80 = 80.0 * scale_factor;
                            plot_ui.hline(
                                HLine::new("80% Line", line_80)
                                    .color(egui::Color32::from_rgb(255, 165, 0))
                                    .style(egui_plot::LineStyle::Dashed { length: 8.0 })
                                    .width(2.0)
                            );
                        }
                    }
                }
                PlotMode::XbarR => {
                    // X-bar and R chart mode - shows process mean and range control charts
                    // This mode requires displaying TWO charts, so we'll show only the first Y-series
                    if let Some(&y_idx) = self.state.view.y_indices.first() {
                        let color = Self::get_series_color(0);
                        let name = &self.headers[y_idx];

                        // Get Y values directly from data
                        let y_values: Vec<f64> = self.data.iter()
                            .map(|row| row[y_idx])
                            .filter(|&v| !v.is_nan() && v.is_finite())
                            .collect();

                        let (xbar_points, r_points, xbar_mean, xbar_ucl, xbar_lcl, r_mean, r_ucl, r_lcl) =
                            Self::calculate_xbarr(&y_values, self.state.spc.xbarr_subgroup_size);

                        if !xbar_points.is_empty() {
                            // Note: egui_plot doesn't support dual Y-axes easily
                            // We'll draw X-bar and R on the same plot with different colors

                            // Draw X-bar points and lines
                            plot_ui.line(Line::new(format!("{} X-bar", name), xbar_points.clone()).color(color));
                            plot_ui.points(Points::new(format!("{} X-bar", name), xbar_points.clone()).radius(4.0).color(color));

                            // X-bar control limits
                            plot_ui.hline(
                                HLine::new(format!("{} X-bar Mean", name), xbar_mean)
                                    .color(color)
                                    .style(egui_plot::LineStyle::Dashed { length: 8.0 })
                                    .width(2.0)
                            );
                            plot_ui.hline(
                                HLine::new(format!("{} X-bar UCL", name), xbar_ucl)
                                    .color(egui::Color32::RED)
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                    .width(2.0)
                            );
                            plot_ui.hline(
                                HLine::new(format!("{} X-bar LCL", name), xbar_lcl)
                                    .color(egui::Color32::RED)
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                    .width(2.0)
                            );

                            // Draw R points and lines (with different color)
                            let r_color = egui::Color32::from_rgb(255, 127, 14); // Orange
                            plot_ui.line(Line::new(format!("{} R-chart", name), r_points.clone()).color(r_color));
                            plot_ui.points(Points::new(format!("{} R-chart", name), r_points.clone()).radius(4.0).color(r_color).shape(egui_plot::MarkerShape::Diamond));

                            // R control limits
                            plot_ui.hline(
                                HLine::new(format!("{} R Mean", name), r_mean)
                                    .color(r_color)
                                    .style(egui_plot::LineStyle::Dashed { length: 8.0 })
                                    .width(2.0)
                            );
                            plot_ui.hline(
                                HLine::new(format!("{} R UCL", name), r_ucl)
                                    .color(egui::Color32::from_rgb(200, 0, 0))
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                    .width(2.0)
                            );
                            if r_lcl > 0.0 {
                                plot_ui.hline(
                                    HLine::new(format!("{} R LCL", name), r_lcl)
                                        .color(egui::Color32::from_rgb(200, 0, 0))
                                        .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                        .width(2.0)
                                );
                            }
                        }
                    }
                }
                PlotMode::PChart => {
                    // p-chart mode - proportion defective control chart
                    if let Some(&y_idx) = self.state.view.y_indices.first() {
                        let color = Self::get_series_color(0);
                        let name = &self.headers[y_idx];

                        // Get Y values (number of defects per sample)
                        let defects: Vec<f64> = self.data.iter()
                            .map(|row| row[y_idx])
                            .filter(|&v| !v.is_nan() && v.is_finite())
                            .collect();

                        let (proportions, p_bar, ucl, lcl) = Self::calculate_pchart(&defects, self.state.spc.pchart_sample_size);

                        if !proportions.is_empty() {
                            // Draw proportion points and line
                            plot_ui.line(Line::new(format!("{} Proportion", name), proportions.clone()).color(color));
                            plot_ui.points(Points::new(format!("{} Proportion", name), proportions.clone()).radius(4.0).color(color));

                            // Draw p-bar (center line)
                            plot_ui.hline(
                                HLine::new("p-bar (avg proportion)", p_bar)
                                    .color(color)
                                    .style(egui_plot::LineStyle::Dashed { length: 8.0 })
                                    .width(2.0)
                            );

                            // Draw UCL
                            plot_ui.hline(
                                HLine::new("UCL", ucl)
                                    .color(egui::Color32::RED)
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                    .width(2.0)
                            );

                            // Draw LCL (if > 0)
                            if lcl > 0.0 {
                                plot_ui.hline(
                                    HLine::new("LCL", lcl)
                                        .color(egui::Color32::RED)
                                        .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                        .width(2.0)
                                );
                            }
                        }
                    }
                }
            }
        });

        // Calculate excursions for table highlighting
        let mut all_excursions = std::collections::HashSet::new();
        let mut all_we_violations = Vec::new();

        if self.state.spc.show_outliers || self.state.spc.show_spc_limits || self.state.spc.show_we_rules {
            for series_idx in 0..self.state.view.y_indices.len() {
                let y_values: Vec<f64> = all_series[series_idx].iter().map(|p| p[1]).collect();

                if self.state.spc.show_outliers {
                    let outliers = Self::detect_outliers(&y_values, self.state.spc.outlier_threshold);
                    all_excursions.extend(outliers);
                }

                if self.state.spc.show_spc_limits {
                    let (mean, std_dev) = Self::calculate_statistics(&y_values);
                    let ucl = mean + self.state.spc.sigma_multiplier * std_dev;
                    let lcl = mean - self.state.spc.sigma_multiplier * std_dev;

                    for (i, &v) in y_values.iter().enumerate() {
                        if v > ucl || v < lcl {
                            all_excursions.insert(i);
                        }
                    }
                }

                if self.state.spc.show_we_rules {
                    let we_detailed = Self::detect_western_electric_violations_detailed(&y_values);
                    for violation in &we_detailed {
                        all_excursions.insert(violation.point_index);
                    }
                    all_we_violations.extend(we_detailed);
                }
            }
        }
        self.state.spc.excursion_rows = all_excursions.into_iter().collect();
        self.state.spc.we_violations = all_we_violations;

        // Handle right-click context menu
        plot_response.response.context_menu(|ui| {
            if ui.button("Reset View").clicked() {
                self.state.view.reset_bounds = true;
                ui.close();
            }
            if ui.button("Toggle Grid").clicked() {
                self.state.view.show_grid = !self.state.view.show_grid;
                ui.close();
            }
            if ui.button("Toggle Legend").clicked() {
                self.state.view.show_legend = !self.state.view.show_legend;
                ui.close();
            }
            ui.separator();
            if ui.button("Clear Selection").clicked() {
                self.state.view.selected_point = None;
                ui.close();
            }
        });

        // Handle click to select point first
        let was_clicked = plot_response.response.clicked();
        let click_pos = plot_response.response.interact_pointer_pos();

        // Show hover tooltip
        if let Some(pointer_pos) = plot_response.response.hover_pos() {
            let plot_pos = plot_response.transform.value_from_position(pointer_pos);

            // Find closest point across all series
            let mut closest_series_idx = 0;
            let mut closest_point_idx = 0;
            let mut min_dist = f64::INFINITY;

            for (series_idx, points_data) in all_series.iter().enumerate() {
                for (point_idx, point) in points_data.iter().enumerate() {
                    let dx = (point[0] - plot_pos.x) / (plot_response.transform.bounds().width());
                    let dy = (point[1] - plot_pos.y) / (plot_response.transform.bounds().height());
                    let dist = dx * dx + dy * dy;

                    if dist < min_dist {
                        min_dist = dist;
                        closest_series_idx = series_idx;
                        closest_point_idx = point_idx;
                    }
                }
            }

            // Only show tooltip if close enough
            if min_dist < 0.0004 {
                self.state.view.hovered_point = Some((closest_series_idx, closest_point_idx));
                let point = &all_series[closest_series_idx][closest_point_idx];
                let y_idx = self.state.view.y_indices[closest_series_idx];

                let x_label = if self.state.view.x_is_timestamp {
                    DateTime::<Utc>::from_timestamp(point[0] as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| format!("{:.2}", point[0]))
                } else {
                    format!("{:.2}", point[0])
                };

                let y_is_timestamp = self.is_column_timestamp(y_idx);
                let y_label = if y_is_timestamp {
                    DateTime::<Utc>::from_timestamp(point[1] as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| format!("{:.2}", point[1]))
                } else {
                    format!("{:.2}", point[1])
                };

                plot_response.response.on_hover_ui(|ui| {
                    ui.label(format!("Row: {}", closest_point_idx + 1));
                    ui.label(format!("{}: {}", self.headers[self.state.view.x_index], x_label));
                    let color = Self::get_series_color(closest_series_idx);
                    ui.colored_label(color, format!("{}: {}", self.headers[y_idx], y_label));

                    // Show WE rule violations if any
                    if self.state.spc.show_we_rules {
                        for violation in &self.state.spc.we_violations {
                            if violation.point_index == closest_point_idx {
                                ui.separator();
                                ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "⚠ WE Rule Violations:");
                                for rule in &violation.rules {
                                    ui.label(format!("  • {}", rule));
                                }
                                break;
                            }
                        }
                    }
                });
            } else {
                self.state.view.hovered_point = None;
            }
        } else {
            self.state.view.hovered_point = None;
        }

        // Process click event (stored before on_hover_ui consumed the response)
        if was_clicked {
            if let Some(pointer_pos) = click_pos {
                let plot_pos = plot_response.transform.value_from_position(pointer_pos);

                // Find closest point across all series
                let mut closest_series_idx = 0;
                let mut closest_point_idx = 0;
                let mut min_dist = f64::INFINITY;

                for (series_idx, points_data) in all_series.iter().enumerate() {
                    for (point_idx, point) in points_data.iter().enumerate() {
                        let dx = (point[0] - plot_pos.x) / (plot_response.transform.bounds().width());
                        let dy = (point[1] - plot_pos.y) / (plot_response.transform.bounds().height());
                        let dist = dx * dx + dy * dy;

                        if dist < min_dist {
                            min_dist = dist;
                            closest_series_idx = series_idx;
                            closest_point_idx = point_idx;
                        }
                    }
                }

                // Select if close enough, otherwise deselect
                if min_dist < 0.0004 {
                    // Clear selection if switching to a different series
                    if let Some((prev_series, _)) = self.state.view.selected_point {
                        if prev_series != closest_series_idx {
                            self.state.view.selected_point = None;
                        }
                    }
                    self.state.view.selected_point = Some((closest_series_idx, closest_point_idx));
                    self.state.ui.scroll_to_row = Some(closest_point_idx);
                } else {
                    self.state.view.selected_point = None;
                }
            }
        }
    }

}

impl App for PlotOxide {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set theme
        if self.state.view.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        // Handle keyboard shortcuts
        ctx.input(|i| {
            if i.key_pressed(egui::Key::R) {
                self.state.view.reset_bounds = true;
            }
            if i.key_pressed(egui::Key::G) {
                self.state.view.show_grid = !self.state.view.show_grid;
            }
            if i.key_pressed(egui::Key::L) {
                self.state.view.show_legend = !self.state.view.show_legend;
            }
            if i.key_pressed(egui::Key::T) {
                self.state.view.dark_mode = !self.state.view.dark_mode;
            }
            if i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::F1) {
                self.state.view.show_help = !self.state.view.show_help;
            }
            if i.key_pressed(egui::Key::Escape) {
                self.state.view.show_help = false;
            }
        });

        // Main layout using StripBuilder
        CentralPanel::default().show(ctx, |ui| {
            // Build horizontal strip layout
            let mut horizontal_strip = StripBuilder::new(ui);

            // Add series panel if we have headers
            if !self.headers.is_empty() {
                horizontal_strip = horizontal_strip.size(Size::exact(constants::layout::SERIES_PANEL_WIDTH));
            }

            // Add center (plot area) - takes remaining space
            horizontal_strip = horizontal_strip.size(Size::remainder());

            // Add data table panel if enabled
            if self.state.view.show_data_table && !self.raw_data.is_empty() {
                horizontal_strip = horizontal_strip.size(Size::exact(constants::layout::DATA_PANEL_WIDTH));
            }

            horizontal_strip.horizontal(|mut strip| {
                // Left panel: Series selection
                if !self.headers.is_empty() {
                    strip.cell(|ui| {
                        self.render_series_panel(ctx, ui);
                    });
                }

                // Center: Toolbar/controls and plot
                strip.cell(|ui| {
                    // Build vertical strip for toolbar, plot, and stats
                    let mut vertical_strip = StripBuilder::new(ui);

                    // Toolbar area (auto-sized to content)
                    vertical_strip = vertical_strip.size(Size::initial(120.0));

                    // Plot area (remainder of space)
                    vertical_strip = vertical_strip.size(Size::remainder());

                    // Stats panel (conditional)
                    if self.state.view.show_stats_panel && !self.state.view.y_indices.is_empty() {
                        vertical_strip = vertical_strip.size(Size::exact(constants::layout::STATS_PANEL_HEIGHT));
                    }

                    vertical_strip.vertical(|mut strip| {
                        // Toolbar and controls
                        strip.cell(|ui| {
                            let has_data = self.render_toolbar_and_controls(ctx, ui);

                            // If no data to plot, show message
                            if !has_data {
                                ui.vertical_centered(|ui| {
                                    ui.heading("No data loaded");
                                    ui.label("Click 'Open CSV File' or drag and drop a CSV file to get started");
                                });
                            }
                        });

                        // Plot area
                        strip.cell(|ui| {
                            // Only render plot if we have data and Y series selected
                            if !self.headers.is_empty() && !self.data.is_empty() && !self.state.view.y_indices.is_empty() {
                                self.render_plot(ctx, ui);
                            }
                        });

                        // Stats panel (conditional)
                        if self.state.view.show_stats_panel && !self.state.view.y_indices.is_empty() {
                            strip.cell(|ui| {
                                self.render_stats_panel(ui);
                            });
                        }
                    });
                });

                // Right panel: Data table
                if self.state.view.show_data_table && !self.raw_data.is_empty() {
                    strip.cell(|ui| {
                        self.render_data_table_panel(ui);
                    });
                }
            });

            // Status bar at bottom
            ui.add_space(ui.available_height() - 20.0);
            ui.separator();
            ui.horizontal(|ui| {
                if let Some(ref file) = self.state.current_file {
                    if let Some(name) = file.file_name() {
                        ui.label(format!("📁 {}", name.to_string_lossy()));
                        ui.separator();
                    }
                }
                ui.label(format!("Rows: {} | Cols: {}", self.data.len(), self.headers.len()));
                if !self.state.view.y_indices.is_empty() {
                    ui.separator();
                    ui.label(format!("Series: {}", self.state.view.y_indices.len()));
                }
                if let Some((_series_idx, point_idx)) = self.state.view.selected_point {
                    ui.separator();
                    ui.label(format!("Selected: Row {}", point_idx + 1));
                }
            });
        });

        // Help dialog
        self.render_help_dialog(ctx);
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "PlotOxide - Advanced Data Plotter",
        options,
        Box::new(|_| Ok(Box::new(PlotOxide::default()))),
    )
    .unwrap();
}
