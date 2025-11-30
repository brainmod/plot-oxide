//! Range input widget for min/max value filtering

#![allow(dead_code)]

use egui::{Response, Ui};

/// A reusable widget for inputting an optional range (min/max)
pub struct RangeInput<'a> {
    label: &'a str,
    range: &'a mut Option<(f64, f64)>,
    default_min: f64,
    default_max: f64,
    speed: f64,
}

impl<'a> RangeInput<'a> {
    /// Create a new range input widget
    pub fn new(label: &'a str, range: &'a mut Option<(f64, f64)>) -> Self {
        Self {
            label,
            range,
            default_min: 0.0,
            default_max: 100.0,
            speed: 0.1,
        }
    }

    /// Set default values for when the range is enabled
    pub fn defaults(mut self, min: f64, max: f64) -> Self {
        self.default_min = min;
        self.default_max = max;
        self
    }

    /// Set the drag speed for value inputs
    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    /// Show the widget
    pub fn show(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            ui.label(self.label);

            let mut min_enabled = self.range.is_some();
            ui.checkbox(&mut min_enabled, "Min");

            let (mut min_val, mut max_val) = self.range.unwrap_or((self.default_min, self.default_max));

            if min_enabled {
                ui.add(egui::DragValue::new(&mut min_val).speed(self.speed));
            }

            let mut max_enabled = self.range.is_some();
            ui.checkbox(&mut max_enabled, "Max");

            if max_enabled {
                ui.add(egui::DragValue::new(&mut max_val).speed(self.speed));
            }

            // Update the range based on checkbox states
            *self.range = if min_enabled || max_enabled {
                Some((min_val, max_val))
            } else {
                None
            };
        })
        .response
    }
}
