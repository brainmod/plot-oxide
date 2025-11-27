use crate::app::PlotOxide;

/// Render the Y series selection panel (left sidebar)
pub fn render_series_panel(app: &mut PlotOxide, ctx: &eframe::egui::Context, ui: &mut eframe::egui::Ui) {
    // Get data from DataSource
    let headers = app.headers();
    let data = app.data();

    let ctrl_held = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);
    let shift_held = ctx.input(|i| i.modifiers.shift);

    ui.heading("Y Series");
    ui.separator();

    let old_y_indices = app.state.view.y_indices.clone();

    eframe::egui::ScrollArea::vertical().show(ui, |ui| {
        for (i, header) in headers.iter().enumerate() {
            let is_selected = app.state.view.y_indices.contains(&i);

            // Check for sigma violations for ALL columns (independent of selection)
            let violation_color = if !data.is_empty() {
                let y_values: Vec<f64> = data.iter().map(|row| row[i]).collect();
                let (mean, std_dev) = PlotOxide::calculate_statistics(&y_values);
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
                    Some(eframe::egui::Color32::from_rgb(255, 50, 50))
                } else if has_2sigma {
                    Some(eframe::egui::Color32::from_rgb(255, 165, 0))
                } else {
                    None
                }
            } else {
                None
            };

            let color = if is_selected {
                PlotOxide::get_series_color(app.state.view.y_indices.iter().position(|&x| x == i).unwrap_or(0))
            } else {
                eframe::egui::Color32::GRAY
            };

            ui.horizontal(|ui| {
                let response = ui.selectable_label(is_selected, header);
                if is_selected {
                    ui.painter().circle_filled(
                        response.rect.left_center() - eframe::egui::vec2(10.0, 0.0),
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
                        if let Some(last) = app.state.view.last_selected_series {
                            let start = last.min(i);
                            let end = last.max(i);
                            if ctrl_held {
                                // Add range to existing selection
                                for idx in start..=end {
                                    if !app.state.view.y_indices.contains(&idx) {
                                        app.state.view.y_indices.push(idx);
                                    }
                                }
                            } else {
                                // Replace with range
                                app.state.view.y_indices = (start..=end).collect();
                            }
                        } else {
                            app.state.view.y_indices = vec![i];
                        }
                        app.state.view.last_selected_series = Some(i);
                    } else if ctrl_held {
                        // Toggle individual item
                        if is_selected {
                            app.state.view.y_indices.retain(|&x| x != i);
                        } else {
                            app.state.view.y_indices.push(i);
                        }
                        app.state.view.last_selected_series = Some(i);
                    } else {
                        // Single-select mode (replace)
                        app.state.view.y_indices = vec![i];
                        app.state.view.last_selected_series = Some(i);
                    }
                }
            });
        }
    });

    // Clear point selection if series changed
    if app.state.view.y_indices != old_y_indices {
        app.state.view.selected_point = None;
        app.state.view.reset_bounds = true;
    }
}
