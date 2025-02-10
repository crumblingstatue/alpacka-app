mod ui;

use {
    crate::config::Config,
    alpacka::Pkg,
    egui_colors::{Colorix, tokens::ThemeColor},
    smol_str::SmolStr,
    ui::UiState,
};

pub struct AlpackaApp {
    pac: PacState,
    ui: UiState,
    cfg: Config,
    pac_recv: std::sync::mpsc::Receiver<anyhow::Result<PacState>>,
}

struct SyncDb {
    name: SmolStr,
    pkgs: Vec<Pkg>,
}

#[derive(Default)]
struct PacState {
    alpaca_local_pkg_list: Vec<alpacka::Pkg>,
    alpacka_filt_pkg_list: Vec<usize>,
    alpacka_syncdbs: Vec<SyncDb>,
    alpacka_filt_remote_pkg_list: Vec<(SmolStr, usize)>,
}

impl PacState {
    fn new_spawned() -> std::sync::mpsc::Receiver<anyhow::Result<Self>> {
        let (send, recv) = std::sync::mpsc::channel();
        std::thread::spawn(move || send.send(Self::new()));
        recv
    }
    fn new() -> anyhow::Result<Self> {
        let mut local_db = alpacka::read_local_db()?;
        local_db.sort_by(|a, b| a.desc.name.cmp(&b.desc.name));
        let mut syncdbs = Vec::new();
        for db_name in [
            "core-testing",
            "core",
            "extra-testing",
            "extra",
            "multilib-testing",
            "multilib",
        ] {
            let mut pkgs = alpacka::read_syncdb(db_name)?;
            pkgs.sort_by(|a, b| a.desc.name.cmp(&b.desc.name));
            syncdbs.push(SyncDb {
                name: db_name.into(),
                pkgs,
            });
        }
        Ok(Self {
            alpacka_filt_pkg_list: (0..local_db.len()).collect(),
            alpaca_local_pkg_list: local_db,
            alpacka_filt_remote_pkg_list: {
                let mut vec = Vec::new();
                let mut i = 0;
                for db in &syncdbs {
                    for _ in 0..db.pkgs.len() {
                        vec.push((db.name.clone(), i));
                        i += 1;
                    }
                }
                vec
            },
            alpacka_syncdbs: syncdbs,
        })
    }
}

impl AlpackaApp {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            pac: PacState::default(),
            ui: UiState::default(),
            cfg: Config::load_or_default(),
            pac_recv: PacState::new_spawned(),
        })
    }
    pub fn sync_from_config(&mut self, egui_ctx: &eframe::egui::Context) {
        if let Some(color_theme) = &self.cfg.color_theme {
            self.ui.shared.colorix = Some(Colorix::global(
                egui_ctx,
                color_theme.map(ThemeColor::Custom),
            ))
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
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.sync_to_config();
        if let Err(e) = self.cfg.save() {
            eprintln!("Failed to save config: {e}");
        }
    }
}
