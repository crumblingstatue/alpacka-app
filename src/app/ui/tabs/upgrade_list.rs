use {
    super::{PkgListState, local_pkg_list::pkg_list_table_builder, remote_pkg_list::pkg_ver_cmp},
    crate::{
        app::ui::{PacChildHandler, SharedUiState, cmd::Cmd},
        packages::{DbIdx, Packages, PkgIdx, PkgRef},
        query_syntax::PkgListQuery,
    },
    ansi_term_buf::Term,
    eframe::egui,
};

#[derive(Default)]
pub(in crate::app::ui) struct State {
    pkg_list: PkgListState,
    pub(in crate::app::ui) force_close: bool,
    pub(in crate::app::ui) just_opened: bool = true,
    upgrade_list: Vec<Upgrade>,
}

struct Upgrade {
    local: PkgIdx,
    remote: PkgRef,
}

pub fn ui(
    ui: &mut egui::Ui,
    pkgs: &mut Packages,
    ui_state: &mut SharedUiState,
    tab_state: &mut State,
) {
    if tab_state.just_opened {
        tab_state.upgrade_list = determine_upgrades(pkgs);
        tab_state.just_opened = false;
    }
    egui::TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            let re = ui.add(
                egui::TextEdit::singleline(&mut tab_state.pkg_list.query_src).hint_text("🔍 Query"),
            );
            if ui.input(|inp| inp.key_pressed(egui::Key::Num2) && inp.modifiers.shift) {
                re.request_focus();
            }
            if re.changed() {
                tab_state.pkg_list.query = PkgListQuery::compile(&tab_state.pkg_list.query_src);
                pkgs.filt_local_pkgs =
                    pkgs.dbs[0]
                        .pkgs
                        .iter()
                        .enumerate()
                        .filter_map(|(i, pkg)| {
                            let filt_lo = tab_state.pkg_list.query.string.to_ascii_lowercase();
                            (pkg.desc.name.contains(&filt_lo)
                                || pkg.desc.desc.as_ref().is_some_and(|desc| {
                                    desc.to_ascii_lowercase().contains(&filt_lo)
                                })
                                || pkg
                                    .desc
                                    .provides
                                    .iter()
                                    .any(|dep| dep.name.contains(&filt_lo)))
                            .then_some(PkgIdx::from_usize(i))
                        })
                        .collect();
            }
            ui.spacing();
            ui.label(format!("{} packages listed", tab_state.upgrade_list.len()));
            if ui
                .add_enabled(
                    ui_state.pac_handler.is_none(),
                    egui::Button::new("pacman -Su"),
                )
                .clicked()
                && let Err(e) = spawn_pacman_su(&mut ui_state.pac_handler)
            {
                ui_state.error_popup = Some(e.to_string());
            }
        });
        ui.add_space(4.0);
    });
    pkg_list_table_builder(ui)
        .header(18.0, |mut row| {
            row.col(|ui| {
                ui.label("Name");
            });
            row.col(|ui| {
                ui.label("Upgrade");
            });
            row.col(|ui| {
                ui.label("Description");
            });
        })
        .body(|mut body| {
            body.ui_mut().style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            body.rows(22.0, tab_state.upgrade_list.len(), |mut row| {
                let upg = &tab_state.upgrade_list[row.index()];
                let idx = upg.local;
                let local = &pkgs.dbs[0].pkgs[idx.to_usize()];
                let (rem_db, rem_pkg) = upg.remote.into_components();
                let remote = &pkgs.dbs[rem_db.to_usize()].pkgs[rem_pkg.to_usize()];
                row.col(|ui| {
                    if ui.link(local.desc.name.as_str()).clicked() {
                        ui_state.cmd.push(Cmd::OpenPkgTab(PkgRef::local(idx)));
                    }
                });
                row.col(|ui| {
                    ui.label(ver_layout_job(local, remote));
                });
                row.col(|ui| {
                    ui.label(
                        local
                            .desc
                            .desc
                            .as_deref()
                            .unwrap_or("<missing description>"),
                    );
                });
            });
        });
}

fn ver_layout_job(local: &alpacka::Pkg, remote: &alpacka::Pkg) -> egui::text::LayoutJob {
    let size = 12.0;
    let diff = difference::Changeset::new(
        local.desc.version.as_str(),
        remote.desc.version.as_str(),
        "",
    );
    let mut lj = egui::text::LayoutJob::default();
    for change in &diff.diffs {
        match change {
            difference::Difference::Same(frag) => lj.append(
                frag,
                0.0,
                egui::TextFormat::simple(
                    egui::FontId::proportional(size),
                    egui::Color32::LIGHT_GRAY,
                ),
            ),
            difference::Difference::Add(_frag) => {}
            difference::Difference::Rem(frag) => lj.append(
                frag,
                0.0,
                egui::TextFormat::simple(egui::FontId::proportional(size), egui::Color32::DARK_RED),
            ),
        }
    }
    lj.append(
        " ➡ ",
        0.0,
        egui::TextFormat::simple(egui::FontId::proportional(size), egui::Color32::WHITE),
    );
    for change in &diff.diffs {
        match change {
            difference::Difference::Same(frag) => lj.append(
                frag,
                0.0,
                egui::TextFormat::simple(
                    egui::FontId::proportional(size),
                    egui::Color32::LIGHT_GRAY,
                ),
            ),
            difference::Difference::Add(frag) => lj.append(
                frag,
                0.0,
                egui::TextFormat::simple(egui::FontId::proportional(size), egui::Color32::GREEN),
            ),
            difference::Difference::Rem(_frag) => {}
        }
    }
    lj
}

fn determine_upgrades(pkgs: &mut Packages) -> Vec<Upgrade> {
    let mut out = Vec::new();
    if let Some((localdb, syncs)) = pkgs.dbs.split_first() {
        for (li, local) in localdb.pkgs.iter().enumerate() {
            for (di, syncdb) in syncs.iter().enumerate() {
                for (ri, remote) in syncdb.pkgs.iter().enumerate() {
                    if local.desc.name == remote.desc.name
                        && pkg_ver_cmp(&remote.desc, local).is_newer()
                    {
                        out.push(Upgrade {
                            local: PkgIdx::from_usize(li),
                            remote: PkgRef::from_components(
                                DbIdx::from_usize(di + 1),
                                PkgIdx::from_usize(ri),
                            ),
                        });
                    }
                }
            }
        }
    }
    out
}

fn spawn_pacman_su(pac_handler: &mut Option<PacChildHandler>) -> anyhow::Result<()> {
    let (pty, the_pts) = pty_process::blocking::open()?;
    let child = pty_process::blocking::Command::new("pkexec")
        .args(["pacman", "-Su"])
        .spawn(the_pts)?;
    *pac_handler = Some(PacChildHandler {
        child,
        pty,
        term: Term::new(100),
        exit_status: None,
        input_buf: String::new(),
    });
    Ok(())
}
