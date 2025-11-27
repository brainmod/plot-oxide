use crate::app::PlotOxide;
use crate::state::{PlotMode, LineStyle};
use chrono::{DateTime, TimeZone, Utc};
use egui_plot::{Bar, BarChart, BoxElem, BoxPlot, BoxSpread, HLine, Line, Plot, Points};

/// Render the main plot area
pub fn render_plot(app: &mut PlotOxide, ctx: &eframe::egui::Context, ui: &mut eframe::egui::Ui) {
        // Get data from DataSource
        let data = app.data();
        let headers = app.headers();

        // Pre-calculate statistics for outlier filtering (performance optimization)
        if app.state.filters.filter_outliers {
            app.state.outlier_stats_cache.clear();
            for &y_idx in &app.state.view.y_indices {
                let y_values: Vec<f64> = data.iter().map(|row| row[y_idx]).collect();
                let stats = PlotOxide::calculate_statistics(&y_values);
                app.state.outlier_stats_cache.insert(y_idx, stats);
            }
        }

        // Create data for all series with filtering
        let all_series: Vec<Vec<[f64; 2]>> = app.state.view.y_indices.iter()
            .map(|&y_idx| {
                let points: Vec<[f64; 2]> = data.iter()
                    .enumerate()
                    .filter_map(|(row_idx, row)| {
                        let x_val = if app.state.view.use_row_index {
                            row_idx as f64
                        } else {
                            row[app.state.view.x_index]
                        };
                        let y_val = row[y_idx];

                        // Apply filters
                        if app.passes_filters(row_idx, x_val, y_val, y_idx) {
                            Some([x_val, y_val])
                        } else {
                            None
                        }
                    })
                    .collect();

                // Downsample if dataset is large
                if points.len() > app.state.view.downsample_threshold {
                    PlotOxide::downsample_lttb(&points, app.state.view.downsample_threshold)
                } else {
                    points
                }
            })
            .collect();

        // Detect modifier keys for constrained zoom
        let shift_held = ctx.input(|i| i.modifiers.shift);
        let ctrl_held = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);
        let alt_held = ctx.input(|i| i.modifiers.alt);

        // Calculate plot height with minimum constraints to ensure axis labels render properly
        let available_height = ui.available_height();
        let plot_height = if available_height > 600.0 {
            // Cap maximum height to prevent axis label rendering issues
            available_height.min(800.0)
        } else {
            // Use available height but ensure minimum for proper axis rendering
            available_height.max(200.0)
        };

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
                let dt = DateTime::<Utc>::from_timestamp(mark.value as i64, 0);
                if let Some(dt) = dt {
                    // Use space instead of newline to prevent layout issues
                    dt.format("%Y-%m-%d %H:%M").to_string()
                } else {
                    format!("{:.2}", mark.value)
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

        let plot_response = plot.show(ui, |plot_ui| {
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

                        // Get Y values directly from data (not from all_series which has X,Y pairs)
                        let y_values: Vec<f64> = data.iter()
                            .map(|row| row[y_idx])
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

                        // Get Y values directly from data
                        let y_values: Vec<f64> = data.iter()
                            .map(|row| row[y_idx])
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

                        // Get Y values directly from data
                        let y_values: Vec<f64> = data.iter()
                            .map(|row| row[y_idx])
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

                        // Get Y values (number of defects per sample)
                        let defects: Vec<f64> = data.iter()
                            .map(|row| row[y_idx])
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
                let y_values: Vec<f64> = all_series[series_idx].iter().map(|p| p[1]).collect();

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

                plot_response.response.on_hover_ui(|ui| {
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
    }

