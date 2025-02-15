use {alpacka::Pkg, smol_str::SmolStr};

/// Used to index into a package list in order to refer to a package efficiently
#[derive(Clone, Copy, Debug)]
pub struct PkgIdx(u32);

impl PkgIdx {
    /// Create from an usize index.
    ///
    /// It's expected that the usize doesn't exceed `u32::MAX` (there won't be billions of packages).
    #[expect(clippy::cast_possible_truncation)]
    pub fn from_usize(idx: usize) -> Self {
        Self(idx as u32)
    }
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

/// Used to index into a sync db list
#[derive(Clone, Copy, Debug)]
pub struct SyncDbIdx(u8);

impl SyncDbIdx {
    /// Create from an usize index.
    ///
    /// It's expected that the usize doesn't exceed `u8::MAX` (there won't be hundreds of sync dbs).
    #[expect(clippy::cast_possible_truncation)]
    pub fn from_usize(idx: usize) -> Self {
        Self(idx as u8)
    }
    pub fn to_usize(self) -> usize {
        usize::from(self.0)
    }
}

pub struct SyncDb {
    pub name: SmolStr,
    pub pkgs: Vec<Pkg>,
}

#[derive(Default)]
pub struct Packages {
    pub local_pkg_list: Vec<alpacka::Pkg>,
    pub filt_local_pkgs: Vec<PkgIdx>,
    pub filt_remote_pkgs: Vec<(SyncDbIdx, PkgIdx)>,
    pub syncdbs: Vec<SyncDb>,
}

impl Packages {
    pub fn new_spawned() -> std::sync::mpsc::Receiver<anyhow::Result<Self>> {
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
            filt_local_pkgs: (0..local_db.len()).map(PkgIdx::from_usize).collect(),
            local_pkg_list: local_db,
            filt_remote_pkgs: {
                let mut vec = Vec::new();
                for (db_idx, db) in syncdbs.iter().enumerate() {
                    for i in 0..db.pkgs.len() {
                        vec.push((SyncDbIdx::from_usize(db_idx), PkgIdx::from_usize(i)));
                    }
                }
                vec
            },
            syncdbs,
        })
    }
}
