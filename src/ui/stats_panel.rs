use crate::app::PlotOxide;

/// Render the statistics summary panel (bottom panel)
pub fn render_stats_panel(app: &mut PlotOxide, ui: &mut eframe::egui::Ui) {
    ui.heading("Statistics Summary");
    ui.separator();

    eframe::egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.horizontal(|ui| {
            // Get data from DataSource
            let data = app.data();
            let headers = app.headers();

            // Calculate all series data
            let all_series: Vec<Vec<[f64; 2]>> = app.state.view.y_indices.iter()
                .map(|&y_idx| {
                    data.iter()
                        .map(|row| [row[app.state.view.x_index], row[y_idx]])
                        .collect()
                })
                .collect();

            for (series_idx, &y_idx) in app.state.view.y_indices.iter().enumerate() {
                let color = PlotOxide::get_series_color(series_idx);
                let name = &headers[y_idx];
                let y_values: Vec<f64> = all_series[series_idx].iter().map(|p| p[1]).collect();

                if !y_values.is_empty() {
                    let (mean, std_dev) = PlotOxide::calculate_statistics(&y_values);
                    let median = PlotOxide::calculate_median(&y_values);
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
                        if app.state.spc.show_capability {
                            let (cp, cpk) = PlotOxide::calculate_process_capability(&y_values, app.state.spc.spec_lower, app.state.spc.spec_upper);
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
