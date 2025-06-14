#![forbid(unsafe_code)]
#![feature(array_windows, default_field_values)]
#![warn(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::use_self,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::suboptimal_flops,
    clippy::missing_const_for_fn,
    unused_qualifications
)]

use {
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
    let mut app = AlpackaApp::new();
    if let Err(e) = eframe::run_native(
        "alpacka",
        NativeOptions::default(),
        Box::new(move |cc| {
            cc.egui_ctx
                .send_viewport_cmd(ViewportCommand::Maximized(true));
            app.sync_from_config(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    ) {
        eprintln!("Fatal error: {e}");
    }
}
