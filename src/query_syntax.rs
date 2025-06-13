#[derive(Default, PartialEq, Debug)]
pub struct PkgListQuery {
    /// Filter out packages that don't satisfy these properties
    pub flags: QueryFlags,
    /// If non-empty, filter out packages that don't contain this string
    pub string: String,
}

impl PkgListQuery {
    pub fn compile(src: &str) -> Self {
        let mut head = src;
        let mut flags = QueryFlags::default();
        while let Some(at) = head.find('@') {
            let end = head.find(' ').unwrap_or(head.len());
            let Some(token) = head.get(at..end) else {
                break;
            };
            match token {
                "@installed" => flags.installed = true,
                "@older" => flags.older = true,
                "@newer" => flags.newer = true,
                "@asexplicit" | "@explicit" => flags.explicitly_installed = true,
                _ => break,
            }
            let next = std::cmp::min(end + 1, head.len());
            match head.get(next..) {
                Some(next) => head = next,
                None => break,
            }
        }
        Self {
            flags,
            string: head.to_owned(),
        }
    }
}

#[expect(clippy::struct_excessive_bools)]
#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct QueryFlags {
    pub installed: bool,
    pub newer: bool,
    pub older: bool,
    pub explicitly_installed: bool,
}

impl QueryFlags {
    pub const fn any(self) -> bool {
        self.installed || self.newer || self.older
    }
}

#[test]
fn test_compile() {
    assert_eq!(
        PkgListQuery::compile("@installed cool"),
        PkgListQuery {
            flags: QueryFlags {
                installed: true,
                newer: false,
                older: false,
                explicitly_installed: false,
            },
            string: "cool".into(),
        }
    );
    assert_eq!(
        PkgListQuery::compile("hello world"),
        PkgListQuery {
            flags: QueryFlags {
                installed: false,
                newer: false,
                older: false,
                explicitly_installed: false,
            },
            string: "hello world".into(),
        }
    );
    assert_eq!(
        PkgListQuery::compile("@"),
        PkgListQuery {
            flags: QueryFlags {
                installed: false,
                newer: false,
                older: false,
                explicitly_installed: false,
            },
            string: "@".into(),
        }
    );
}
