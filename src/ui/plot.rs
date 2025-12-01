use crate::app::PlotOxide;
use crate::state::{PlotMode, LineStyle};
use chrono::{DateTime, Utc}; // Removed TimeZone
use egui_plot::{Bar, BarChart, BoxElem, BoxPlot, BoxSpread, HLine, Line, Plot, Points};

/// Render the main plot area
pub fn render_plot(app: &mut PlotOxide, ctx: &eframe::egui::Context, ui: &mut eframe::egui::Ui) {
    profiling::scope!("render_plot");
    
    // Get data source directly to avoid materializing row-major data
    let ds = if let Some(ds) = &app.state.data {
        ds
    } else {
        return;
    };
    let headers = app.headers();

    // Helper to get column data safely (clones data for compatibility with non-optimized plot modes)
    let get_col_data = |col_idx: usize| -> Vec<f64> {
        ds.get_cached_column(col_idx)
            .map(|r| r.to_vec())
            .unwrap_or_default()
    };

    // Pre-calculate statistics for outlier filtering (performance optimization)
    // We need to ensure cache is populated before borrowing multiple times
    let needed_cols: Vec<usize> = std::iter::once(app.state.view.x_index)
        .chain(app.state.view.y_indices.iter().copied())
        .collect();
    
    for col in &needed_cols {
        let _ = ds.get_cached_column(*col); 
    }

    // Calculate outlier stats from FILTERED data (after X/Y range filters)
    // This ensures outlier detection is relative to the visible dataset
    if app.state.filters.filter_outliers {
        app.state.outlier_stats_cache.clear();
        let use_row_index = app.state.view.use_row_index;
        let x_index = app.state.view.x_index;

        for &y_idx in &app.state.view.y_indices {
            if let Ok(y_ref) = ds.get_cached_column(y_idx) {
                // Apply X/Y range filters to get the subset of data
                let filtered_values: Vec<f64> = if use_row_index {
                    let x_iter = (0..ds.height()).map(|i| i as f64);
                    x_iter.zip(y_ref.iter())
                        .filter_map(|(x_val, &y_val)| {
                            if app.passes_non_outlier_filters(x_val, y_val) {
                                Some(y_val)
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    if let Ok(x_ref) = ds.get_cached_column(x_index) {
                        x_ref.iter().zip(y_ref.iter())
                            .filter_map(|(&x_val, &y_val)| {
                                if app.passes_non_outlier_filters(x_val, y_val) {
                                    Some(y_val)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    } else {
                        y_ref.to_vec()
                    }
                };

                // Calculate stats from filtered data
                if !filtered_values.is_empty() {
                    let stats = PlotOxide::calculate_statistics(&filtered_values);
                    app.state.outlier_stats_cache.insert(y_idx, stats);
                }
            }
        }
    }

    // Detect if user is currently interacting (Phase 4.3)
    let is_dragging = ctx.input(|i| i.pointer.is_decidedly_dragging());

    // Extract filter parameters to avoid borrow conflicts
    let downsample_threshold = app.state.view.downsample_threshold;
    let use_row_index = app.state.view.use_row_index;
    let x_index = app.state.view.x_index;
    let y_indices = app.state.view.y_indices.clone();

    // Create data for all series with filtering and optimized downsampling
    let all_series: Vec<Vec<[f64; 2]>> = {
        profiling::scope!("series_data_prep");

        // Process each series
        let mut series_data = Vec::new();

        for &y_idx in &y_indices {
            let y_ref = match ds.get_cached_column(y_idx) {
                Ok(r) => r,
                Err(_) => {
                    series_data.push(Vec::new());
                    continue;
                }
            };

            // Prepare points using iterators to avoid allocating intermediate X/Y vectors
            let points: Vec<[f64; 2]> = if use_row_index {
                let x_iter = (0..ds.height()).map(|i| i as f64);
                x_iter.zip(y_ref.iter())
                    .enumerate()
                    .filter_map(|(row_idx, (x_val, &y_val))| {
                        if app.passes_filters(row_idx, x_val, y_val, y_idx) {
                            Some([x_val, y_val])
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                if let Ok(x_ref) = ds.get_cached_column(x_index) {
                    x_ref.iter().zip(y_ref.iter())
                        .enumerate()
                        .filter_map(|(row_idx, (&x_val, &y_val))| {
                            if app.passes_filters(row_idx, x_val, y_val, y_idx) {
                                Some([x_val, y_val])
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            };

            // Downsample if dataset is large (Phase 4.3: adaptive downsampling)
            let downsampled = if points.len() > downsample_threshold {
                // Convert to tuple format for downsampler
                let tuple_points: Vec<(f64, f64)> = points.iter().map(|p| (p[0], p[1])).collect();
                app.state.downsampler.downsample(&tuple_points, downsample_threshold, is_dragging)
            } else {
                points
            };

            series_data.push(downsampled);
        }

        series_data
    };

    // Detect modifier keys for constrained zoom
    let shift_held = ctx.input(|i| i.modifiers.shift);
    let ctrl_held = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);
    let alt_held = ctx.input(|i| i.modifiers.alt);

    // Calculate plot height to fill available space
    let available_height = ui.available_height();
    // Ensure a minimum height for usability, but otherwise fill the space
    let plot_height = available_height.max(200.0);

    let mut plot = Plot::new("plot")
        .allow_zoom(app.state.view.allow_zoom)
        .allow_drag(app.state.view.allow_drag && !alt_held)
        .allow_boxed_zoom(app.state.view.allow_zoom && alt_held)
        .allow_scroll(app.state.view.allow_zoom)
        .show_grid(app.state.view.show_grid)
        .height(plot_height);

    // Apply axis-locked zoom if modifiers held
    if shift_held && app.state.view.allow_zoom {
        plot = plot.allow_zoom([true, false]).allow_boxed_zoom(false); // X-only
    } else if ctrl_held && app.state.view.allow_zoom {
        plot = plot.allow_zoom([false, true]).allow_boxed_zoom(false); // Y-only
    }

    if app.state.view.reset_bounds {
        plot = plot.reset();
        app.state.view.reset_bounds = false;
    }

    if app.state.view.show_legend {
        plot = plot.legend(egui_plot::Legend::default().position(egui_plot::Corner::RightTop));
    }

    // Add custom axis formatters for timestamps
    if app.state.view.x_is_timestamp {
        plot = plot.x_axis_formatter(|mark, _range| {
            // Handle fractional seconds by extracting seconds and nanoseconds
            let secs = mark.value.floor() as i64;
            let nanos = ((mark.value.fract() * 1_000_000_000.0) as u32).min(999_999_999);

            if let Some(dt) = DateTime::<Utc>::from_timestamp(secs, nanos) {
                // Use newline for better readability on x-axis
                dt.format("%Y-%m-%d\n%H:%M:%S").to_string()
            } else {
                format!("{:.2}", mark.value)
            }
        })
        .label_formatter(|name, value| {
            if name.is_empty() {
                let secs = value.x.floor() as i64;
                let nanos = ((value.x.fract() * 1_000_000_000.0) as u32).min(999_999_999);
                if let Some(dt) = DateTime::<Utc>::from_timestamp(secs, nanos) {
                    format!("{}\n{:.2}", dt.format("%Y-%m-%d %H:%M:%S"), value.y)
                } else {
                    format!("x: {:.3}\ny: {:.2}", value.x, value.y)
                }
            } else {
                format!("{}\nx: {:.3}\ny: {:.2}", name, value.x, value.y)
            }
        });
    } else {
        // Ensure x-axis labels always render with sensible formatting
        plot = plot.x_axis_formatter(|mark, _range| {
            if mark.value.abs() < 0.01 && mark.value != 0.0 {
                format!("{:.2e}", mark.value)
            } else if mark.value.abs() >= 1000.0 {
                format!("{:.0}", mark.value)
            } else {
                format!("{:.2}", mark.value)
            }
        });
    }

    let mut plot_response = plot.show(ui, |plot_ui| {
        match app.state.view.plot_mode {
            PlotMode::Scatter => {
                // Plot each series in scatter mode
                for (series_idx, (&y_idx, points_data)) in app.state.view.y_indices.iter().zip(&all_series).enumerate() {
                    let color = PlotOxide::get_series_color(series_idx);
                    let name = &headers[y_idx];

            // Draw sigma zone lines if enabled
            if app.state.spc.show_sigma_zones && !points_data.is_empty() {
                let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                let (mean, std_dev) = PlotOxide::calculate_statistics(&y_values);

                // ±1σ lines (blue)
                plot_ui.hline(HLine::new(format!("{} +1σ", name), mean + 1.0 * std_dev)
                    .color(eframe::egui::Color32::from_rgb(150, 150, 255))
                    .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                    .width(1.0));
                plot_ui.hline(HLine::new(format!("{} -1σ", name), mean - 1.0 * std_dev)
                    .color(eframe::egui::Color32::from_rgb(150, 150, 255))
                    .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                    .width(1.0));

                // ±2σ lines (orange)
                plot_ui.hline(HLine::new(format!("{} +2σ", name), mean + 2.0 * std_dev)
                    .color(eframe::egui::Color32::from_rgb(255, 200, 100))
                    .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                    .width(1.0));
                plot_ui.hline(HLine::new(format!("{} -2σ", name), mean - 2.0 * std_dev)
                    .color(eframe::egui::Color32::from_rgb(255, 200, 100))
                    .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                    .width(1.0));

                // ±3σ lines (red)
                plot_ui.hline(HLine::new(format!("{} +3σ", name), mean + 3.0 * std_dev)
                    .color(eframe::egui::Color32::from_rgb(255, 150, 150))
                    .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                    .width(1.0));
                plot_ui.hline(HLine::new(format!("{} -3σ", name), mean - 3.0 * std_dev)
                    .color(eframe::egui::Color32::from_rgb(255, 150, 150))
                    .style(egui_plot::LineStyle::Dotted { spacing: 8.0 })
                    .width(1.0));
            }

            // Draw specification limits if capability enabled
            if app.state.spc.show_capability {
                plot_ui.hline(
                    HLine::new("LSL", app.state.spc.spec_lower)
                        .color(eframe::egui::Color32::from_rgb(255, 140, 0))
                        .style(egui_plot::LineStyle::Solid)
                        .width(2.0),
                );
                plot_ui.hline(
                    HLine::new("USL", app.state.spc.spec_upper)
                        .color(eframe::egui::Color32::from_rgb(255, 140, 0))
                        .style(egui_plot::LineStyle::Solid)
                        .width(2.0),
                );
            }

            // Draw SPC control limits if enabled
            if app.state.spc.show_spc_limits {
                let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                let (mean, std_dev) = PlotOxide::calculate_statistics(&y_values);
                let ucl = mean + app.state.spc.sigma_multiplier * std_dev;
                let lcl = mean - app.state.spc.sigma_multiplier * std_dev;

                // Center line (mean)
                plot_ui.hline(
                    HLine::new(format!("{} Mean", name), mean)
                        .color(color)
                        .style(egui_plot::LineStyle::Dashed { length: 8.0 })
                        .width(1.5),
                );

                // Upper control limit
                plot_ui.hline(
                    HLine::new(format!("{} UCL", name), ucl)
                        .color(eframe::egui::Color32::RED)
                        .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                        .width(2.0),
                );

                // Lower control limit
                plot_ui.hline(
                    HLine::new(format!("{} LCL", name), lcl)
                        .color(eframe::egui::Color32::RED)
                        .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                        .width(2.0),
                );
            }

            // Draw data series
            match app.state.view.line_style {
                LineStyle::Line => {
                    plot_ui.line(Line::new(name, points_data.clone()).color(color));
                }
                LineStyle::Points => {
                    plot_ui.points(Points::new(name, points_data.clone()).radius(3.0).color(color));
                }
                LineStyle::LineAndPoints => {
                    plot_ui.line(Line::new(name, points_data.clone()).color(color));
                    plot_ui.points(Points::new(name, points_data.clone()).radius(3.0).color(color));
                }
            }

            // Highlight outliers
            if app.state.spc.show_outliers {
                let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                let outlier_indices = PlotOxide::detect_outliers(&y_values, app.state.spc.outlier_threshold);
                let outlier_points: Vec<[f64; 2]> = outlier_indices.iter()
                    .map(|&i| points_data[i])
                    .collect();

                if !outlier_points.is_empty() {
                    plot_ui.points(
                        Points::new(format!("{} Outliers", name), outlier_points)
                            .color(eframe::egui::Color32::RED)
                            .filled(true)
                            .radius(5.0)
                            .shape(egui_plot::MarkerShape::Diamond),
                    );
                }
            }

            // Highlight Western Electric violations
            if app.state.spc.show_we_rules {
                let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                let we_indices = PlotOxide::detect_western_electric_violations(&y_values);
                let we_points: Vec<[f64; 2]> = we_indices.iter()
                    .map(|&i| points_data[i])
                    .collect();

                if !we_points.is_empty() {
                    plot_ui.points(
                        Points::new(format!("{} WE Violations", name), we_points)
                            .color(eframe::egui::Color32::from_rgb(255, 165, 0))
                            .filled(false)
                            .radius(7.0)
                            .shape(egui_plot::MarkerShape::Square),
                    );
                }
            }

            // Highlight selected point
            if let Some((sel_series, sel_point)) = app.state.view.selected_point {
                if series_idx == sel_series && sel_point < points_data.len() {
                    plot_ui.points(
                        Points::new("", vec![points_data[sel_point]])
                            .color(eframe::egui::Color32::from_rgb(255, 215, 0))
                            .filled(false)
                            .radius(10.0)
                            .shape(egui_plot::MarkerShape::Circle),
                    );
                }
            }

            // Highlight table-hovered point (use white for visibility)
            if let Some(row_idx) = app.state.view.table_hovered_row {
                if row_idx < points_data.len() {
                    plot_ui.points(
                        Points::new("", vec![points_data[row_idx]])
                            .color(eframe::egui::Color32::WHITE)
                            .filled(true)
                            .radius(6.0)
                            .shape(egui_plot::MarkerShape::Circle),
                    );
                }
            }

            // Draw moving average if enabled
            if app.state.spc.show_moving_avg && points_data.len() >= app.state.spc.ma_window {
                let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                let ma_y = PlotOxide::calculate_sma(&y_values, app.state.spc.ma_window);

                // Map MA y-values to actual x-values
                let ma_points: Vec<[f64; 2]> = ma_y.iter()
                    .map(|&[idx, y_val]| {
                        let actual_x = points_data[idx as usize][0];
                        [actual_x, y_val]
                    })
                    .collect();

                if !ma_points.is_empty() {
                    let ma_color = eframe::egui::Color32::from_rgb(100, 100, 100);
                    plot_ui.line(
                        Line::new(format!("{} MA({})", name, app.state.spc.ma_window), ma_points)
                            .color(ma_color)
                            .style(egui_plot::LineStyle::Dashed { length: 5.0 })
                            .width(1.5),
                    );
                }
            }

            // Draw EWMA if enabled
            if app.state.spc.show_ewma && !points_data.is_empty() {
                let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();
                let ewma_y = PlotOxide::calculate_ewma(&y_values, app.state.spc.ewma_lambda);

                // Map EWMA y-values to actual x-values
                let ewma_points: Vec<[f64; 2]> = ewma_y.iter()
                    .map(|&[idx, y_val]| {
                        let actual_x = points_data[idx as usize][0];
                        [actual_x, y_val]
                    })
                    .collect();

                if !ewma_points.is_empty() {
                    let ewma_color = eframe::egui::Color32::from_rgb(80, 150, 80);
                    plot_ui.line(
                        Line::new(format!("{} EWMA(λ={:.2})", name, app.state.spc.ewma_lambda), ewma_points)
                            .color(ewma_color)
                            .style(egui_plot::LineStyle::Solid)
                            .width(2.0),
                    );
                }
            }

            // Draw regression if enabled
            if app.state.spc.show_regression && points_data.len() >= app.state.spc.regression_order + 1 {
                if app.state.spc.regression_order == 1 {
                    // Linear regression
                    if let Some((slope, intercept, r2)) = PlotOxide::linear_regression(points_data) {
                        let x_min = points_data.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                        let x_max = points_data.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);

                        let reg_line = vec![
                            [x_min, slope * x_min + intercept],
                            [x_max, slope * x_max + intercept],
                        ];

                        plot_ui.line(
                            Line::new(format!("{} Lin (R²={:.3})", name, r2), reg_line)
                                .color(eframe::egui::Color32::from_rgb(180, 100, 180))
                                .style(egui_plot::LineStyle::Solid)
                                .width(2.0),
                        );
                    }
                } else {
                    // Polynomial regression
                    if let Some((coeffs, r2)) = PlotOxide::polynomial_regression(points_data, app.state.spc.regression_order) {
                        let x_min = points_data.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                        let x_max = points_data.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);

                        let num_points = 100;
                        let step = (x_max - x_min) / num_points as f64;
                        let poly_line: Vec<[f64; 2]> = (0..=num_points)
                            .map(|i| {
                                    let x = x_min + i as f64 * step;
                                    let y: f64 = coeffs.iter().enumerate()
                                        .map(|(i, &c)| c * x.powi(i as i32))
                                        .sum();
                                    [x, y]
                                })
                                .collect();

                            plot_ui.line(
                                Line::new(format!("{} Poly{} (R²={:.3})", name, app.state.spc.regression_order, r2), poly_line)
                                    .color(eframe::egui::Color32::from_rgb(180, 100, 180))
                                    .style(egui_plot::LineStyle::Solid)
                                    .width(2.0),
                            );
                        }
                    }
                }

                    }
                }
                PlotMode::Histogram => {
                    // Histogram mode - show histogram with proper bin widths
                    // Note: We ignore the X-axis data and work only with Y-values
                    for (series_idx, &y_idx) in app.state.view.y_indices.iter().enumerate() {
                        let color = PlotOxide::get_series_color(series_idx);
                        let name = &headers[y_idx];

                        // Get Y values directly from data (column-wise)
                        let y_values: Vec<f64> = get_col_data(y_idx)
                            .into_iter()
                            .filter(|&v| !v.is_nan() && v.is_finite())
                            .collect();

                        let (hist_data, _min, bin_width) = PlotOxide::calculate_histogram(&y_values, app.state.view.histogram_bins);

                        if !hist_data.is_empty() {
                            // Calculate bar width based on bin_width and number of series
                            let bar_width = bin_width * 0.9 / app.state.view.y_indices.len() as f64;
                            let offset = (series_idx as f64 - (app.state.view.y_indices.len() - 1) as f64 / 2.0) * bar_width;

                            let bars: Vec<Bar> = hist_data.iter()
                                .map(|&[x, count]| {
                                    // X is the bin left edge, center within bin and add offset for multiple series
                                    Bar::new(x + bin_width / 2.0 + offset, count).width(bar_width)
                                })
                                .collect();

                            plot_ui.bar_chart(
                                BarChart::new(name.clone(), bars)
                                    .color(color)
                            );
                        }
                    }
                }
                PlotMode::BoxPlot => {
                    // Box plot mode - only show box plots
                    // We use all_series here because it already contains the filtered/processed points for the plot
                    for (series_idx, (&y_idx, points_data)) in app.state.view.y_indices.iter().zip(&all_series).enumerate() {
                        let color = PlotOxide::get_series_color(series_idx);
                        let name = &headers[y_idx];
                        let y_values: Vec<f64> = points_data.iter().map(|p| p[1]).collect();

                        if let Some((lower_whisker, q1, median, q3, upper_whisker)) = PlotOxide::calculate_boxplot_stats(&y_values) {
                            let x_pos = series_idx as f64;

                            let box_elem = BoxElem::new(x_pos, BoxSpread::new(lower_whisker, q1, median, q3, upper_whisker));
                            plot_ui.box_plot(
                                BoxPlot::new(format!("{}", name), vec![box_elem])
                                    .color(color)
                            );
                        }
                    }
                }
                PlotMode::Pareto => {
                    // Pareto chart mode - frequency bars + cumulative line
                    for (series_idx, &y_idx) in app.state.view.y_indices.iter().enumerate() {
                        let color = PlotOxide::get_series_color(series_idx);
                        let name = &headers[y_idx];

                        // Get Y values directly from data (column-wise)
                        let y_values: Vec<f64> = get_col_data(y_idx)
                            .into_iter()
                            .filter(|&v| !v.is_nan() && v.is_finite())
                            .collect();

                        let (freq_data, cumulative_pct) = PlotOxide::calculate_pareto(&y_values);

                        if !freq_data.is_empty() {
                            // Draw frequency bars
                            let bar_width = 0.8 / app.state.view.y_indices.len() as f64;
                            let offset = (series_idx as f64 - (app.state.view.y_indices.len() - 1) as f64 / 2.0) * bar_width;

                            let bars: Vec<Bar> = freq_data.iter()
                                .enumerate()
                                .map(|(i, &(_, count))| {
                                    Bar::new(i as f64 + offset, count as f64).width(bar_width)
                                })
                                .collect();

                            plot_ui.bar_chart(
                                BarChart::new(format!("{} Frequency", name), bars)
                                    .color(color)
                            );

                            // Draw cumulative percentage line (scaled to match bar heights)
                            let max_count = freq_data.iter().map(|(_, c)| c).max().unwrap_or(&1);
                            let scale_factor = *max_count as f64 / 100.0;

                            let cumulative_line: Vec<[f64; 2]> = cumulative_pct.iter()
                                .enumerate()
                                .map(|(i, &pct)| [i as f64, pct * scale_factor])
                                .collect();

                            plot_ui.line(
                                Line::new(format!("{} Cumulative %", name), cumulative_line)
                                    .color(eframe::egui::Color32::RED)
                                    .style(egui_plot::LineStyle::Solid)
                                    .width(2.5)
                            );

                            // Draw 80% line (Pareto principle)
                            let line_80 = 80.0 * scale_factor;
                            plot_ui.hline(
                                HLine::new("80% Line", line_80)
                                    .color(eframe::egui::Color32::from_rgb(255, 165, 0))
                                    .style(egui_plot::LineStyle::Dashed { length: 8.0 })
                                    .width(2.0)
                            );
                        }
                    }
                }
                PlotMode::XbarR => {
                    // X-bar and R chart mode - shows process mean and range control charts
                    // This mode requires displaying TWO charts, so we'll show only the first Y-series
                    if let Some(&y_idx) = app.state.view.y_indices.first() {
                        let color = PlotOxide::get_series_color(0);
                        let name = &headers[y_idx];

                        // Get Y values directly from data (column-wise)
                        let y_values: Vec<f64> = get_col_data(y_idx)
                            .into_iter()
                            .filter(|&v| !v.is_nan() && v.is_finite())
                            .collect();

                        let (xbar_points, r_points, xbar_mean, xbar_ucl, xbar_lcl, r_mean, r_ucl, r_lcl) =
                            PlotOxide::calculate_xbarr(&y_values, app.state.spc.xbarr_subgroup_size);

                        if !xbar_points.is_empty() {
                            // Note: egui_plot doesn't support dual Y-axes easily
                            // We'll draw X-bar and R on the same plot with different colors

                            // Draw X-bar points and lines
                            plot_ui.line(Line::new(format!("{} X-bar", name), xbar_points.clone()).color(color));
                            plot_ui.points(Points::new(format!("{} X-bar", name), xbar_points.clone()).radius(4.0).color(color));

                            // X-bar control limits
                            plot_ui.hline(
                                HLine::new(format!("{} X-bar Mean", name), xbar_mean)
                                    .color(color)
                                    .style(egui_plot::LineStyle::Dashed { length: 8.0 })
                                    .width(2.0)
                            );
                            plot_ui.hline(
                                HLine::new(format!("{} X-bar UCL", name), xbar_ucl)
                                    .color(eframe::egui::Color32::RED)
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                    .width(2.0)
                            );
                            plot_ui.hline(
                                HLine::new(format!("{} X-bar LCL", name), xbar_lcl)
                                    .color(eframe::egui::Color32::RED)
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                    .width(2.0)
                            );

                            // Draw R points and lines (with different color)
                            let r_color = eframe::egui::Color32::from_rgb(255, 127, 14); // Orange
                            plot_ui.line(Line::new(format!("{} R-chart", name), r_points.clone()).color(r_color));
                            plot_ui.points(Points::new(format!("{} R-chart", name), r_points.clone()).radius(4.0).color(r_color).shape(egui_plot::MarkerShape::Diamond));

                            // R control limits
                            plot_ui.hline(
                                HLine::new(format!("{} R Mean", name), r_mean)
                                    .color(r_color)
                                    .style(egui_plot::LineStyle::Dashed { length: 8.0 })
                                    .width(2.0)
                            );
                            plot_ui.hline(
                                HLine::new(format!("{} R UCL", name), r_ucl)
                                    .color(eframe::egui::Color32::from_rgb(200, 0, 0))
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                    .width(2.0)
                            );
                            if r_lcl > 0.0 {
                                plot_ui.hline(
                                    HLine::new(format!("{} R LCL", name), r_lcl)
                                        .color(eframe::egui::Color32::from_rgb(200, 0, 0))
                                        .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                        .width(2.0)
                                );
                            }
                        }
                    }
                }
                PlotMode::PChart => {
                    // p-chart mode - proportion defective control chart
                    if let Some(&y_idx) = app.state.view.y_indices.first() {
                        let color = PlotOxide::get_series_color(0);
                        let name = &headers[y_idx];

                        // Get Y values (number of defects per sample) (column-wise)
                        let defects: Vec<f64> = get_col_data(y_idx)
                            .into_iter()
                            .filter(|&v| !v.is_nan() && v.is_finite())
                            .collect();

                        let (proportions, p_bar, ucl, lcl) = PlotOxide::calculate_pchart(&defects, app.state.spc.pchart_sample_size);

                        if !proportions.is_empty() {
                            // Draw proportion points and line
                            plot_ui.line(Line::new(format!("{} Proportion", name), proportions.clone()).color(color));
                            plot_ui.points(Points::new(format!("{} Proportion", name), proportions.clone()).radius(4.0).color(color));

                            // Draw p-bar (center line)
                            plot_ui.hline(
                                HLine::new("p-bar (avg proportion)", p_bar)
                                    .color(color)
                                    .style(egui_plot::LineStyle::Dashed { length: 8.0 })
                                    .width(2.0)
                            );

                            // Draw UCL
                            plot_ui.hline(
                                HLine::new("UCL", ucl)
                                    .color(eframe::egui::Color32::RED)
                                    .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                    .width(2.0)
                            );

                            // Draw LCL (if > 0)
                            if lcl > 0.0 {
                                plot_ui.hline(
                                    HLine::new("LCL", lcl)
                                        .color(eframe::egui::Color32::RED)
                                        .style(egui_plot::LineStyle::Dashed { length: 10.0 })
                                        .width(2.0)
                                );
                            }
                        }
                    }
                }
            }
        });

    // Calculate excursions for table highlighting
    let mut all_excursions = std::collections::HashSet::new();
    let mut all_we_violations = Vec::new();

    if app.state.spc.show_outliers || app.state.spc.show_spc_limits || app.state.spc.show_we_rules {
        for series_idx in 0..app.state.view.y_indices.len() {
            // Use column data directly
            let y_values: Vec<f64> = get_col_data(app.state.view.y_indices[series_idx]);

            if app.state.spc.show_outliers {
                let outliers = PlotOxide::detect_outliers(&y_values, app.state.spc.outlier_threshold);
                all_excursions.extend(outliers);
            }

            if app.state.spc.show_spc_limits {
                let (mean, std_dev) = PlotOxide::calculate_statistics(&y_values);
                let ucl = mean + app.state.spc.sigma_multiplier * std_dev;
                let lcl = mean - app.state.spc.sigma_multiplier * std_dev;

                for (i, &v) in y_values.iter().enumerate() {
                    if v > ucl || v < lcl {
                        all_excursions.insert(i);
                    }
                }
            }

            if app.state.spc.show_we_rules {
                let we_detailed = PlotOxide::detect_western_electric_violations_detailed(&y_values);
                for violation in &we_detailed {
                    all_excursions.insert(violation.point_index);
                }
                all_we_violations.extend(we_detailed);
            }
        }
    }
    app.state.spc.excursion_rows = all_excursions.into_iter().collect();
    app.state.spc.we_violations = all_we_violations;

    // Handle right-click context menu
    plot_response.response.context_menu(|ui| {
        if ui.button("Reset View").clicked() {
            app.state.view.reset_bounds = true;
            ui.close();
        }
        if ui.button("Toggle Grid").clicked() {
            app.state.view.show_grid = !app.state.view.show_grid;
            ui.close();
        }
        if ui.button("Toggle Legend").clicked() {
            app.state.view.show_legend = !app.state.view.show_legend;
            ui.close();
        }
        ui.separator();
        if ui.button("Clear Selection").clicked() {
            app.state.view.selected_point = None;
            ui.close();
        }
    });

    // Handle click to select point first
    let was_clicked = plot_response.response.clicked();
    let click_pos = plot_response.response.interact_pointer_pos();

    // Show hover tooltip
    if let Some(pointer_pos) = plot_response.response.hover_pos() {
        let plot_pos = plot_response.transform.value_from_position(pointer_pos);

        // Find closest point across all series
        let mut closest_series_idx = 0;
        let mut closest_point_idx = 0;
        let mut min_dist = f64::INFINITY;

        for (series_idx, points_data) in all_series.iter().enumerate() {
            for (point_idx, point) in points_data.iter().enumerate() {
                let dx = (point[0] - plot_pos.x) / (plot_response.transform.bounds().width());
                let dy = (point[1] - plot_pos.y) / (plot_response.transform.bounds().height());
                let dist = dx * dx + dy * dy;

                if dist < min_dist {
                    min_dist = dist;
                    closest_series_idx = series_idx;
                    closest_point_idx = point_idx;
                }
            }
        }

        // Only show tooltip if close enough
        if min_dist < 0.0004 {
            app.state.view.hovered_point = Some((closest_series_idx, closest_point_idx));
            let point = &all_series[closest_series_idx][closest_point_idx];
            let y_idx = app.state.view.y_indices[closest_series_idx];

            let x_label = if app.state.view.x_is_timestamp {
                DateTime::<Utc>::from_timestamp(point[0] as i64, 0)
                    .map(|dt| {
                        // Show full timestamp in tooltip with milliseconds if present
                        let frac = point[0].fract();
                        if frac.abs() > 0.001 {
                            format!("{}.{:03}", dt.format("%Y-%m-%d %H:%M:%S"), (frac * 1000.0).abs() as u32)
                        } else {
                            dt.format("%Y-%m-%d %H:%M:%S").to_string()
                        }
                    })
                    .unwrap_or_else(|| format!("{:.3}", point[0]))
            } else {
                format!("{:.4}", point[0])
            };

            let y_is_timestamp = app.is_column_timestamp(y_idx);
            let y_label = if y_is_timestamp {
                DateTime::<Utc>::from_timestamp(point[1] as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| format!("{:.2}", point[1]))
            } else {
                format!("{:.2}", point[1])
            };

            plot_response.response = plot_response.response.on_hover_ui(|ui| {
                ui.label(format!("Row: {}", closest_point_idx + 1));
                ui.label(format!("{}: {}", headers[app.state.view.x_index], x_label));
                let color = PlotOxide::get_series_color(closest_series_idx);
                ui.colored_label(color, format!("{}: {}", headers[y_idx], y_label));

                // Show WE rule violations if any
                if app.state.spc.show_we_rules {
                    for violation in &app.state.spc.we_violations {
                        if violation.point_index == closest_point_idx {
                            ui.separator();
                            ui.colored_label(eframe::egui::Color32::from_rgb(255, 165, 0), "⚠ WE Rule Violations:");
                            for rule in &violation.rules {
                                ui.label(format!("  • {}", rule));
                            }
                            break;
                        }
                    }
                }
            });
        } else {
            app.state.view.hovered_point = None;
        }
    } else {
        app.state.view.hovered_point = None;
    }

    // Process click event (stored before on_hover_ui consumed the response)
    if was_clicked {
        if let Some(pointer_pos) = click_pos {
            let plot_pos = plot_response.transform.value_from_position(pointer_pos);

            // Find closest point across all series
            let mut closest_series_idx = 0;
            let mut closest_point_idx = 0;
            let mut min_dist = f64::INFINITY;

            for (series_idx, points_data) in all_series.iter().enumerate() {
                for (point_idx, point) in points_data.iter().enumerate() {
                    let dx = (point[0] - plot_pos.x) / (plot_response.transform.bounds().width());
                    let dy = (point[1] - plot_pos.y) / (plot_response.transform.bounds().height());
                    let dist = dx * dx + dy * dy;

                    if dist < min_dist {
                        min_dist = dist;
                        closest_series_idx = series_idx;
                        closest_point_idx = point_idx;
                    }
                }
            }

            // Select if close enough, otherwise deselect
            if min_dist < 0.0004 {
                // Clear selection if switching to a different series
                if let Some((prev_series, _)) = app.state.view.selected_point {
                    if prev_series != closest_series_idx {
                        app.state.view.selected_point = None;
                    }
                }
                app.state.view.selected_point = Some((closest_series_idx, closest_point_idx));
                app.state.ui.scroll_to_row = Some(closest_point_idx);
            } else {
                app.state.view.selected_point = None;
            }
        }
    }
    
    // === Edge Indicators & Minimap ===
    // Draw indicators when data extends beyond visible area

    // Only show edge indicators and minimap if there's actual data
    let has_data = all_series.iter().any(|s| !s.is_empty());

    if !has_data {
        return;
    }

    // Calculate actual data bounds across all series
    let (data_x_min, data_x_max, data_y_min, data_y_max) = {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        
        for series in &all_series {
            for point in series {
                if point[0].is_finite() {
                    x_min = x_min.min(point[0]);
                    x_max = x_max.max(point[0]);
                }
                if point[1].is_finite() {
                    y_min = y_min.min(point[1]);
                    y_max = y_max.max(point[1]);
                }
            }
        }
        (x_min, x_max, y_min, y_max)
    };
    
    // Get current view bounds
    let view_bounds = plot_response.transform.bounds();
    let view_x_min = view_bounds.min()[0];
    let view_x_max = view_bounds.max()[0];
    let view_y_min = view_bounds.min()[1];
    let view_y_max = view_bounds.max()[1];

    // Calculate zoom and pan state to determine if user has interacted with the view
    // This prevents false positives when series are hidden via legend toggle
    let view_x_range = view_x_max - view_x_min;
    let view_y_range = view_y_max - view_y_min;
    let data_x_range = data_x_max - data_x_min;
    let data_y_range = data_y_max - data_y_min;

    let x_zoom_ratio = if view_x_range > 0.0 && data_x_range > 0.0 {
        data_x_range / view_x_range
    } else {
        1.0
    };
    let y_zoom_ratio = if view_y_range > 0.0 && data_y_range > 0.0 {
        data_y_range / view_y_range
    } else {
        1.0
    };

    // Check if view is zoomed in
    let is_zoomed = x_zoom_ratio > 1.5 || y_zoom_ratio > 1.5;

    // Check if view is panned (center offset from data center by >10% of range)
    let view_x_center = (view_x_min + view_x_max) / 2.0;
    let view_y_center = (view_y_min + view_y_max) / 2.0;
    let data_x_center = (data_x_min + data_x_max) / 2.0;
    let data_y_center = (data_y_min + data_y_max) / 2.0;

    let x_offset_ratio = if data_x_range > 0.0 {
        (view_x_center - data_x_center).abs() / data_x_range
    } else {
        0.0
    };
    let y_offset_ratio = if data_y_range > 0.0 {
        (view_y_center - data_y_center).abs() / data_y_range
    } else {
        0.0
    };

    let is_panned = x_offset_ratio > 0.15 || y_offset_ratio > 0.15;

    // Only show edge indicators if user has zoomed or panned
    if !is_zoomed && !is_panned {
        return; // Auto-fit view, no indicators needed
    }

    // Check which directions have data SIGNIFICANTLY outside view (>15% of view range)
    // This prevents showing indicators for hidden series or minor auto-fit variations
    let x_threshold = view_x_range * 0.15;
    let y_threshold = view_y_range * 0.15;

    let has_left = data_x_min < (view_x_min - x_threshold);
    let has_right = data_x_max > (view_x_max + x_threshold);
    let has_bottom = data_y_min < (view_y_min - y_threshold);
    let has_top = data_y_max > (view_y_max + y_threshold);

    // If no directions have significant data outside, don't show any indicators
    if !has_left && !has_right && !has_bottom && !has_top {
        return;
    }

    let plot_rect = plot_response.response.rect;
    let painter = ui.painter();
    
    // Draw gradient edge indicators
    let indicator_width = 20.0;
    let indicator_color = eframe::egui::Color32::from_rgba_unmultiplied(100, 150, 255, 60);
    let arrow_color = eframe::egui::Color32::from_rgba_unmultiplied(100, 150, 255, 180);
    
    if has_left {
        // Left gradient
        let gradient_rect = eframe::egui::Rect::from_min_max(
            plot_rect.left_top(),
            eframe::egui::pos2(plot_rect.left() + indicator_width, plot_rect.bottom()),
        );
        painter.rect_filled(gradient_rect, 0.0, indicator_color);
        // Arrow
        let arrow_center = eframe::egui::pos2(plot_rect.left() + 8.0, plot_rect.center().y);
        painter.text(
            arrow_center,
            eframe::egui::Align2::CENTER_CENTER,
            "<",
            eframe::egui::FontId::monospace(16.0),
            arrow_color,
        );
    }

    if has_right {
        // Right gradient
        let gradient_rect = eframe::egui::Rect::from_min_max(
            eframe::egui::pos2(plot_rect.right() - indicator_width, plot_rect.top()),
            plot_rect.right_bottom(),
        );
        painter.rect_filled(gradient_rect, 0.0, indicator_color);
        // Arrow
        let arrow_center = eframe::egui::pos2(plot_rect.right() - 8.0, plot_rect.center().y);
        painter.text(
            arrow_center,
            eframe::egui::Align2::CENTER_CENTER,
            ">",
            eframe::egui::FontId::monospace(16.0),
            arrow_color,
        );
    }

    if has_bottom {
        // Bottom gradient
        let gradient_rect = eframe::egui::Rect::from_min_max(
            eframe::egui::pos2(plot_rect.left(), plot_rect.bottom() - indicator_width),
            plot_rect.right_bottom(),
        );
        painter.rect_filled(gradient_rect, 0.0, indicator_color);
        // Arrow
        let arrow_center = eframe::egui::pos2(plot_rect.center().x, plot_rect.bottom() - 8.0);
        painter.text(
            arrow_center,
            eframe::egui::Align2::CENTER_CENTER,
            "v",
            eframe::egui::FontId::monospace(16.0),
            arrow_color,
        );
    }

    if has_top {
        // Top gradient
        let gradient_rect = eframe::egui::Rect::from_min_max(
            plot_rect.left_top(),
            eframe::egui::pos2(plot_rect.right(), plot_rect.top() + indicator_width),
        );
        painter.rect_filled(gradient_rect, 0.0, indicator_color);
        // Arrow
        let arrow_center = eframe::egui::pos2(plot_rect.center().x, plot_rect.top() + 8.0);
        painter.text(
            arrow_center,
            eframe::egui::Align2::CENTER_CENTER,
            "^",
            eframe::egui::FontId::monospace(16.0),
            arrow_color,
        );
    }

    // Draw minimap (only when zoomed in, not for panning at 1:1 scale)
    if is_zoomed && data_x_max > data_x_min && data_y_max > data_y_min {
        let minimap_size = 80.0;
        let minimap_margin = 10.0;
        let minimap_rect = eframe::egui::Rect::from_min_size(
            eframe::egui::pos2(
                plot_rect.right() - minimap_size - minimap_margin,
                plot_rect.bottom() - minimap_size - minimap_margin, //- 35.0,  // Positioned above filename
            ),
            eframe::egui::vec2(minimap_size, minimap_size),
        );
        
        // Background
        painter.rect_filled(
            minimap_rect,
            4.0,
            eframe::egui::Color32::from_rgba_unmultiplied(30, 30, 30, 200),
        );
        painter.rect_stroke(
            minimap_rect,
            4.0,
            eframe::egui::Stroke::new(1.0, eframe::egui::Color32::from_rgb(80, 80, 80)),
            eframe::egui::StrokeKind::Inside,
        );
        
        // Draw simplified data outline (just bounding box of each series)
        for (series_idx, series) in all_series.iter().enumerate() {
            if series.is_empty() {
                continue;
            }
            let color = PlotOxide::get_series_color(series_idx).gamma_multiply(0.7);
            
            // Map a few points to minimap coordinates
            let step = (series.len() / 50).max(1);
            let mini_points: Vec<eframe::egui::Pos2> = series.iter()
                .step_by(step)
                .filter_map(|p| {
                    if !p[0].is_finite() || !p[1].is_finite() {
                        return None;
                    }
                    let x_frac = (p[0] - data_x_min) / (data_x_max - data_x_min);
                    let y_frac = (p[1] - data_y_min) / (data_y_max - data_y_min);
                    Some(eframe::egui::pos2(
                        minimap_rect.left() + x_frac as f32 * minimap_rect.width(),
                        minimap_rect.bottom() - y_frac as f32 * minimap_rect.height(),
                    ))
                })
                .collect();
            
            if mini_points.len() >= 2 {
                for window in mini_points.windows(2) {
                    painter.line_segment([window[0], window[1]], eframe::egui::Stroke::new(1.0, color));
                }
            }
        }
        
        // Draw viewport rectangle
        let vp_x_min = ((view_x_min - data_x_min) / (data_x_max - data_x_min)).clamp(0.0, 1.0);
        let vp_x_max = ((view_x_max - data_x_min) / (data_x_max - data_x_min)).clamp(0.0, 1.0);
        let vp_y_min = ((view_y_min - data_y_min) / (data_y_max - data_y_min)).clamp(0.0, 1.0);
        let vp_y_max = ((view_y_max - data_y_min) / (data_y_max - data_y_min)).clamp(0.0, 1.0);
        
        let viewport_rect = eframe::egui::Rect::from_min_max(
            eframe::egui::pos2(
                minimap_rect.left() + vp_x_min as f32 * minimap_rect.width(),
                minimap_rect.bottom() - vp_y_max as f32 * minimap_rect.height(),
            ),
            eframe::egui::pos2(
                minimap_rect.left() + vp_x_max as f32 * minimap_rect.width(),
                minimap_rect.bottom() - vp_y_min as f32 * minimap_rect.height(),
            ),
        );
        
        painter.rect_filled(
            viewport_rect,
            2.0,
            eframe::egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
        );
        painter.rect_stroke(
            viewport_rect,
            2.0,
            eframe::egui::Stroke::new(1.5, eframe::egui::Color32::from_rgb(200, 200, 200)),
            eframe::egui::StrokeKind::Inside,
        );
    }
}