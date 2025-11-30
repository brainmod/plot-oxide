//! SPC (Statistical Process Control) controls widget

use crate::state::SpcConfig;
use egui::{Response, Ui};

/// A reusable widget for SPC controls
pub struct SpcControls<'a> {
    config: &'a mut SpcConfig,
}

impl<'a> SpcControls<'a> {
    /// Create a new SPC controls widget
    pub fn new(config: &'a mut SpcConfig) -> Self {
        Self { config }
    }

    /// Show the SPC controls
    pub fn show(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            ui.label("SPC:");

            // Control Limits
            ui.checkbox(&mut self.config.show_spc_limits, "Control Limits");
            if self.config.show_spc_limits {
                ui.label("σ:");
                ui.add(egui::Slider::new(&mut self.config.sigma_multiplier, 1.0..=6.0).step_by(0.5));
            }

            // Sigma Zones
            ui.checkbox(&mut self.config.show_sigma_zones, "Zones");

            // Western Electric Rules
            ui.checkbox(&mut self.config.show_we_rules, "WE Rules");

            // ui.separator();

            // Process Capability (Cp/Cpk)
            ui.checkbox(&mut self.config.show_capability, "Cp/Cpk");
            if self.config.show_capability {
                ui.label("LSL:");
                ui.add(egui::DragValue::new(&mut self.config.spec_lower).speed(0.1));
                ui.label("USL:");
                ui.add(egui::DragValue::new(&mut self.config.spec_upper).speed(0.1));
            }

            // ui.separator();

            // Outliers
            ui.checkbox(&mut self.config.show_outliers, "Outliers");
            if self.config.show_outliers {
                ui.label("Z:");
                ui.add(egui::Slider::new(&mut self.config.outlier_threshold, 2.0..=6.0).step_by(0.5));
            }

            // ui.separator();

            // Moving Average
            ui.checkbox(&mut self.config.show_moving_avg, "MA");
            if self.config.show_moving_avg {
                ui.label("Win:");
                ui.add(egui::Slider::new(&mut self.config.ma_window, 3..=50));
            }

            // EWMA (Exponentially Weighted Moving Average)
            ui.checkbox(&mut self.config.show_ewma, "EWMA");
            if self.config.show_ewma {
                ui.label("λ:");
                ui.add(egui::Slider::new(&mut self.config.ewma_lambda, 0.05..=0.5).step_by(0.05));
            }

            // ui.separator();

            // Regression
            ui.checkbox(&mut self.config.show_regression, "Regression");
            if self.config.show_regression {
                ui.label("Order:");
                ui.add(egui::Slider::new(&mut self.config.regression_order, 1..=4));
            }
        })
        .response
    }
}
