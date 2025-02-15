use {
    super::{PkgListQuery, PkgListState, local_pkg_list::pkg_list_table_builder},
    crate::{
        app::{
            PacState,
            ui::{SharedUiState, cmd::Cmd},
        },
        util::PkgId,
    },
    alpacka::{Pkg, PkgDesc},
    eframe::egui,
};

pub fn ui(
    ui: &mut egui::Ui,
    pac: &mut PacState,
    ui_state: &mut SharedUiState,
    tab_state: &mut PkgListState,
) {
    egui::TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
        top_panel_ui(pac, tab_state, ui);
    });
    pkg_list_table_builder(ui)
        .header(18.0, |mut row| {
            row.col(|ui| {
                ui.label("Name");
            });
            row.col(|ui| {
                ui.label("Version");
            });
            row.col(|ui| {
                ui.label("Description");
            });
        })
        .body(|mut body| {
            body.ui_mut().style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            let list = &pac.alpacka_filt_remote_pkg_list;
            body.rows(22.0, list.len(), |mut row| {
                let (db_name, idx) = &list[row.index()];
                let Some(db) = &pac.alpacka_syncdbs.iter().find(|db| &db.name == db_name) else {
                    row.col(|ui| {
                        ui.label(format!("<Error: can't find db '{db_name}'>"));
                    });
                    return;
                };
                let Some(pkg) = db.pkgs.get(*idx) else {
                    row.col(|ui| {
                        ui.label(format!("<Error: invalid index: {idx}>"));
                    });
                    return;
                };
                row.col(|ui| {
                    ui.horizontal(|ui| {
                        if ui.link(format!("{db_name}/{}", pkg.desc.name)).clicked() {
                            ui_state.cmd.push(Cmd::OpenPkgTab(PkgId::qualified(
                                db_name,
                                pkg.desc.name.as_str(),
                            )));
                        }
                        installed_label_for_remote_pkg(
                            ui,
                            ui_state,
                            &pkg.desc,
                            &pac.alpaca_local_pkg_list,
                        );
                    });
                });
                row.col(|ui| {
                    ui.label(pkg.desc.version.as_str());
                });
                row.col(|ui| {
                    ui.label(pkg.desc.desc.as_deref().unwrap_or("<missing description>"));
                });
            });
        });
}

fn top_panel_ui(pac: &mut PacState, tab_state: &mut PkgListState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let re = ui.add(egui::TextEdit::singleline(&mut tab_state.query_src).hint_text("🔍 Query"));
        if ui.input(|inp| inp.key_pressed(egui::Key::Num2) && inp.modifiers.shift) {
            re.request_focus();
        }
        if re.changed() {
            tab_state.query = PkgListQuery::compile(&tab_state.query_src);
            pac.alpacka_filt_remote_pkg_list = pac
                .alpacka_syncdbs
                .iter()
                .flat_map(|syncdb| {
                    syncdb
                        .pkgs
                        .iter()
                        .enumerate()
                        .map(|(idx, pkg)| (syncdb.name.clone(), idx, pkg))
                })
                .filter_map(|(db, idx, pkg)| {
                    let filt_lo = tab_state.query.string.to_ascii_lowercase();
                    let mut flags = tab_state.query.flags;
                    if flags.installed || flags.newer || flags.older {
                        if let Some((_, cmp)) =
                            remote_local_cmp(&pkg.desc, &pac.alpaca_local_pkg_list)
                        {
                            flags.installed = false;
                            match cmp {
                                RemoteLocalCmp::Newer => flags.newer = false,
                                RemoteLocalCmp::Same => {}
                                RemoteLocalCmp::Older => flags.older = false,
                            }
                        }
                    }
                    if flags.any() {
                        return None;
                    }
                    (pkg.desc.name.contains(&filt_lo)
                        || pkg
                            .desc
                            .desc
                            .as_ref()
                            .is_some_and(|desc| desc.to_ascii_lowercase().contains(&filt_lo)))
                    .then_some((db, idx))
                })
                .collect();
        }
        ui.spacing();
        ui.label(format!(
            "{} packages listed",
            pac.alpacka_filt_remote_pkg_list.len()
        ));
    });
    ui.add_space(4.0);
}

pub fn remote_local_cmp<'p>(
    remote: &PkgDesc,
    local_pkg_list: &'p [Pkg],
) -> Option<(&'p Pkg, RemoteLocalCmp)> {
    local_pkg_list
        .iter()
        .find(|pkg2| pkg2.desc.name == remote.name)
        .map(|local_pkg| {
            let cmp = match remote.version.cmp(&local_pkg.desc.version) {
                std::cmp::Ordering::Less => RemoteLocalCmp::Older,
                std::cmp::Ordering::Equal => RemoteLocalCmp::Same,
                std::cmp::Ordering::Greater => RemoteLocalCmp::Newer,
            };
            (local_pkg, cmp)
        })
}

pub enum RemoteLocalCmp {
    Newer,
    Same,
    Older,
}

pub fn installed_label_for_remote_pkg(
    ui: &mut egui::Ui,
    ui_state: &mut SharedUiState,
    remote: &PkgDesc,
    local_pkg_list: &[Pkg],
) {
    if let Some((local_pkg, cmp)) = remote_local_cmp(remote, local_pkg_list) {
        let re = match cmp {
            RemoteLocalCmp::Older => ui
                .add(
                    egui::Label::new({
                        egui::RichText::new("[older]").color(egui::Color32::ORANGE)
                    })
                    .sense(egui::Sense::click()),
                )
                .on_hover_ui(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("This package is older than the locally installed");
                        ui.label(
                            egui::RichText::new(local_pkg.desc.name.as_str())
                                .color(egui::Color32::YELLOW),
                        );
                        ui.label(
                            egui::RichText::new(local_pkg.desc.version.as_str().to_string())
                                .color(egui::Color32::ORANGE),
                        );
                    });
                }),
            RemoteLocalCmp::Same => {
                ui.add(egui::Label::new("[installed]").sense(egui::Sense::click()))
            }
            RemoteLocalCmp::Newer => ui
                .add(
                    egui::Label::new(egui::RichText::new("[newer]").color(egui::Color32::YELLOW))
                        .sense(egui::Sense::click()),
                )
                .on_hover_ui(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("This package is newer than the locally installed");
                        ui.label(
                            egui::RichText::new(local_pkg.desc.name.as_str())
                                .color(egui::Color32::YELLOW),
                        );
                        ui.label(
                            egui::RichText::new(local_pkg.desc.version.as_str())
                                .color(egui::Color32::ORANGE),
                        );
                    });
                }),
        };
        if re.hovered() {
            ui.output_mut(|out| out.cursor_icon = egui::CursorIcon::PointingHand);
        }
        if re.clicked() {
            ui_state
                .cmd
                .push(Cmd::OpenPkgTab(PkgId::local(local_pkg.desc.name.as_str())));
        }
    }
}
