#![forbid(unsafe_code)]
#![warn(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::use_self,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::suboptimal_flops,
    unused_qualifications
)]
// These lints actively lead me to writing buggy code
#![allow(clippy::collapsible_if, clippy::collapsible_else_if)]

use {
    crate::config::Config,
    app::AlpackaApp,
    eframe::{NativeOptions, egui::ViewportCommand},
};

mod app;
mod config;
mod packages;
mod query_syntax;
mod util;
mod vercmp;

fn main() {
    if let Err(e) = egui_logger::builder().init() {
        eprintln!("Fatal error. Failed to initialize logger: {e}");
        return;
    }
    if let Err(e) = eframe::run_native(
        "alpacka",
        NativeOptions::default(),
        Box::new(move |cc| {
            let cfg = Config::load_or_default();
            cc.egui_ctx.set_zoom_factor(cfg.zoom_factor);
            let mut app = AlpackaApp::new(cfg);
            cc.egui_ctx
                .send_viewport_cmd(ViewportCommand::Maximized(true));
            cc.egui_ctx.add_font_provider(std::sync::Arc::new(
                egui_system_font_frog::SystemFontProvider::new(),
            ));
            app.sync_from_config(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    ) {
        eprintln!("Fatal error: {e}");
    }
}
