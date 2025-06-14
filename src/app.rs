mod ui;

use {
    crate::{
        config::Config,
        packages::{Dbs, LoadRecv, PkgCache},
    },
    egui_colors::{Colorix, tokens::ThemeColor},
    ui::UiState,
};

pub struct AlpackaApp {
    pkgs: PkgCache,
    dbs: Option<Dbs>,
    ui: UiState,
    cfg: Config,
    load_recv: LoadRecv,
    open_upgrade_window: bool,
}

impl AlpackaApp {
    pub fn new() -> Self {
        Self {
            pkgs: PkgCache::default(),
            dbs: None,
            ui: UiState::default(),
            cfg: Config::load_or_default(),
            load_recv: crate::packages::spawn_load_thread(),
            open_upgrade_window: false,
        }
    }
    pub fn sync_from_config(&mut self, egui_ctx: &eframe::egui::Context) {
        if let Some(color_theme) = &self.cfg.color_theme {
            self.ui.shared.colorix = Some(Colorix::global(
                egui_ctx,
                color_theme.map(ThemeColor::Custom),
            ));
        }
    }
    fn sync_to_config(&mut self) {
        self.cfg.color_theme = self
            .ui
            .shared
            .colorix
            .as_ref()
            .map(|colorix| colorix.theme().map(|theme| theme.rgb()));
    }
}

impl eframe::App for AlpackaApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ui::top_panel_ui(self, ctx);
        ui::central_panel_ui(self, ctx);
        ui::cmd::process_cmds(self, ctx);
        ui::modals(self, ctx);
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.sync_to_config();
        if let Err(e) = self.cfg.save() {
            log::error!("Failed to save config: {e}");
        }
    }
}
