use crate::app::PlotOxide;
use egui_extras::{Column, TableBuilder};

/// Render the data table panel with virtual scrolling (Phase 3.1)
pub fn render_data_table_panel(app: &mut PlotOxide, ui: &mut eframe::egui::Ui) {
    // puffin::profile_function!(); // Disabled: puffin version incompatibility
    
    let ds = match &app.state.data {
        Some(ds) => ds,
        None => return,
    };
    
    let headers = app.headers();
    let total_rows = ds.height();

    // ui.heading("Data Table");

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

    let row_height = 18.0;
    
    // Pre-compute filtered/sorted row indices only when filter changes
    // For now, compute inline but avoid materializing all data
    let filter_active = !app.state.ui.row_filter.is_empty();
    let filter_lower = app.state.ui.row_filter.to_lowercase();
    
    TableBuilder::new(ui)
        .striped(true)
        .cell_layout(eframe::egui::Layout::left_to_right(eframe::egui::Align::Center))
        .column(Column::initial(40.0).resizable(false)) // Row number
        .columns(Column::initial(80.0).resizable(true), display_cols.len())
        .sense(eframe::egui::Sense::click())
        .header(20.0, |mut header| {
            header.col(|ui| { ui.strong("#"); });
            for &col_idx in &display_cols {
                header.col(|ui| {
                    let label = &headers[col_idx];
                    let sort_indicator = if app.state.ui.sort_column == Some(col_idx) {
                        if app.state.ui.sort_ascending { " ↑" } else { " ↓" }
                    } else { "" };
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
        .body(|body| {
            // Virtual scrolling: only render visible rows
            body.rows(row_height, total_rows, |mut row| {
                let row_idx = row.index();
                
                // Skip filtered rows (simple implementation - could be optimized with index cache)
                if filter_active {
                    let mut matches = false;
                    for &col_idx in &display_cols {
                        if let Some(val) = ds.get_string(row_idx, col_idx) {
                            if val.to_lowercase().contains(&filter_lower) {
                                matches = true;
                                break;
                            }
                        }
                    }
                    if !matches {
                        // Render empty row for filtered items (maintains scroll position)
                        row.col(|_| {});
                        for _ in &display_cols { row.col(|_| {}); }
                        return;
                    }
                }
                
                let is_hovered = app.state.view.hovered_point.map(|(_, pi)| pi == row_idx).unwrap_or(false);
                let is_selected = app.state.view.selected_point.map(|(_, pi)| pi == row_idx).unwrap_or(false);
                let is_excursion = app.state.spc.excursion_rows.contains(&row_idx);

                if is_selected || is_hovered {
                    row.set_selected(true);
                }

                row.col(|ui| {
                    if is_excursion {
                        ui.colored_label(eframe::egui::Color32::RED, format!("⚠ {}", row_idx + 1));
                    } else {
                        ui.label(format!("{}", row_idx + 1));
                    }
                });

                for &col_idx in &display_cols {
                    row.col(|ui| {
                        // Direct cell access - no full row materialization
                        let cell = ds.get_string(row_idx, col_idx).unwrap_or_default();
                        if col_idx == app.state.view.x_index || app.state.view.y_indices.contains(&col_idx) {
                            ui.strong(&cell);
                        } else {
                            ui.label(&cell);
                        }
                    });
                }
            });
        });
}
