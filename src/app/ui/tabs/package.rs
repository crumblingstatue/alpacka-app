use {
    super::remote_pkg_list::installed_label_for_remote_pkg,
    crate::{
        app::ui::{
            SharedUiState,
            cmd::{Cmd, CmdBuf},
        },
        packages::{DbIdx, Dbs, PkgIdx, PkgRef},
        util::deduped_files,
    },
    alpacka::{InstallReason, Pkg},
    eframe::egui,
    humansize::format_size_i,
    std::process::Command,
};

pub struct PkgTab {
    pub id: PkgRef,
    tab: PkgTabTab,
    pub force_close: bool,
    files_filt_string: String,
    /// Only do local-only dependency resolution
    pub local_only: bool,
}

impl PkgTab {
    pub fn new(id: PkgRef) -> Self {
        Self {
            id,
            tab: PkgTabTab::default(),
            force_close: false,
            files_filt_string: String::new(),
            local_only: true,
        }
    }
}

#[derive(PartialEq, Default)]
enum PkgTabTab {
    #[default]
    General,
    Files,
}

pub fn ui(ui: &mut egui::Ui, dbs: &Dbs, ui_state: &mut SharedUiState, pkg_tab: &mut PkgTab) {
    if ui.input(|inp| {
        let esc = inp.key_pressed(egui::Key::Escape);
        let ctrl_w = inp.modifiers.ctrl && inp.key_pressed(egui::Key::W);
        esc || ctrl_w
    }) {
        pkg_tab.force_close = true;
    }
    pkg_ui(ui, ui_state, pkg_tab, dbs);
}

fn db_name_is_arch(name: &str) -> bool {
    [
        "core",
        "extra",
        "core-testing",
        "extra-testing",
        "multilib",
        "multilib-testing",
    ]
    .into_iter()
    .any(|repo| name == repo)
}

