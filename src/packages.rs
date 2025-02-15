use {alpacka::Pkg, smol_str::SmolStr};

/// Used to index into a package list in order to refer to a package efficiently
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Refers to a package that's either in a local or a remote database
///
/// Internally, it uses 8 bits for the db index, and 24 bits for the package index
#[derive(Clone, Copy)]
pub struct PkgRef(u32);

impl PkgRef {
    pub fn from_components(db: SyncDbIdx, pkg: PkgIdx) -> Self {
        let merged = u32::from(db.0) << 24 | pkg.0;
        Self(merged)
    }
    pub fn into_components(self) -> (SyncDbIdx, PkgIdx) {
        (SyncDbIdx((self.0 >> 24) as u8), PkgIdx(self.0 & 0xFF_FFFF))
    }
}

#[test]
fn test_pkg_ref_cons() {
    let pkg_ref = PkgRef::from_components(SyncDbIdx(42), PkgIdx(617));
    assert_eq!(pkg_ref.into_components(), (SyncDbIdx(42), PkgIdx(617)));
}

pub struct Db {
    pub name: SmolStr,
    pub pkgs: Vec<Pkg>,
}

#[derive(Default)]
pub struct Packages {
    pub filt_local_pkgs: Vec<PkgIdx>,
    pub filt_remote_pkgs: Vec<PkgRef>,
    pub dbs: Vec<Db>,
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
        let local_len = local_db.len();
        syncdbs.push(Db {
            name: "local".into(),
            pkgs: local_db,
        });
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
            syncdbs.push(Db {
                name: db_name.into(),
                pkgs,
            });
        }
        Ok(Self {
            filt_local_pkgs: (0..local_len).map(PkgIdx::from_usize).collect(),
            filt_remote_pkgs: {
                let mut vec = Vec::new();
                for (db_idx, db) in syncdbs.iter().enumerate().skip(1) {
                    for i in 0..db.pkgs.len() {
                        vec.push(PkgRef::from_components(
                            SyncDbIdx::from_usize(db_idx),
                            PkgIdx::from_usize(i),
                        ));
                    }
                }
                vec
            },
            dbs: syncdbs,
        })
    }
}
