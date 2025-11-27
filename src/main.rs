#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::App;
use eframe::egui::{self, CentralPanel};
use egui_extras::{Size, StripBuilder};

// Module declarations
mod app;
mod constants;
mod data;
mod error;
mod state;
mod widgets;
mod ui;

// Use PlotOxide from app module
use app::PlotOxide;

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

            // Add series panel if we have data
            if self.state.has_data() {
                horizontal_strip = horizontal_strip.size(Size::exact(constants::layout::SERIES_PANEL_WIDTH));
            }

            // Add center (plot area) - takes remaining space
            horizontal_strip = horizontal_strip.size(Size::remainder());

            // Add data table panel if enabled
            if self.state.view.show_data_table && self.state.has_data() {
                horizontal_strip = horizontal_strip.size(Size::exact(constants::layout::DATA_PANEL_WIDTH));
            }

            horizontal_strip.horizontal(|mut strip| {
                // Left panel: Series selection
                if self.state.has_data() {
                    strip.cell(|ui| {
                        ui::render_series_panel(self, ctx, ui);
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
                            let has_data = ui::render_toolbar_and_controls(self, ctx, ui);

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
                            if self.state.has_data() && !self.state.view.y_indices.is_empty() {
                                ui::render_plot(self, ctx, ui);
                            }
                        });

                        // Stats panel (conditional)
                        if self.state.view.show_stats_panel && !self.state.view.y_indices.is_empty() {
                            strip.cell(|ui| {
                                ui::render_stats_panel(self, ui);
                            });
                        }
                    });
                });

                // Right panel: Data table
                if self.state.view.show_data_table && self.state.has_data() {
                    strip.cell(|ui| {
                        ui::render_data_table_panel(self, ui);
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
                ui.label(format!("Rows: {} | Cols: {}", self.state.row_count(), self.state.column_count()));
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
        ui::render_help_dialog(self, ctx);
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
