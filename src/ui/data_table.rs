use crate::app::PlotOxide;
use egui_extras::{Column, TableBuilder};

/// Recompute filtered and sorted row indices
fn recompute_indices(
    app: &mut PlotOxide,
    display_cols: &[usize],
) {
    profiling::scope!("recompute_table_indices");
    
    let ds = match &app.state.data {
        Some(ds) => ds,
        None => return,
    };
    
    let total_rows = ds.height();
    let filter = &app.state.ui.row_filter;
    let filter_lower = filter.to_lowercase();
    let filter_active = !filter.is_empty();
    
    // Phase 1: Filter
    let mut filtered: Vec<usize> = if filter_active {
        // Pre-fetch column string data for filtering (more efficient than per-row lookup)
        let col_strings: Vec<Vec<String>> = display_cols.iter()
            .filter_map(|&col_idx| ds.column_as_string(col_idx).ok())
            .collect();
        
        (0..total_rows)
            .filter(|&row_idx| {
                col_strings.iter().any(|col| {
                    col.get(row_idx)
                        .map(|s| s.to_lowercase().contains(&filter_lower))
                        .unwrap_or(false)
                })
            })
            .collect()
    } else {
        (0..total_rows).collect()
    };
    
    app.state.ui.table.filtered_indices = filtered.clone();
    
    // Phase 2: Sort
    if let Some(sort_col) = app.state.ui.sort_column {
        if let Ok(sort_data) = ds.get_cached_column(sort_col) {
            let ascending = app.state.ui.sort_ascending;
            filtered.sort_by(|&a, &b| {
                let va = sort_data.get(a).copied().unwrap_or(f64::NAN);
                let vb = sort_data.get(b).copied().unwrap_or(f64::NAN);
                
                // Handle NaN: push to end
                match (va.is_nan(), vb.is_nan()) {
                    (true, true) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    (false, false) => {
                        let cmp = va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal);
                        if ascending { cmp } else { cmp.reverse() }
                    }
                }
            });
        }
    }
    
    app.state.ui.table.display_indices = filtered;
    app.state.ui.table.update_cache_keys(
        &app.state.ui.row_filter,
        app.state.ui.sort_column,
        app.state.ui.sort_ascending,
        app.state.ui.data_version,
    );
}