fn pkg_ui(ui: &mut egui::Ui, ui_state: &mut SharedUiState, pkg_tab: &mut PkgTab, dbs: &Dbs) {
    let (db, pkg) = dbs.resolve(pkg_tab.id);
    let Some(db) = db else {
        ui.label("Unresolved database");
        return;
    };
    let Some(pkg) = pkg else {
        ui.label("<Unresolved package>");
        return;
    };
    let db_name = &db.name;
    let remote = pkg_tab.id.is_remote();
    ui.horizontal(|ui| {
        ui.label(format!("{db_name}/"));
        ui.heading(pkg.desc.name.as_str());
        ui.label(pkg.desc.version.as_str());
        if remote {
            installed_label_for_remote_pkg(ui, ui_state, &pkg.desc, dbs);
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.selectable_value(&mut pkg_tab.tab, PkgTabTab::General, "General");
        ui.selectable_value(&mut pkg_tab.tab, PkgTabTab::Files, "File list");
    });
    ui.separator();
    match pkg_tab.tab {
        PkgTabTab::General => general_tab_ui(ui, &mut ui_state.cmd, dbs, pkg, db_name, pkg_tab),
        PkgTabTab::Files => files_tab_ui(ui, ui_state, pkg_tab, pkg),
    }
}

fn files_tab_ui(ui: &mut egui::Ui, ui_state: &mut SharedUiState, pkg_tab: &mut PkgTab, pkg: &Pkg) {
    ui.add(egui::TextEdit::singleline(&mut pkg_tab.files_filt_string).hint_text("🔍 Filter"));
    let files = &pkg.files;
    let deduped_files = deduped_files(files).filter(|file| {
        file.to_ascii_lowercase()
            .contains(&pkg_tab.files_filt_string.to_ascii_lowercase())
    });
    for file in deduped_files {
        let name = format!("/{file}");
        if ui.link(&name).clicked()
            && let Err(e) = Command::new("xdg-open").arg(name).status()
        {
            ui_state.error_popup = Some(e.to_string());
        }
    }
}

fn general_tab_ui(
    ui: &mut egui::Ui,
    cmd: &mut CmdBuf,
    dbs: &Dbs,
    pkg: &Pkg,
    db_name: &str,
    pkg_tab: &mut PkgTab,
) {
    ui.label(pkg.desc.desc.as_deref().unwrap_or("<no description>"));
    if let Some(url) = pkg.desc.url.as_deref() {
        ui.horizontal(|ui| {
            ui.label("Upstream URL");
            ui.hyperlink(url);
        });
    }
    if db_name_is_arch(db_name) {
        ui.horizontal(|ui| {
            ui.label("Arch package URL");
            ui.hyperlink(format!(
                "https://archlinux.org/packages/{db_name}/{}/{}",
                pkg.desc.arch, pkg.desc.name
            ));
        });
        ui.horizontal(|ui| {
            ui.label("Package Source URL");
            ui.hyperlink(format!(
                "https://gitlab.archlinux.org/archlinux/packaging/packages/{}",
                pkg.desc.name
            ));
        });
    }
    ui.label(format!(
        "Installed size: {}",
        format_size_i(pkg.desc.size, humansize::BINARY)
    ));
    match pkg.desc.install_reason {
        InstallReason::Explicit => ui.label("Explicitly installed"),
        InstallReason::Dep => ui.label("Installed as a dependency"),
    };
    deps_ui(ui, cmd, dbs, pkg);
    opt_deps_ui(ui, cmd, dbs.local_pkgs(), pkg);
    required_by_ui(ui, cmd, pkg, dbs, pkg_tab);
    optional_for_ui(ui, cmd, pkg, dbs);
    provides_ui(ui, pkg);
}

fn required_by_ui(ui: &mut egui::Ui, cmd: &mut CmdBuf, pkg: &Pkg, dbs: &Dbs, pkg_tab: &mut PkgTab) {
    let reqs = calc_required_by(pkg, dbs, pkg_tab.local_only);
    ui.horizontal(|ui| {
        ui.heading(format!("Required by ({})", reqs.len()));
        ui.checkbox(&mut pkg_tab.local_only, "Local only");
    });
    if reqs.is_empty() {
        ui.label("<none>");
    } else {
        ui.horizontal_wrapped(|ui| {
            for (ref_, req) in reqs {
                if ui.link(req.desc.name.as_str()).clicked() {
                    cmd.push(Cmd::OpenPkgTab(ref_));
                }
            }
        });
    }
}

fn calc_required_by<'db>(pkg: &Pkg, dbs: &'db Dbs, local_only: bool) -> Vec<(PkgRef, &'db Pkg)> {
    let mut reqs = Vec::new();
    if local_only {
        calc_required_by_inner(pkg, &mut reqs, DbIdx::LOCAL, dbs.local_pkgs());
    } else {
        for (db_i, db) in dbs.all() {
            calc_required_by_inner(pkg, &mut reqs, db_i, &db.pkgs);
        }
    }
    reqs
}

fn calc_required_by_inner<'db>(
    pkg: &Pkg,
    reqs: &mut Vec<(PkgRef, &'db Pkg)>,
    db_i: DbIdx,
    pkgs: &'db [Pkg],
) {
    for (pkg_i, pkg2) in pkgs.iter().enumerate() {
        if alpacka::dep::pkg_matches_dep(&pkg.desc, &pkg2.desc) {
            reqs.push((
                PkgRef::from_components(db_i, PkgIdx::from_usize(pkg_i)),
                pkg2,
            ));
        }
    }
}

fn provides_ui(ui: &mut egui::Ui, pkg: &Pkg) {
    let provides = &pkg.desc.provides;
    ui.heading(format!("Provides ({})", provides.len()));
    for dep in provides {
        ui.label(dep.name.as_str());
    }
}

fn optional_for_ui(ui: &mut egui::Ui, cmd: &mut CmdBuf, pkg: &Pkg, dbs: &Dbs) {
    let opt_for = pkgs_that_optionally_depend_on(pkg, dbs);
    ui.heading(format!("Optional for ({})", opt_for.len()));
    if opt_for.is_empty() {
        ui.label("<none>");
    } else {
        ui.horizontal_wrapped(|ui| {
            for (ref_, pkg) in opt_for {
                if ui.link(pkg.desc.name.as_str()).clicked() {
                    cmd.push(Cmd::OpenPkgTab(ref_));
                }
            }
        });
    }
}

