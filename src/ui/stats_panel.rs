use crate::app::PlotOxide;
use crate::state::CachedStats;

const HISTOGRAM_BINS: usize = 20;

/// Calculate comprehensive statistics for a column
fn calculate_full_stats(values: &[f64]) -> CachedStats {
    if values.is_empty() {
        return CachedStats::default();
    }
    
    // Filter out NaN values
    let mut clean: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if clean.is_empty() {
        return CachedStats::default();
    }
    
    let count = clean.len();
    let sum: f64 = clean.iter().sum();
    let mean = sum / count as f64;
    
    // Sort for percentiles and median
    clean.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let min = clean[0];
    let max = clean[count - 1];
    
    // Percentiles using linear interpolation
    let percentile = |p: f64| -> f64 {
        let idx = p * (count - 1) as f64;
        let lo = idx.floor() as usize;
        let hi = (lo + 1).min(count - 1);
        let frac = idx - lo as f64;
        clean[lo] * (1.0 - frac) + clean[hi] * frac
    };
    
    let median = percentile(0.5);
    let p5 = percentile(0.05);
    let p25 = percentile(0.25);
    let p75 = percentile(0.75);
    let p95 = percentile(0.95);
    
    // Standard deviation
    let variance: f64 = clean.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count as f64;
    let std_dev = variance.sqrt();
    
    // Histogram for sparkline
    let range = max - min;
    let mut histogram = vec![0u32; HISTOGRAM_BINS];
    
    if range > 0.0 {
        let bin_width = range / HISTOGRAM_BINS as f64;
        for &v in &clean {
            let bin = ((v - min) / bin_width).floor() as usize;
            let bin = bin.min(HISTOGRAM_BINS - 1);
            histogram[bin] += 1;
        }
    } else {
        // All values are the same
        histogram[HISTOGRAM_BINS / 2] = count as u32;
    }
    
    let histogram_max = *histogram.iter().max().unwrap_or(&1);
    
    CachedStats {
        count,
        min,
        max,
        mean,
        median,
        std_dev,
        p5,
        p25,
        p75,
        p95,
        histogram,
        histogram_max,
    }
}

/// Draw a sparkline histogram
fn draw_sparkline(ui: &mut eframe::egui::Ui, stats: &CachedStats, color: eframe::egui::Color32) {
    let desired_size = eframe::egui::vec2(ui.available_width().min(200.0), 24.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, eframe::egui::Sense::hover());
    
    if stats.histogram.is_empty() || stats.histogram_max == 0 {
        return;
    }
    
    let painter = ui.painter();
    let bin_width = rect.width() / stats.histogram.len() as f32;
    let max_height = rect.height() - 2.0;
    
    // Background
    painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
    
    // Bars
    for (i, &count) in stats.histogram.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let height = (count as f32 / stats.histogram_max as f32) * max_height;
        let bar_rect = eframe::egui::Rect::from_min_size(
            eframe::egui::pos2(rect.left() + i as f32 * bin_width, rect.bottom() - height - 1.0),
            eframe::egui::vec2(bin_width - 1.0, height),
        );
        painter.rect_filled(bar_rect, 0.0, color.gamma_multiply(0.7));
    }
}

