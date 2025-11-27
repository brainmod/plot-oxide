//! Data filtering controls widget

use crate::state::FilterConfig;
use egui::{Response, Ui};

/// A reusable widget for data filtering controls
pub struct FilterControls<'a> {
    config: &'a mut FilterConfig,
}

impl<'a> FilterControls<'a> {
    /// Create a new filter controls widget
    pub fn new(config: &'a mut FilterConfig) -> Self {
        Self { config }
    }

    /// Show the filter controls
    pub fn show(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            ui.label("Filters:");

            // Empty values filter (Y series only)
            ui.checkbox(&mut self.config.filter_empty, "Empty (Y series only)");

            ui.separator();

            // Y Range filter
            ui.label("Y Range:");
            let mut y_min_enabled = self.config.filter_y_min.is_some();
            ui.checkbox(&mut y_min_enabled, "Min");
            if y_min_enabled {
                let mut val = self.config.filter_y_min.unwrap_or(0.0);
                ui.add(egui::DragValue::new(&mut val).speed(0.1));
                self.config.filter_y_min = Some(val);
            } else {
                self.config.filter_y_min = None;
            }

            let mut y_max_enabled = self.config.filter_y_max.is_some();
            ui.checkbox(&mut y_max_enabled, "Max");
            if y_max_enabled {
                let mut val = self.config.filter_y_max.unwrap_or(100.0);
                ui.add(egui::DragValue::new(&mut val).speed(0.1));
                self.config.filter_y_max = Some(val);
            } else {
                self.config.filter_y_max = None;
            }

            ui.separator();

            // X Range filter
            ui.label("X Range:");
            let mut x_min_enabled = self.config.filter_x_min.is_some();
            ui.checkbox(&mut x_min_enabled, "Min");
            if x_min_enabled {
                let mut val = self.config.filter_x_min.unwrap_or(0.0);
                ui.add(egui::DragValue::new(&mut val).speed(0.1));
                self.config.filter_x_min = Some(val);
            } else {
                self.config.filter_x_min = None;
            }

            let mut x_max_enabled = self.config.filter_x_max.is_some();
            ui.checkbox(&mut x_max_enabled, "Max");
            if x_max_enabled {
                let mut val = self.config.filter_x_max.unwrap_or(100.0);
                ui.add(egui::DragValue::new(&mut val).speed(0.1));
                self.config.filter_x_max = Some(val);
            } else {
                self.config.filter_x_max = None;
            }

            ui.separator();

            // Outlier filter
            ui.checkbox(&mut self.config.filter_outliers, "Filter Outliers");
            if self.config.filter_outliers {
                ui.label("Z:");
                ui.add(egui::Slider::new(&mut self.config.filter_outlier_sigma, 2.0..=6.0).step_by(0.5));
            }
        })
        .response
    }
}
