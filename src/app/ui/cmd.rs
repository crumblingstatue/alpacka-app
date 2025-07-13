use {
    super::{Tab, tabs::package::PkgTab},
    crate::{
        app::{AlpackaApp, ui::spawn_pacman_cmd_root_pkexec},
        packages::PkgRef,
    },
    eframe::egui,
    egui_dock::{LeafNode, Node, NodeIndex, TabIndex},
};

#[derive(Default)]
pub struct CmdBuf {
    cmds: Vec<Cmd>,
}

impl CmdBuf {
    pub fn push(&mut self, cmd: Cmd) {
        self.cmds.push(cmd);
    }
}

pub enum Cmd {
    OpenPkgTab(PkgRef),
    Rscn(smol_str::SmolStr),
    AsDep(smol_str::SmolStr),
    AsExplicit(smol_str::SmolStr),
}

pub fn process_cmds(app: &mut AlpackaApp, _ctx: &egui::Context) {
    for cmd in std::mem::take(&mut app.ui.shared.cmd.cmds) {
        match cmd {
            Cmd::OpenPkgTab(id) => {
                // First, try to activate already existing tab for this package
                let mut focus_indices = None;
                for (node_idx, (surf_idx, node)) in
                    app.ui.dock_state.iter_all_nodes_mut().enumerate()
                {
                    if let Node::Leaf(LeafNode { tabs, active, .. }) = node {
                        for (tab_idx, tab) in tabs.iter_mut().enumerate() {
                            if let Tab::Pkg(pkg_tab) = tab
                                && pkg_tab.id == id
                            {
                                focus_indices = Some((surf_idx, NodeIndex(node_idx)));
                                *active = TabIndex(tab_idx);
                            }
                        }
                    }
                }
                // Determine where to place the new tab.
                //
                // For now, we just push to the last leaf node, and hope that's good enough.
                #[expect(clippy::collapsible_else_if)]
                if let Some(indices) = focus_indices {
                    app.ui.dock_state.set_focused_node_and_surface(indices);
                } else {
                    if let Some(Node::Leaf(LeafNode { tabs, active, .. })) = app
                        .ui
                        .dock_state
                        .main_surface_mut()
                        .iter_mut()
                        .rfind(|node| node.is_leaf())
                    {
                        tabs.push(Tab::Pkg(PkgTab::new(id)));
                        *active = TabIndex(tabs.len().saturating_sub(1));
                    } else {
                        app.ui
                            .dock_state
                            .push_to_first_leaf(Tab::Pkg(PkgTab::new(id)));
                    }
                }
            }
            Cmd::Rscn(pkg_name) => {
                if let Err(e) = spawn_pacman_cmd_root_pkexec(
                    &mut app.ui.shared.pac_handler,
                    &["-Rscn", pkg_name.as_str()],
                ) {
                    app.ui.shared.error_popup = Some(e.to_string());
                }
            }
            Cmd::AsDep(pkg_name) => {
                if let Err(e) = spawn_pacman_cmd_root_pkexec(
                    &mut app.ui.shared.pac_handler,
                    &["-D", pkg_name.as_str(), "--asdeps"],
                ) {
                    app.ui.shared.error_popup = Some(e.to_string());
                }
            }
            Cmd::AsExplicit(pkg_name) => {
                if let Err(e) = spawn_pacman_cmd_root_pkexec(
                    &mut app.ui.shared.pac_handler,
                    &["-D", pkg_name.as_str(), "--asexplicit"],
                ) {
                    app.ui.shared.error_popup = Some(e.to_string());
                }
            }
        }
    }
}