fn pkgs_that_optionally_depend_on<'db>(dependency: &Pkg, dbs: &'db Dbs) -> Vec<(PkgRef, &'db Pkg)> {
    let mut pkgs = Vec::new();
    for (db_i, db) in dbs.all() {
        for (pkg_i, pkg) in db.pkgs.iter().enumerate() {
            if pkg_optionally_depends_on(pkg, dependency) {
                pkgs.push((
                    PkgRef::from_components(db_i, PkgIdx::from_usize(pkg_i)),
                    pkg,
                ));
            }
        }
    }
    pkgs
}

fn pkg_optionally_depends_on(pkg: &Pkg, dependency: &Pkg) -> bool {
    alpacka::dep::pkg_matches_opt_dep(&dependency.desc, &pkg.desc)
}

fn opt_deps_ui(ui: &mut egui::Ui, cmd: &mut CmdBuf, local_list: &[Pkg], pkg: &Pkg) {
    let opt_deps = &pkg.desc.opt_depends;
    ui.heading(format!("Optional dependencies ({})", opt_deps.len()));
    if opt_deps.is_empty() {
        ui.label("<none>");
    } else {
        for opt_dep in opt_deps {
            ui.horizontal(|ui| {
                let installed = local_list.iter().enumerate().find_map(|(i, pkg)| {
                    (pkg.desc.name == opt_dep.dep.name).then_some(PkgIdx::from_usize(i))
                });
                if let Some(ref_) = installed {
                    if ui.link(opt_dep.dep.name.as_str()).clicked() {
                        cmd.push(Cmd::OpenPkgTab(PkgRef::local(ref_)));

                        if let Some(ver) = opt_dep.dep.ver.as_ref().map(|v| v.ver.as_str()) {
                            ui.label(format!("={ver}"));
                        }
                    }
                } else {
                    ui.label(opt_dep.dep.name.as_str());
                }
                if let Some(desc) = &opt_dep.reason {
                    ui.label(desc.as_str());
                }
                if installed.is_some() {
                    ui.label("[installed]");
                }
            });
        }
    }
}

fn deps_ui(ui: &mut egui::Ui, cmd: &mut CmdBuf, dbs: &Dbs, pkg: &Pkg) {
    let deps = &pkg.desc.depends;
    ui.heading(format!("Dependencies ({})", deps.len()));
    if deps.is_empty() {
        ui.label("<none>");
    } else {
        ui.horizontal_wrapped(|ui| {
            for dep in deps {
                match resolve_dep(dep, dbs) {
                    Some((ref_, pkg)) => {
                        let label = if dep.name == pkg.desc.name {
                            dep.name.as_str()
                        } else {
                            &format!("{} ({})", dep.name, pkg.desc.name)
                        };
                        if ui.link(label).clicked() {
                            cmd.push(Cmd::OpenPkgTab(ref_));
                        }
                    }
                    None => {
                        ui.label(format!("{} (unresolved)", dep.name));
                    }
                }
            }
        });
    }
}

fn resolve_dep<'db>(dep: &alpacka::Depend, dbs: &'db Dbs) -> Option<(PkgRef, &'db Pkg)> {
    for (db_i, db) in dbs.all() {
        for (pkg_i, pkg) in db.pkgs.iter().enumerate() {
            if pkg.desc.name == dep.name
                || pkg.desc.provides.iter().any(|dep2| {
                    // TODO: This might not be correct/enough
                    dep2.name == dep.name
                        && dep2.ver.as_ref().map(|v| &v.ver) >= dep.ver.as_ref().map(|v| &v.ver)
                })
            {
                return Some((
                    PkgRef::from_components(db_i, PkgIdx::from_usize(pkg_i)),
                    pkg,
                ));
            }
        }
    }
    None
}
