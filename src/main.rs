#![forbid(unsafe_code)]
#![feature(let_chains, array_windows, default_field_values)]
#![warn(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::use_self,
    unused_qualifications
)]

use {app::AlpackaApp, eframe::NativeOptions};

mod app;
mod config;
mod packages;
mod query_syntax;
mod util;

fn main() {
    let mut app = AlpackaApp::new();
    if let Err(e) = eframe::run_native(
        "alpacka",
        NativeOptions::default(),
        Box::new(move |cc| {
            app.sync_from_config(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    ) {
        eprintln!("Fatal error: {e}");
    }
}
