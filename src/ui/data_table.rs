use crate::app::PlotOxide;
use egui_extras::{Column, TableBuilder};

/// Render the data table panel (right sidebar)
pub fn render_data_table_panel(app: &mut PlotOxide, ui: &mut eframe::egui::Ui) {
    ui.heading("Data Table");

    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(&mut app.state.ui.row_filter);
        if ui.button("✖").clicked() {
            app.state.ui.row_filter.clear();
        }
    });

    ui.separator();

    // Build list of displayed columns (X + Y series)
    let mut display_cols = vec![app.state.view.x_index];
    display_cols.extend(&app.state.view.y_indices);
    display_cols.sort_unstable();
    display_cols.dedup();

    let mut table_scroll = eframe::egui::ScrollArea::vertical().id_salt("data_table_scroll");

    if let Some(row_to_scroll) = app.state.ui.scroll_to_row.take() {
        table_scroll = table_scroll.vertical_scroll_offset((row_to_scroll as f32) * 18.0);
    }

    table_scroll.show(ui, |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(eframe::egui::Layout::left_to_right(eframe::egui::Align::Center))
            .column(Column::initial(40.0).resizable(false)) // Row number
            .columns(Column::initial(80.0).resizable(true), display_cols.len())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("#");
                });
                for &col_idx in &display_cols {
                    header.col(|ui| {
                        let label = &app.headers[col_idx];
                        let sort_indicator = if app.state.ui.sort_column == Some(col_idx) {
                            if app.state.ui.sort_ascending { " ↑" } else { " ↓" }
                        } else {
                            ""
                        };
                        if ui.button(format!("{}{}", label, sort_indicator)).clicked() {
                            if app.state.ui.sort_column == Some(col_idx) {
                                app.state.ui.sort_ascending = !app.state.ui.sort_ascending;
                            } else {
                                app.state.ui.sort_column = Some(col_idx);
                                app.state.ui.sort_ascending = true;
                            }
                        }
                    });
                }
            })
            .body(|mut body| {
                // Calculate row indices (filtering and sorting)
                let mut row_indices: Vec<usize> = (0..app.raw_data.len()).collect();

                // Apply filter
                if !app.state.ui.row_filter.is_empty() {
                    let filter_lower = app.state.ui.row_filter.to_lowercase();
                    row_indices.retain(|&idx| {
                        app.raw_data[idx].iter().any(|cell| cell.to_lowercase().contains(&filter_lower))
                    });
                }

                // Apply sort
                if let Some(sort_col) = app.state.ui.sort_column {
                    row_indices.sort_by(|&a, &b| {
                        let val_a = &app.data[a][sort_col];
                        let val_b = &app.data[b][sort_col];
                        if app.state.ui.sort_ascending {
                            val_a.partial_cmp(val_b).unwrap_or(std::cmp::Ordering::Equal)
                        } else {
                            val_b.partial_cmp(val_a).unwrap_or(std::cmp::Ordering::Equal)
                        }
                    });
                }

                for &row_idx in &row_indices {
                    let row_data = &app.raw_data[row_idx];
                    let is_hovered = app.state.view.hovered_point.map(|(_, pi)| pi == row_idx).unwrap_or(false);
                    let is_selected = app.state.view.selected_point.map(|(_, pi)| pi == row_idx).unwrap_or(false);
                    let is_excursion = app.state.spc.excursion_rows.contains(&row_idx);

                    body.row(18.0, |mut row_ui| {
                        if is_selected || is_hovered {
                            row_ui.set_selected(true);
                        }

                        row_ui.col(|ui| {
                            if is_excursion {
                                ui.colored_label(eframe::egui::Color32::RED, format!("⚠ {}", row_idx + 1));
                            } else {
                                ui.label(format!("{}", row_idx + 1));
                            }
                        });

                        for &col_idx in &display_cols {
                            let cell = &row_data[col_idx];
                            row_ui.col(|ui| {
                                // Highlight X column or Y series
                                if col_idx == app.state.view.x_index {
                                    ui.strong(cell);
                                } else if app.state.view.y_indices.contains(&col_idx) {
                                    ui.strong(cell);
                                } else {
                                    ui.label(cell);
                                }
                            });
                        }

                        // Detect if this row is hovered (after all columns)
                        if row_ui.response().hovered() {
                            app.state.view.table_hovered_row = Some(row_idx);
                        }
                    });
                }
            });
    });
}