/// Render the statistics summary panel
pub fn render_stats_panel(app: &mut PlotOxide, ui: &mut eframe::egui::Ui) {
    // P0 FIX: Use DataSource directly instead of app.data() which does expensive row-major conversion
    let ds = match &app.state.data {
        Some(ds) => ds,
        None => {
            ui.label("No data loaded.");
            return;
        }
    };
    
    let headers = app.headers();
    let data_version = app.state.ui.data_version;
    
    // Process each selected Y series
    for (series_idx, &y_idx) in app.state.view.y_indices.iter().enumerate() {
        let color = PlotOxide::get_series_color(series_idx);
        let name = &headers[y_idx];
        
        // Check cache first
        let stats = if let Some(cached) = app.state.ui.get_cached_stats(y_idx) {
            cached.clone()
        } else {
            // Compute stats from column data (column-major, no row conversion)
            let y_values = match ds.get_cached_column(y_idx) {
                Ok(col) => col.to_vec(),
                Err(_) => continue,
            };
            
            let computed = calculate_full_stats(&y_values);
            app.state.ui.cache_stats(y_idx, computed.clone());
            computed
        };
        
        if stats.count == 0 {
            continue;
        }
        
        ui.group(|ui| {
            ui.set_min_width(260.0);
            
            // Header with name and copy button
            ui.horizontal(|ui| {
                ui.colored_label(color, format!("● {}", name));
                ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::Center), |ui| {
                    if ui.small_button("📋").on_hover_text("Copy stats").clicked() {
                        let text = format!(
                            "{}\nCount: {}\nMin: {:.4}\nMax: {:.4}\nMean: {:.4}\nMedian: {:.4}\nStd Dev: {:.4}\nP5: {:.4}\nP25: {:.4}\nP75: {:.4}\nP95: {:.4}",
                            name, stats.count, stats.min, stats.max, stats.mean, stats.median, stats.std_dev,
                            stats.p5, stats.p25, stats.p75, stats.p95
                        );
                        ui.ctx().copy_text(text);
                    }
                });
            });
            
            // Sparkline histogram
            draw_sparkline(ui, &stats, color);
            
            // Count and range
            ui.horizontal(|ui| {
                ui.label(format!("n={}", stats.count));
                ui.separator();
                ui.label(format!("Range: {:.4}", stats.max - stats.min));
            });
            
            // Min/Max
            ui.horizontal(|ui| {
                ui.label(format!("Min: {:.4}", stats.min));
                ui.separator();
                ui.label(format!("Max: {:.4}", stats.max));
            });
            
            // Mean, Median, StdDev
            ui.horizontal(|ui| {
                ui.label(format!("μ={:.4}", stats.mean));
                ui.separator();
                ui.label(format!("Med={:.4}", stats.median));
                ui.separator();
                ui.label(format!("σ={:.4}", stats.std_dev));
            });
            
            // Percentiles
            ui.horizontal(|ui| {
                ui.small(format!("P5={:.2}", stats.p5));
                ui.small(format!("P25={:.2}", stats.p25));
                ui.small(format!("P75={:.2}", stats.p75));
                ui.small(format!("P95={:.2}", stats.p95));
            });
            
            // Process capability if SPC enabled
            if app.state.spc.show_capability {
                let (cp, cpk) = PlotOxide::calculate_process_capability(
                    &[stats.mean], // Use cached mean
                    app.state.spc.spec_lower,
                    app.state.spc.spec_upper,
                );
                
                // Recalculate with proper values
                let cp = (app.state.spc.spec_upper - app.state.spc.spec_lower) / (6.0 * stats.std_dev);
                let cpu = (app.state.spc.spec_upper - stats.mean) / (3.0 * stats.std_dev);
                let cpl = (stats.mean - app.state.spc.spec_lower) / (3.0 * stats.std_dev);
                let cpk = cpu.min(cpl);
                
                ui.horizontal(|ui| {
                    let cp_color = if cp >= 1.33 { 
                        eframe::egui::Color32::GREEN 
                    } else if cp >= 1.0 { 
                        eframe::egui::Color32::YELLOW 
                    } else { 
                        eframe::egui::Color32::RED 
                    };
                    let cpk_color = if cpk >= 1.33 { 
                        eframe::egui::Color32::GREEN 
                    } else if cpk >= 1.0 { 
                        eframe::egui::Color32::YELLOW 
                    } else { 
                        eframe::egui::Color32::RED 
                    };
                    
                    ui.label("Cp:");
                    ui.colored_label(cp_color, format!("{:.3}", cp));
                    ui.separator();
                    ui.label("Cpk:");
                    ui.colored_label(cpk_color, format!("{:.3}", cpk));
                });
                
                // Percentage within spec
                // We need the actual values for this, so compute from cached column
                if let Some(ds) = &app.state.data {
                    if let Ok(col) = ds.get_cached_column(y_idx) {
                        let in_spec = col.iter()
                            .filter(|&&v| v.is_finite() && v >= app.state.spc.spec_lower && v <= app.state.spc.spec_upper)
                            .count();
                        let pct = (in_spec as f64 / stats.count as f64) * 100.0;
                        ui.label(format!("Within spec: {:.1}% ({}/{})", pct, in_spec, stats.count));
                    }
                }
            }
        });
        
        ui.add_space(4.0);
    }
}
