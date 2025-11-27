use crate::app::PlotOxide;

pub fn render_help_dialog(app: &mut PlotOxide, ctx: &eframe::egui::Context) {
    if app.state.view.show_help {
        eframe::egui::Window::new("⌨ Keyboard Shortcuts")
            .anchor(eframe::egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading("Navigation");
                ui.label("R - Reset view");
                ui.label("G - Toggle grid");
                ui.label("L - Toggle legend");
                ui.label("T - Toggle dark/light theme");
                ui.label("H / F1 - Toggle help");
                ui.label("ESC - Close help");

                ui.separator();
                ui.heading("Mouse Controls");
                ui.label("Scroll - Zoom in/out");
                ui.label("Shift + Scroll - Zoom X-axis only");
                ui.label("Ctrl + Scroll - Zoom Y-axis only");
                ui.label("Drag - Pan view");
                ui.label("Alt + Drag - Box zoom");
                ui.label("Click point - Select point");
                ui.label("Right-click - Context menu");

                ui.separator();
                ui.heading("Series Selection");
                ui.label("Click - Select single");
                ui.label("Ctrl/Cmd + Click - Toggle item");
                ui.label("Shift + Click - Select range");
                ui.label("Ctrl + Shift + Click - Add range");

                ui.separator();
                if ui.button("Close").clicked() {
                    app.state.view.show_help = false;
                }
            });
    }
}
