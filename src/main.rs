#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::App;
use eframe::egui::{self, CentralPanel, SidePanel};

// Module declarations
mod app;
mod constants;
mod data;
mod error;
mod perf;
mod state;
mod widgets;
mod ui;

// Use PlotOxide from app module
use app::PlotOxide;
use state::ActivePanel;
use perf::WorkerResult;

impl App for PlotOxide {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Phase 0: Profiling instrumentation (enable with --features profile-with-puffin or profile-with-tracy)
        profiling::scope!("update");
        profiling::finish_frame!();
        
        // Phase 5: Poll background worker for completed work
        while let Some(result) = self.state.worker.poll() {
            match result {
                WorkerResult::FileLoaded { path, df } => {
                    // Convert DataFrame to DataSource
                    if let Ok(ds) = crate::data::DataSource::from_dataframe(df, Some(path.clone())) {
                        self.state.data = Some(ds);
                        self.state.current_file = Some(path);
                        self.state.lttb_cache.invalidate();
                        self.state.downsampler.force_settle();
                        
                        // Auto-select columns
                        if self.state.column_count() > 1 {
                            self.state.view.y_indices = vec![1];
                        }
                    }
                    self.state.is_loading = false;
                }
                WorkerResult::LttbReady { .. } => {
                    // Cache is updated by the cache itself
                }
                WorkerResult::Error { msg } => {
                    self.state.ui.set_error(msg);
                    self.state.is_loading = false;
                }
            }
        }
        
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
            // P for profiler
            if i.key_pressed(egui::Key::P) && i.modifiers.ctrl {
                self.state.show_profiler = !self.state.show_profiler;
            }
        });

        // 1. Slim Icon Strip (Far Left)
        SidePanel::left("icon_strip")
            .resizable(false)
            .exact_width(50.0)
            .show(ctx, |ui| {
                // Top Icons (Primary Panels)
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space(8.0);

                    // Helper closure to create consistent toggle buttons
                    let mut toggle_btn = |icon: &str, panel: ActivePanel, tooltip: &str| {
                        let is_active = self.state.ui.active_panel == panel;
                        let btn = egui::Button::new(egui::RichText::new(icon).size(20.0))
                            .frame(false)
                            .min_size(egui::vec2(36.0, 36.0))
                            .selected(is_active);
                        
                        if ui.add(btn).on_hover_text(tooltip).clicked() {
                            self.state.ui.toggle_panel(panel);
                        }
                    };

                    // Primary Tool Icons
                    toggle_btn("📂", ActivePanel::Controls, "Controls & Files");
                    toggle_btn("📈", ActivePanel::Series, "Series Selection");
                    toggle_btn("📋", ActivePanel::Table, "Data Table");
                    toggle_btn("∑", ActivePanel::Stats, "Statistics");

                    ui.add_space(32.0);

                    // Reset View Button
                    let reset_btn = egui::Button::new(egui::RichText::new("🔄").size(20.0))
                        .frame(false)
                        .min_size(egui::vec2(36.0, 36.0));
                    if ui.add(reset_btn).on_hover_text("Reset View (R)").clicked() {
                        self.reset_view();
                    }
                    
                    // Grid Toggle
                    let grid_btn = egui::Button::new(egui::RichText::new("⊞").size(20.0))
                        .frame(false)
                        .min_size(egui::vec2(36.0, 36.0))
                        .selected(self.state.view.show_grid);
                    if ui.add(grid_btn).on_hover_text("Grid (G)").clicked() {
                        self.state.view.show_grid = !self.state.view.show_grid;
                    }

                    // Legend Toggle
                    let legend_btn = egui::Button::new(egui::RichText::new("🏷").size(20.0))
                        .frame(false)
                        .min_size(egui::vec2(36.0, 36.0))
                        .selected(self.state.view.show_legend);
                    if ui.add(legend_btn).on_hover_text("Legend (L)").clicked() {
                        self.state.view.show_legend = !self.state.view.show_legend;
                    }

                    // Zoom Toggle
                    let zoom_btn = egui::Button::new(egui::RichText::new("🔍").size(20.0))
                        .frame(false)
                        .min_size(egui::vec2(36.0, 36.0))
                        .selected(self.state.view.allow_zoom);
                    if ui.add(zoom_btn).on_hover_text("Zoom").clicked() {
                        self.state.view.allow_zoom = !self.state.view.allow_zoom;
                    }

                    // Pan Toggle
                    let pan_btn = egui::Button::new(egui::RichText::new("✋").size(20.0))
                        .frame(false)
                        .min_size(egui::vec2(36.0, 36.0))
                        .selected(self.state.view.allow_drag);
                    if ui.add(pan_btn).on_hover_text("Pan").clicked() {
                        self.state.view.allow_drag = !self.state.view.allow_drag;
                    }

                    ui.add_space(32.0);

                    // Theme Toggle
                    let theme_icon = if self.state.view.dark_mode { "🌙" } else { "☀" };
                    let theme_btn = egui::Button::new(egui::RichText::new(theme_icon).size(20.0))
                        .frame(false)
                        .min_size(egui::vec2(36.0, 36.0));
                    if ui.add(theme_btn).on_hover_text("Toggle Theme").clicked() {
                        self.state.view.dark_mode = !self.state.view.dark_mode;
                    }

                    // Help Toggle
                    let help_btn = egui::Button::new(egui::RichText::new("❓").size(20.0))
                        .frame(false)
                        .min_size(egui::vec2(36.0, 36.0));
                    if ui.add(help_btn).on_hover_text("Help (F1)").clicked() {
                        self.state.view.show_help = !self.state.view.show_help;
                    }
                });
            });

        // 2. Conditional "Focus Panel" (Next to icon strip)
        if self.state.ui.active_panel != ActivePanel::None {
            SidePanel::left("focus_panel")
                .default_width(300.0)
                .width_range(200.0..=800.0)
                .resizable(true)
                .show(ctx, |ui| {
                    // Header with title and close button
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let title = match self.state.ui.active_panel {
                            ActivePanel::Controls => "⚙ Controls",
                            ActivePanel::Series => "📈 Y-Series",
                            ActivePanel::Table => "📋 Data Table",
                            ActivePanel::Stats => "∑ Statistics",
                            ActivePanel::None => "",
                        };
                        ui.heading(title);
                        
                        // Close button aligned to right
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close_btn = egui::Button::new(egui::RichText::new("✖").size(16.0).strong())
                                .frame(false);
                            if ui.add(close_btn).on_hover_text("Close Panel").clicked() {
                                self.state.ui.active_panel = ActivePanel::None;
                            }
                        });
                    });
                    ui.separator();

                    // Render content based on active panel
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        match self.state.ui.active_panel {
                            ActivePanel::Controls => {
                                // Re-use existing toolbar logic
                                // Note: Horizontal layouts inside here might wrap, which is desired behavior
                                ui::render_toolbar_and_controls(self, ctx, ui);
                            },
                            ActivePanel::Series => {
                                if self.state.has_data() {
                                    ui::render_series_panel(self, ctx, ui);
                                } else {
                                    ui.label("No data loaded.");
                                }
                            },
                            ActivePanel::Table => {
                                if self.state.has_data() {
                                    ui::render_data_table_panel(self, ui);
                                } else {
                                    ui.label("No data loaded.");
                                }
                            },
                            ActivePanel::Stats => {
                                if self.state.has_data() {
                                    ui::render_stats_panel(self, ui);
                                } else {
                                    ui.label("No data loaded.");
                                }
                            },
                            ActivePanel::None => {}
                        }
                    });
                });
        }

        // 3. Central Plot Area (Fills remaining space)
        CentralPanel::default().show(ctx, |ui| {
            if self.state.has_data() {
                // Main Plot
                if !self.state.view.y_indices.is_empty() {
                    ui::render_plot(self, ctx, ui);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Select a Y-Series from the 📈 panel to begin.");
                    });
                }
            } else {
                // Welcome Screen
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Welcome to PlotOxide");
                        ui.add_space(10.0);
                        ui.label("A high-performance data visualization tool.");
                        ui.add_space(20.0);
                        if ui.button("📂 Open Data File").clicked() {
                            // Trigger file dialog
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Data Files", &["csv", "parquet"])
                                .pick_file()
                            {
                                if let Err(e) = self.load_file(path) {
                                    self.state.ui.set_error(e.user_message());
                                }
                            }
                        }
                        ui.add_space(10.0);
                        ui.label("or drag and drop a file here");
                    });
                });
            }
            
            // // File status overlay (Bottom Right)
            // if let Some(ref file) = self.state.current_file {
            //     let text = format!("📄 {}", file.file_name().unwrap_or_default().to_string_lossy());
            //     let rect = ui.max_rect();
            //     // Simple positioning at bottom right with some padding
            //     let pos = rect.right_bottom() - egui::vec2(10.0, 25.0);
                
            //     // Draw a small background for legibility
            //     let painter = ui.painter();
            //     let galley = ui.painter().layout_no_wrap(
            //         text, 
            //         egui::FontId::proportional(12.0), 
            //         ui.visuals().text_color()
            //     );
                
            //     let text_rect = egui::Rect::from_min_size(
            //         pos - galley.size(), 
            //         galley.size()
            //     ).expand(4.0);
                
            //     painter.rect(
            //         text_rect,
            //         4.0,
            //         ui.visuals().window_fill().gamma_multiply(0.8),
            //         egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
            //         egui::StrokeKind::Middle,
            //     );
                
            //     painter.galley(text_rect.min + egui::vec2(4.0, 4.0), galley, ui.visuals().text_color());
            // }
            
            // Error Toast
            // FIX: Clone the error message to avoid holding an immutable borrow
            let error_message = self.state.ui.error_message.clone();
            if let Some(error) = error_message {
                let rect = ui.max_rect();
                let pos = rect.right_top() + egui::vec2(-10.0, 10.0);
                
                egui::Window::new("Error")
                    .fixed_pos(pos)
                    .anchor(egui::Align2::RIGHT_TOP, [0.0, 0.0])
                    .collapsible(false)
                    .auto_sized()
                    .title_bar(false)
                    .frame(egui::Frame::popup(ui.style()).fill(egui::Color32::from_rgb(50, 0, 0)))
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("⚠").color(egui::Color32::RED));
                            ui.label(egui::RichText::new(error).color(egui::Color32::WHITE));
                            if ui.button("✖").clicked() {
                                self.state.ui.clear_error();
                            }
                        });
                    });
            }
        });

        // Help dialog (Modal)
        ui::render_help_dialog(self, ctx);
        
        // Handle drag and drop globally
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
             ctx.input(|i| {
                if let Some(dropped) = i.raw.dropped_files.first() {
                    if let Some(path) = &dropped.path {
                        if let Err(e) = self.load_file(path.clone()) {
                            self.state.ui.set_error(e.user_message());
                        }
                    }
                }
            });
        }
    }
}

fn main() {
    // Initialize puffin profiler server when feature is enabled
    #[cfg(feature = "profile-with-puffin")]
    let _puffin_server = {
        let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
        puffin::set_scopes_on(true);
        let server = puffin_http::Server::new(&server_addr).expect("Failed to start puffin server");
        eprintln!("Puffin profiler server running on {}", server_addr);
        eprintln!("Connect with: puffin_viewer {}", server_addr);
        server
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    
    eframe::run_native(
        "PlotOxide - Advanced Data Plotter",
        options,
        Box::new(|_| Ok(Box::new(PlotOxide::default()))),
    )
    .unwrap();
}