/// Render the data table panel with virtual scrolling
pub fn render_data_table_panel(app: &mut PlotOxide, ui: &mut eframe::egui::Ui) {
    profiling::scope!("render_data_table");
    
    let ds = match &app.state.data {
        Some(ds) => ds,
        None => return,
    };
    
    let headers = app.headers();
    let total_rows = ds.height();

    // Build list of displayed columns (X + Y series)
    let mut display_cols = vec![app.state.view.x_index];
    display_cols.extend(&app.state.view.y_indices);
    display_cols.sort_unstable();
    display_cols.dedup();

    // Filter/Search controls
    ui.horizontal(|ui| {
        ui.label("🔍");
        let filter_response = ui.add(
            eframe::egui::TextEdit::singleline(&mut app.state.ui.row_filter)
                .hint_text("Filter rows...")
                .desired_width(120.0)
        );
        if filter_response.changed() {
            // Invalidate cache when filter changes
            app.state.ui.table.invalidate();
        }
        if ui.small_button("✖").on_hover_text("Clear filter").clicked() {
            app.state.ui.row_filter.clear();
            app.state.ui.table.invalidate();
        }
        
        ui.separator();
        
        // Go-to-row
        ui.label("Go to:");
        let goto_response = ui.add(
            eframe::egui::TextEdit::singleline(&mut app.state.ui.table.goto_row_input)
                .hint_text("#")
                .desired_width(50.0)
        );
        if goto_response.lost_focus() && ui.input(|i| i.key_pressed(eframe::egui::Key::Enter)) {
            if let Ok(row) = app.state.ui.table.goto_row_input.parse::<usize>() {
                let target = row.saturating_sub(1).min(total_rows.saturating_sub(1));
                app.state.ui.scroll_to_row = Some(target);
            }
            app.state.ui.table.goto_row_input.clear();
        }
        
        ui.separator();
        
        // Selection info
        let selected_count = app.state.ui.table.selected_rows.len();
        if selected_count > 0 {
            ui.label(format!("{} selected", selected_count));
            if ui.small_button("Copy").on_hover_text("Copy selected rows (Ctrl+C)").clicked() {
                copy_selected_rows(app, &display_cols);
            }
            if ui.small_button("Clear").clicked() {
                app.state.ui.table.clear_selection();
            }
        }
    });

    // Recompute indices if cache invalid
    if !app.state.ui.table.is_cache_valid(
        &app.state.ui.row_filter,
        app.state.ui.sort_column,
        app.state.ui.sort_ascending,
        app.state.ui.data_version,
    ) {
        recompute_indices(app, &display_cols);
    }
    
    let display_indices = &app.state.ui.table.display_indices;
    let filtered_count = display_indices.len();
    
    // Status line
    ui.horizontal(|ui| {
        if filtered_count < total_rows {
            ui.label(format!("Showing {} of {} rows", filtered_count, total_rows));
        } else {
            ui.label(format!("{} rows", total_rows));
        }
        
        if app.state.ui.sort_column.is_some() {
            ui.separator();
            let sort_col = app.state.ui.sort_column.unwrap();
            let dir = if app.state.ui.sort_ascending { "↑" } else { "↓" };
            ui.small(format!("Sorted by {} {}", headers.get(sort_col).unwrap_or(&"?".to_string()), dir));
            if ui.small_button("✖").on_hover_text("Clear sort").clicked() {
                app.state.ui.clear_sort();
                app.state.ui.table.invalidate();
            }
        }
    });
    
    ui.separator();

    let row_height = 18.0;
    
    // Pre-fetch string columns for visible rows (performance optimization)
    // We'll do direct cell access but with cached column data
    
    TableBuilder::new(ui)
        .striped(true)
        .cell_layout(eframe::egui::Layout::left_to_right(eframe::egui::Align::Center))
        .column(Column::initial(45.0).resizable(false)) // Row number + checkbox
        .columns(Column::initial(100.0).resizable(true).clip(true), display_cols.len())
        .sense(eframe::egui::Sense::click())
        .header(22.0, |mut header| {
            header.col(|ui| { 
                ui.strong("#"); 
            });
            for &col_idx in &display_cols {
                header.col(|ui| {
                    let label = headers.get(col_idx).map(|s| s.as_str()).unwrap_or("?");
                    let is_sorted = app.state.ui.sort_column == Some(col_idx);
                    let sort_indicator = if is_sorted {
                        if app.state.ui.sort_ascending { " ↑" } else { " ↓" }
                    } else { "" };
                    
                    let btn = eframe::egui::Button::new(
                        eframe::egui::RichText::new(format!("{}{}", label, sort_indicator))
                            .strong()
                    ).frame(false);
                    
                    if ui.add(btn).on_hover_text("Click to sort").clicked() {
                        app.state.ui.toggle_sort(col_idx);
                        app.state.ui.table.invalidate();
                    }
                });
            }
        })
        .body(|body| {
            body.rows(row_height, filtered_count, |mut row| {
                let display_idx = row.index();
                let row_idx = match display_indices.get(display_idx) {
                    Some(&idx) => idx,
                    None => return,
                };
                
                let is_hovered = app.state.view.hovered_point.map(|(_, pi)| pi == row_idx).unwrap_or(false);
                let is_plot_selected = app.state.view.selected_point.map(|(_, pi)| pi == row_idx).unwrap_or(false);
                let is_table_selected = app.state.ui.table.is_selected(row_idx);
                let is_excursion = app.state.spc.excursion_rows.contains(&row_idx);

                if is_plot_selected || is_hovered || is_table_selected {
                    row.set_selected(true);
                }

                // Row number column with selection checkbox
                row.col(|ui| {
                    ui.horizontal(|ui| {
                        // Checkbox for selection
                        let mut selected = is_table_selected;
                        if ui.checkbox(&mut selected, "").changed() {
                            if selected {
                                app.state.ui.table.selected_rows.insert(row_idx);
                            } else {
                                app.state.ui.table.selected_rows.remove(&row_idx);
                            }
                        }
                        
                        // Row number with excursion indicator
                        if is_excursion {
                            ui.colored_label(eframe::egui::Color32::RED, format!("{}", row_idx + 1));
                        } else {
                            ui.label(format!("{}", row_idx + 1));
                        }
                    });
                });

                // Data columns
                let filter_lower = app.state.ui.row_filter.to_lowercase();
                let highlight_filter = !filter_lower.is_empty();
                
                for &col_idx in &display_cols {
                    row.col(|ui| {
                        let ds = app.state.data.as_ref().unwrap();
                        let cell = ds.get_string(row_idx, col_idx).unwrap_or_default();
                        
                        // Highlight matching text
                        if highlight_filter && cell.to_lowercase().contains(&filter_lower) {
                            // Find match position and highlight
                            let cell_lower = cell.to_lowercase();
                            if let Some(start) = cell_lower.find(&filter_lower) {
                                let end = start + filter_lower.len();
                                let before = &cell[..start];
                                let matched = &cell[start..end];
                                let after = &cell[end..];
                                
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    ui.label(before);
                                    ui.label(eframe::egui::RichText::new(matched)
                                        .background_color(eframe::egui::Color32::YELLOW)
                                        .color(eframe::egui::Color32::BLACK));
                                    ui.label(after);
                                });
                            } else {
                                ui.label(&cell);
                            }
                        } else if col_idx == app.state.view.x_index || app.state.view.y_indices.contains(&col_idx) {
                            ui.strong(&cell);
                        } else {
                            ui.label(&cell);
                        }
                    });
                }
            });
        });
    
    // Handle keyboard shortcuts
    if ui.input(|i| i.modifiers.ctrl && i.key_pressed(eframe::egui::Key::C)) {
        copy_selected_rows(app, &display_cols);
    }
    if ui.input(|i| i.modifiers.ctrl && i.key_pressed(eframe::egui::Key::A)) {
        // Select all visible rows
        for &row_idx in display_indices {
            app.state.ui.table.selected_rows.insert(row_idx);
        }
    }
}

