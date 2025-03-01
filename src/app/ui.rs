use {
    super::{AlpackaApp, Packages},
    cmd::CmdBuf,
    eframe::egui,
    egui_colors::Colorix,
    egui_dock::{DockArea, DockState},
    std::{
        process::{Command, ExitStatus},
        sync::mpsc::TryRecvError,
    },
    tabs::{Tab, TabViewState},
};

pub mod cmd;
mod paint_util;
mod tabs;

pub(super) struct UiState {
    dock_state: DockState<Tab>,
    pub shared: SharedUiState,
}

#[derive(Default)]
pub struct SharedUiState {
    cmd: CmdBuf,
    pub colorix: Option<Colorix>,
    pac_handler: Option<PacChildHandler>,
    pub error_popup: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            shared: SharedUiState::default(),
            dock_state: DockState::new(Tab::default_tabs()),
        }
    }
}

pub struct PacChildHandler {
    recv: Option<proc_chan::EventRecv>,
    exit_status: Option<ExitStatus>,
    out_buf: String,
}

impl PacChildHandler {
    pub fn new(recv: proc_chan::EventRecv) -> Self {
        Self {
            recv: Some(recv),
            exit_status: None,
            out_buf: String::new(),
        }
    }
    pub fn handle_messages(&mut self) -> anyhow::Result<()> {
        if let Some(recv) = self.recv.as_mut() {
            match recv.try_recv() {
                Ok(ev) => match ev {
                    proc_chan::Event::StdoutRead(result) => {
                        self.out_buf.push_str(std::str::from_utf8(&result?)?);
                    }
                    proc_chan::Event::StderrRead(result) => {
                        eprintln!("TODO: pacman stderr: {result:?}");
                    }
                    proc_chan::Event::Exit(exit_status) => {
                        self.exit_status = Some(exit_status?);
                        self.recv = None;
                    }
                },
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.recv = None;
                }
            }
        }
        Ok(())
    }
}

pub fn top_panel_ui(app: &mut AlpackaApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel")
        .exact_height(26.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (re, painter) =
                    ui.allocate_painter(egui::vec2(24.0, 24.0), egui::Sense::hover());
                paint_util::draw_logo(&painter, re.rect.center(), 8.0);
                ui.label("Alpacka");
                ui.separator();
                ui.menu_button("⟳ Sync", |ui| {
                    if ui.button("🔁 Sync databases (pacman -Sy)").clicked() {
                        ui.close_menu();
                        if let Err(e) = spawn_pacman_sy(&mut app.ui.shared.pac_handler) {
                            app.ui.shared.error_popup = Some(e.to_string());
                        }
                    }
                    if ui.button("⟳ Refresh package list").clicked() {
                        ui.close_menu();
                        app.pac_recv = Packages::new_spawned();
                    }
                });
                ui.menu_button("☰ Preferences", |ui| {
                    if ui.button("🎨 Color theme").clicked() {
                        ui.close_menu();
                        app.ui.dock_state.push_to_first_leaf(Tab::ColorTheme);
                    }
                    match crate::config::cfg_dir() {
                        Some(dir) => {
                            if ui.button("Open config dir").clicked() {
                                ui.close_menu();
                                let _ = Command::new("xdg-open").arg(dir).status();
                            }
                        }
                        None => {
                            ui.label("<missing config dir>");
                        }
                    }
                });
                ui.menu_button("❓ Debug", |ui| {
                    if ui.button("Error popup test").clicked() {
                        ui.close_menu();
                        app.ui.shared.error_popup = Some("This is a test error popup".into());
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    match app.pac_recv.try_recv() {
                        Ok(pac) => match pac {
                            Ok(pac) => app.pac = pac,
                            Err(e) => {
                                eprintln!("Failed to load pacma dbs: {e}");
                            }
                        },
                        Err(e) => match e {
                            TryRecvError::Empty => {
                                ui.spinner();
                                ui.label("Loading pacman dbs...");
                            }
                            TryRecvError::Disconnected => {}
                        },
                    }
                    if app.ui.shared.pac_handler.is_some() {
                        ui.spinner();
                        ui.label("running pacman...");
                    }
                });
            });
        });
}

pub fn central_panel_ui(app: &mut AlpackaApp, ctx: &egui::Context) {
    DockArea::new(&mut app.ui.dock_state)
        .show_leaf_collapse_buttons(false)
        .show_leaf_close_all_buttons(false)
        .show(
            ctx,
            &mut TabViewState {
                pac: &mut app.pac,
                ui: &mut app.ui.shared,
            },
        );
}

fn spawn_pacman_sy(pac_handler: &mut Option<PacChildHandler>) -> anyhow::Result<()> {
    let mut command = Command::new("pkexec");
    command.args(["pacman", "-Sy"]);
    let recv = proc_chan::spawn(command, None)?;
    *pac_handler = Some(PacChildHandler::new(recv));
    Ok(())
}

pub fn modals(app: &mut AlpackaApp, ctx: &egui::Context) {
    let mut close_handler = false;
    if let Some(handler) = &mut app.ui.shared.pac_handler {
        if let Err(e) = handler.handle_messages() {
            eprintln!("pacman handler message error: {e}");
        }
        if !handler.out_buf.is_empty() {
            egui::Modal::new(egui::Id::new("pacman output modal")).show(ctx, |ui| {
                ui.heading("Pacman output");
                ui.separator();
                let avail_rect = ui.ctx().available_rect();
                let w = (avail_rect.width() * 0.5).round();
                ui.set_width(w);
                egui::ScrollArea::both()
                    .max_height((avail_rect.height() * 0.5).round())
                    .max_width(w)
                    .show(ui, |ui| {
                        ui.set_width(1000.0);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.add(
                            egui::TextEdit::multiline(&mut handler.out_buf.as_str())
                                .code_editor()
                                .desired_width(f32::INFINITY),
                        );
                    });
                ui.separator();
                if let Some(status) = &handler.exit_status {
                    ui.label(format!("Pacman exited ({status})"));
                    if ui.button("Close").clicked() {
                        close_handler = true;
                        app.pac_recv = Packages::new_spawned();
                    }
                }
            });
        }
    }
    if close_handler {
        app.ui.shared.pac_handler = None;
    }
    if let Some(err) = &app.ui.shared.error_popup {
        let mut close = false;
        egui::Modal::new("error_modal".into()).show(ctx, |ui| {
            ui.heading("Error");
            ui.separator();
            ui.label(err);
            ui.separator();
            if ui.button("Close").clicked() {
                close = true;
            }
        });
        if close {
            app.ui.shared.error_popup = None;
        }
    }
}
