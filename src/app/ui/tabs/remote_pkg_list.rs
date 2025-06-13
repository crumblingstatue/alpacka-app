use {
    super::{PkgListQuery, PkgListState, local_pkg_list::pkg_list_table_builder},
    crate::{
        app::{
            PkgCache,
            ui::{SharedUiState, cmd::Cmd},
        },
        packages::{Dbs, PkgIdx, PkgRef},
    },
    alpacka::{Pkg, PkgDesc},
    eframe::egui,
};

pub fn ui(
    ui: &mut egui::Ui,
    pkgs: &mut PkgCache,
    dbs: &Dbs,
    ui_state: &mut SharedUiState,
    tab_state: &mut PkgListState,
) {
    egui::TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
        top_panel_ui(pkgs, dbs, tab_state, ui);
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
            let list = &pkgs.filt_remote_pkgs;
            body.rows(22.0, list.len(), |mut row| {
                let Some(pkg_ref) = list.get(row.index()) else {
                    row.col(|ui| {
                        ui.label("<Unresolved pkgref>");
                    });
                    return;
                };
                let (db, pkg) = dbs.resolve(*pkg_ref);
                let Some(db) = db else {
                    row.col(|ui| {
                        ui.label("<Unresolved db>");
                    });
                    return;
                };
                let Some(pkg) = pkg else {
                    row.col(|ui| {
                        ui.label("<Unresolved package>");
                    });
                    return;
                };
                row.col(|ui| {
                    ui.horizontal(|ui| {
                        let db_name = &db.name;
                        if ui.link(format!("{db_name}/{}", pkg.desc.name)).clicked() {
                            ui_state.cmd.push(Cmd::OpenPkgTab(*pkg_ref));
                        }
                        installed_label_for_remote_pkg(ui, ui_state, &pkg.desc, dbs);
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

fn top_panel_ui(pkgs: &mut PkgCache, dbs: &Dbs, tab_state: &mut PkgListState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let re = ui.add(egui::TextEdit::singleline(&mut tab_state.query_src).hint_text("🔍 Query"));
        if ui.input(|inp| inp.key_pressed(egui::Key::Num2) && inp.modifiers.shift) {
            re.request_focus();
        }
        if re.changed() {
            tab_state.query = PkgListQuery::compile(&tab_state.query_src);
            pkgs.filt_remote_pkgs = dbs
                .remotes()
                .flat_map(|(db_idx, syncdb)| {
                    syncdb
                        .pkgs
                        .iter()
                        .enumerate()
                        .map(move |(idx, pkg)| (db_idx, idx, pkg))
                })
                .filter_map(|(db, idx, pkg)| {
                    let filt_lo = tab_state.query.string.to_ascii_lowercase();
                    let mut flags = tab_state.query.flags;
                    if (flags.installed || flags.newer || flags.older)
                        && let Some((_, cmp)) = remote_local_cmp(&pkg.desc, dbs.local_pkgs())
                    {
                        flags.installed = false;
                        match cmp {
                            RemoteLocalCmp::Newer => flags.newer = false,
                            RemoteLocalCmp::Same => {}
                            RemoteLocalCmp::Older => flags.older = false,
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
                    .then_some(PkgRef::from_components(db, PkgIdx::from_usize(idx)))
                })
                .collect();
        }
        ui.spacing();
        ui.label(format!("{} packages listed", pkgs.filt_remote_pkgs.len()));
    });
    ui.add_space(4.0);
}

pub fn remote_local_cmp(
    remote: &PkgDesc,
    local_pkg_list: &[Pkg],
) -> Option<(PkgIdx, RemoteLocalCmp)> {
    local_pkg_list
        .iter()
        .enumerate()
        .find(|(_idx, pkg2)| pkg2.desc.name == remote.name)
        .map(|(local_idx, local_pkg)| {
            let cmp = pkg_ver_cmp(remote, local_pkg);
            (PkgIdx::from_usize(local_idx), cmp)
        })
}

pub fn pkg_ver_cmp(remote: &PkgDesc, local_pkg: &Pkg) -> RemoteLocalCmp {
    match crate::vercmp::vercmp(&remote.version, &local_pkg.desc.version) {
        crate::vercmp::AbCmp::ANewer => RemoteLocalCmp::Newer,
        crate::vercmp::AbCmp::Same => RemoteLocalCmp::Same,
        crate::vercmp::AbCmp::BNewer => RemoteLocalCmp::Older,
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum RemoteLocalCmp {
    /// Remote is newer
    Newer,
    /// They are the same version
    Same,
    /// Remote is older
    Older,
}

impl RemoteLocalCmp {
    pub const fn is_newer(&self) -> bool {
        matches!(self, Self::Newer)
    }
}

pub fn installed_label_for_remote_pkg(
    ui: &mut egui::Ui,
    ui_state: &mut SharedUiState,
    remote: &PkgDesc,
    dbs: &Dbs,
) {
    if let Some((local_idx, cmp)) = remote_local_cmp(remote, dbs.local_pkgs()) {
        let Some(local_pkg) = dbs.resolve_local(local_idx) else {
            ui.label("[unresolved]");
            return;
        };
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
            ui_state.cmd.push(Cmd::OpenPkgTab(PkgRef::local(local_idx)));
        }
    }
}
