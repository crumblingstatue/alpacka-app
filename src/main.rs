#![forbid(unsafe_code)]
#![feature(let_chains, array_windows)]
#![warn(clippy::pedantic, clippy::unwrap_used)]

use {app::AlpackaApp, eframe::NativeOptions};

mod alpm_util;
mod app;
mod config;
mod query_syntax;

fn main() -> anyhow::Result<()> {
    let mut app = AlpackaApp::new()?;
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
    Ok(())
}
