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
pub struct DbIdx(u8);

impl DbIdx {
    /// The local database
    pub const LOCAL: Self = Self(0);
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PkgRef(u32);

impl PkgRef {
    pub fn from_components(db: DbIdx, pkg: PkgIdx) -> Self {
        let merged = u32::from(db.0) << 24 | pkg.0;
        Self(merged)
    }
    pub fn local(pkg: PkgIdx) -> Self {
        Self::from_components(DbIdx::LOCAL, pkg)
    }
    pub fn into_components(self) -> (DbIdx, PkgIdx) {
        (DbIdx((self.0 >> 24) as u8), PkgIdx(self.0 & 0xFF_FFFF))
    }
    pub fn display(self, dbs: &[Db]) -> impl std::fmt::Display {
        struct Disp<'db>(DbIdx, PkgIdx, &'db [Db]);
        impl std::fmt::Display for Disp<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let Some(db) = self.2.get(self.0.to_usize()) else {
                    return writeln!(f, "Unresolved db ({:?})", self.0);
                };
                let db_name = &db.name;
                let Some(pkg) = db.pkgs.get(self.1.to_usize()) else {
                    return writeln!(f, "unresolved package idx ({:?})", self.1);
                };
                let pkg_name = &pkg.desc.name;
                write!(f, "{db_name}/{pkg_name}")
            }
        }
        let (db, pkg) = self.into_components();
        Disp(db, pkg, dbs)
    }
    pub fn is_local(self) -> bool {
        let (db, _) = self.into_components();
        db == DbIdx::LOCAL
    }
    pub fn is_remote(self) -> bool {
        !self.is_local()
    }
}

impl std::fmt::Debug for PkgRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (db_id, pkg_id) = self.into_components();
        write!(f, "PkgRef(db: {db_id:?}, pkg: {pkg_id:?})")
    }
}

#[test]
fn test_pkg_ref_cons() {
    let pkg_ref = PkgRef::from_components(DbIdx(42), PkgIdx(617));
    assert_eq!(pkg_ref.into_components(), (DbIdx(42), PkgIdx(617)));
}

pub struct Db {
    pub name: SmolStr,
    pub pkgs: Vec<Pkg>,
}

#[derive(Default)]
pub struct PkgCache {
    pub filt_local_pkgs: Vec<PkgIdx>,
    pub filt_remote_pkgs: Vec<PkgRef>,
}

pub struct Dbs {
    /// Invariant: dbs[0] is present, and it's the local db
    pub inner: Vec<Db>,
}

impl Dbs {
    pub fn resolve(&self, pkg_ref: PkgRef) -> (Option<&Db>, Option<&Pkg>) {
        let (db_idx, pkg_idx) = pkg_ref.into_components();
        let db = self.inner.get(db_idx.to_usize());
        let pkg = db.and_then(|db| db.pkgs.get(pkg_idx.to_usize()));
        (db, pkg)
    }
    pub fn local_pkgs(&self) -> &[Pkg] {
        // Invariant: self.dbs[0] is the local db
        #[expect(clippy::indexing_slicing)]
        &self.inner[0].pkgs
    }
}

impl PkgCache {
    pub fn new_spawned() -> std::sync::mpsc::Receiver<anyhow::Result<(Self, Dbs)>> {
        let (send, recv) = std::sync::mpsc::channel();
        std::thread::spawn(move || send.send(Self::new()));
        recv
    }
    fn new() -> anyhow::Result<(Self, Dbs)> {
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
        Ok((
            Self {
                filt_local_pkgs: (0..local_len).map(PkgIdx::from_usize).collect(),
                filt_remote_pkgs: {
                    let mut vec = Vec::new();
                    for (db_idx, db) in syncdbs.iter().enumerate().skip(1) {
                        for i in 0..db.pkgs.len() {
                            vec.push(PkgRef::from_components(
                                DbIdx::from_usize(db_idx),
                                PkgIdx::from_usize(i),
                            ));
                        }
                    }
                    vec
                },
            },
            Dbs { inner: syncdbs },
        ))
    }
}