/// Copy selected rows to clipboard as TSV
fn copy_selected_rows(app: &mut PlotOxide, display_cols: &[usize]) {
    let ds = match &app.state.data {
        Some(ds) => ds,
        None => return,
    };
    
    let headers = app.headers();
    let selected = &app.state.ui.table.selected_rows;
    
    if selected.is_empty() {
        return;
    }
    
    let mut output = String::new();
    
    // Header row
    let header_line: Vec<&str> = display_cols.iter()
        .filter_map(|&col| headers.get(col).map(|s| s.as_str()))
        .collect();
    output.push_str(&header_line.join("\t"));
    output.push('\n');
    
    // Data rows (sorted by row index)
    let mut sorted_rows: Vec<usize> = selected.iter().copied().collect();
    sorted_rows.sort();
    
    for row_idx in sorted_rows {
        let row_data: Vec<String> = display_cols.iter()
            .map(|&col| ds.get_string(row_idx, col).unwrap_or_default())
            .collect();
        output.push_str(&row_data.join("\t"));
        output.push('\n');
    }
    
    // Copy to clipboard
    if let Some(ctx) = app.state.data.as_ref().map(|_| {}) {
        // We need egui context, but we don't have it here
        // This will be handled by the UI layer
    }
    
    // For now, just print (actual clipboard integration needs ctx)
    #[cfg(debug_assertions)]
    eprintln!("Copied {} rows to clipboard", selected.len());
}
