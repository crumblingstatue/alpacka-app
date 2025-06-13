use {
    super::{AlpackaApp, Packages},
    ansi_term_buf::Term,
    cmd::CmdBuf,
    eframe::egui::{self, TextBuffer},
    egui_colors::Colorix,
    egui_dock::{DockArea, DockState},
    nonblock::NonBlockingReader,
    pty_process::blocking::{Command as PtyCommand, Pty},
    std::{
        io::Write,
        process::{Child, ExitStatus},
        sync::mpsc::TryRecvError,
    },
    tabs::{Tab, TabViewState, upgrade_list},
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
                        if let Err(e) =
                            spawn_pacman_cmd_root_pkexec(&mut app.ui.shared.pac_handler, &["-Sy"])
                        {
                            app.ui.shared.error_popup = Some(e.to_string());
                        } else {
                            app.open_upgrade_window = true;
                        }
                    }
                    if ui.button("Upgrade list").clicked() {
                        ui.close_menu();
                        app.ui
                            .dock_state
                            .push_to_focused_leaf(Tab::UpgradeList(upgrade_list::State::default()));
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
                                let _ = std::process::Command::new("xdg-open").arg(dir).status();
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
                        Ok(result) => match result {
                            Ok(pkgs) => {
                                app.pkgs = Some(pkgs);
                                if app.open_upgrade_window {
                                    app.ui.dock_state.push_to_focused_leaf(Tab::UpgradeList(
                                        upgrade_list::State::default(),
                                    ));
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to load pacman dbs: {e}");
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
                pkgs: app.pkgs.as_mut(),
                ui: &mut app.ui.shared,
            },
        );
}

struct PacChildHandler {
    child: Child,
    pty: Pty,
    term: Term,
    exit_status: Option<ExitStatus>,
    input_buf: String,
}

impl PacChildHandler {
    fn new(child: Child, pty: Pty) -> Self {
        Self {
            child,
            pty,
            term: Term::new(100),
            exit_status: None,
            input_buf: String::new(),
        }
    }
    fn update(&mut self) {
        if self.exit_status.is_some() {
            return;
        }
        let mut buf = Vec::new();
        let mut nbr = match NonBlockingReader::from_fd(&self.pty) {
            Ok(nbr) => nbr,
            Err(e) => {
                eprintln!("Failed to create non-blocking reader: {e}");
                return;
            }
        };
        match nbr.read_available(&mut buf) {
            Ok(n_read) => {
                if n_read != 0 {
                    self.term.feed(&buf);
                }
            }
            Err(e) => {
                eprintln!("error reading from pacman: {e}");
            }
        }
        match self.child.try_wait() {
            Ok(Some(status)) => self.exit_status = Some(status),
            Ok(None) => {}
            Err(e) => {
                eprintln!("Error waiting for pacman: {e}");
            }
        }
    }
}

fn spawn_pacman_cmd_root_pkexec(
    pac_handler: &mut Option<PacChildHandler>,
    args: &[&str],
) -> anyhow::Result<()> {
    let (pty, the_pts) = pty_process::blocking::open()?;
    let child = PtyCommand::new("pkexec")
        .args([["pacman"].as_slice(), args].concat())
        .spawn(the_pts)?;
    *pac_handler = Some(PacChildHandler::new(child, pty));
    Ok(())
}

pub fn modals(app: &mut AlpackaApp, ctx: &egui::Context) {
    let mut close_handler = false;
    if let Some(handler) = &mut app.ui.shared.pac_handler {
        handler.update();
        let out = handler.term.contents_to_string();
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
                        egui::TextEdit::multiline(&mut out.as_str())
                            .code_editor()
                            .desired_width(f32::INFINITY),
                    );
                });
            ui.separator();
            ui.add(
                egui::TextEdit::singleline(&mut handler.input_buf)
                    .hint_text("pacman input")
                    .desired_width(f32::INFINITY),
            );
            if ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                let mut buf = handler.input_buf.take();
                buf.push('\n');
                if let Err(e) = handler.pty.write_all(buf.as_bytes()) {
                    eprintln!("Error writing input: {e}");
                }
            }
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
