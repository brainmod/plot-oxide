use crate::app::PlotOxide;
use crate::state::{PlotMode, LineStyle};
use crate::widgets::{SpcControls, FilterControls};
use eframe::egui::ComboBox;

/// Render the toolbar and control panels
/// Returns false if no Y series selected (skip plot rendering), true otherwise
pub fn render_toolbar_and_controls(app: &mut PlotOxide, ctx: &eframe::egui::Context, ui: &mut eframe::egui::Ui) -> bool {
    // Compact toolbar with icon buttons
    ui.horizontal(|ui| {
        // File operations
        if ui.button("📂").on_hover_text("Open Data File").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Data Files", &["csv", "parquet"])
                .pick_file()
            {
                if let Err(e) = app.load_file(path) {
                    app.state.ui.set_error(e.user_message());
                }
            }
        }

        // Recent files menu
        if !app.state.recent_files.is_empty() {
            eframe::egui::ComboBox::from_label("")
                .selected_text("📋")
                .show_ui(ui, |ui| {
                    ui.label("Recent Files:");
                    ui.separator();
                    // Need to clone to avoid borrow checker issues with load_csv
                    for path in app.state.recent_files.clone().iter() {
                        if let Some(name) = path.file_name() {
                            if ui.button(name.to_string_lossy()).clicked() {
                                let path_clone = path.clone();
                                if let Err(e) = app.load_file(path_clone) {
                                    app.state.ui.set_error(e.user_message());
                                }
                            }
                        }
                    }
                });
        }
    });

    // ui.separator();

    // Display current file with icon
    app.state.current_file
        .as_ref()
        .map(|file| {
        ui.label(
            format!("📄 {}", 
                file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
            )
        )
        .on_hover_text(file.display().to_string())
        });
    

    ui.separator();

    // Handle drag and drop using Option combinators
    ctx.input(|i| {
        i.raw.dropped_files
            .first()
            .and_then(|f| f.path.as_ref())
            .map(|path| {
                if let Err(e) = app.load_file(path.clone()) {
                    app.state.ui.set_error(e.user_message());
                }
            });
    });

    // Show plot only if we have data
    if app.state.has_data() {
        let headers = app.headers();
        // Axis selection and controls
        ui.vertical(|ui| {
            let old_x = app.state.view.x_index;
            let old_use_row = app.state.view.use_row_index;

            ui.checkbox(&mut app.state.view.use_row_index, "Row Index");

            if !app.state.view.use_row_index {
                ComboBox::from_label("X Axis")
                    .selected_text(&headers[app.state.view.x_index])
                    .show_ui(ui, |ui| {
                        for (i, h) in headers.iter().enumerate() {
                            ui.selectable_value(&mut app.state.view.x_index, i, h);
                        }
                    });
            } else {
                ui.label("X Axis: Row #");
            }

            // Update timestamp flag if X axis changed
            if old_x != app.state.view.x_index || old_use_row != app.state.view.use_row_index {
                if !app.state.view.use_row_index {
                    app.state.view.x_is_timestamp = app.is_column_timestamp(app.state.view.x_index);
                } else {
                    app.state.view.x_is_timestamp = false;
                }
                app.state.view.reset_bounds = true;
            }

            // ui.separator();

            // if ui.button("🔄").on_hover_text("Reset the view to default bounds").clicked() {
            //     app.reset_view();
            // }
        });

        ui.separator();
        
        // Compact display controls with icon buttons
        // ui.horizontal(|ui| {
        //     // ui.label("Display:");
        //     // ui.toggle_value(&mut app.state.view.show_grid, "⊞").on_hover_text("Grid (G)");
        //     // ui.toggle_value(&mut app.state.view.show_legend, "🏷").on_hover_text("Legend (L)");
        //     // ui.toggle_value(&mut app.state.view.allow_zoom, "🔍").on_hover_text("Zoom");
        //     // ui.toggle_value(&mut app.state.view.allow_drag, "✋").on_hover_text("Pan");
        //     // ui.toggle_value(&mut app.state.view.show_data_table, "📋").on_hover_text("Data Table");
        //     // ui.toggle_value(&mut app.state.view.show_stats_panel, "∑").on_hover_text("Statistics");

        //     // ui.separator();

        //     // if ui.button("💾").on_hover_text("Export CSV").clicked() {
        //     //     app.export_csv();
        //     // }

        //     // if ui.button("⚙").on_hover_text("Save Config").clicked() {
        //     //     app.save_config();
        //     // }
        //     // if ui.button("📥").on_hover_text("Load Config").clicked() {
        //     //     app.load_config();
        //     // }

        //     // ui.separator();
        //     // if ui.button(if app.state.view.dark_mode { "🌙" } else { "☀" }).on_hover_text("Toggle theme").clicked() {
        //     //     app.state.view.dark_mode = !app.state.view.dark_mode;
        //     // }

        //     // ui.separator();
        //     // if ui.button("❓").on_hover_text("Help (F1)").clicked() {
        //     //     app.state.view.show_help = !app.state.view.show_help;
        //     // }
        // });

        // Plot mode and style controls (collapsible)
        eframe::egui::CollapsingHeader::new("📈 Plot Mode")
            .id_salt("plot_mode")
            .default_open(false)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label("Mode:");
                    ui.radio_value(&mut app.state.view.plot_mode, PlotMode::Scatter, "Scatter/Line");
                    ui.radio_value(&mut app.state.view.plot_mode, PlotMode::Histogram, "Histogram");
                    ui.radio_value(&mut app.state.view.plot_mode, PlotMode::BoxPlot, "Box Plot");
                    ui.radio_value(&mut app.state.view.plot_mode, PlotMode::Pareto, "Pareto");
                    ui.radio_value(&mut app.state.view.plot_mode, PlotMode::XbarR, "X-bar & R");
                    ui.radio_value(&mut app.state.view.plot_mode, PlotMode::PChart, "p-chart");
                });

                // Mode-specific controls
                match app.state.view.plot_mode {
                    PlotMode::Histogram => {
                        ui.vertical(|ui| {
                            ui.label("Bins:");
                            ui.add(eframe::egui::Slider::new(&mut app.state.view.histogram_bins, 5..=50));
                        });
                    }
                    PlotMode::XbarR => {
                        ui.vertical(|ui| {
                            ui.label("Subgroup:");
                            ui.add(eframe::egui::Slider::new(&mut app.state.spc.xbarr_subgroup_size, 2..=10));
                        });
                    }
                    PlotMode::PChart => {
                        ui.vertical(|ui| {
                            ui.label("Sample n:");
                            ui.add(eframe::egui::Slider::new(&mut app.state.spc.pchart_sample_size, 10..=200));
                        });
                    }
                    PlotMode::Scatter => {
                        ui.vertical(|ui| {
                            ui.label("Style:");
                            ui.radio_value(&mut app.state.view.line_style, LineStyle::Line, "Line");
                            ui.radio_value(&mut app.state.view.line_style, LineStyle::Points, "Points");
                            ui.radio_value(&mut app.state.view.line_style, LineStyle::LineAndPoints, "Both");
                        });
                    }
                    _ => {}
                }
            });

        // Only show SPC/Analysis controls in Scatter mode
        if app.state.view.plot_mode == PlotMode::Scatter {
            // SPC Controls (collapsible)
            eframe::egui::CollapsingHeader::new("📊 Overlays")
                .id_salt("spc_controls")
                .default_open(false)
                .show(ui, |ui| {
                    SpcControls::new(&mut app.state.spc).show(ui);
                });

            // Data Filtering Controls (collapsible)
            eframe::egui::CollapsingHeader::new("🔍 Filters")
                .id_salt("filter_controls")
                .default_open(false)
                .show(ui, |ui| {
                    FilterControls::new(&mut app.state.filters).show(ui);
                });
        }

        // ui.separator();

        // Check if no Y series selected
        if app.state.view.y_indices.is_empty() {
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
