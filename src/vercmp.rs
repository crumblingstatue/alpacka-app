#[derive(PartialEq, Debug)]
pub enum AbCmp {
    ANewer,
    Same,
    BNewer,
}

#[derive(Debug)]
struct AlpmVer<'s> {
    epoch: &'s str,
    ver: &'s str,
    rel: &'s str,
}

impl<'s> AlpmVer<'s> {
    fn parse(src: &'s str) -> Self {
        let bytes = src.as_bytes();
        let mut epoch = "0";
        let mut ver = src;
        let mut rel = "1";
        let mut epoch_term = 0;
        for b in bytes {
            if b.is_ascii_digit() {
                epoch_term += 1;
            } else {
                break;
            }
        }
        if bytes.get(epoch_term).is_some_and(|&b| b == b':') {
            epoch = &src[..epoch_term];
            ver = &src[epoch_term + 1..];
        }
        if let Some(hy_pos) = src.rfind('-') {
            rel = &src[hy_pos + 1..];
            ver = &src[..hy_pos];
        }
        Self { epoch, ver, rel }
    }
}

pub fn vercmp(a: &str, b: &str) -> AbCmp {
    if a == b {
        return AbCmp::Same;
    }
    let a = AlpmVer::parse(a);
    let b = AlpmVer::parse(b);
    match rpm_vercmp(a.epoch.as_bytes(), b.epoch.as_bytes()) {
        AbCmp::Same => match rpm_vercmp(a.ver.as_bytes(), b.ver.as_bytes()) {
            AbCmp::Same => rpm_vercmp(a.rel.as_bytes(), b.rel.as_bytes()),
            etc => etc,
        },
        etc => etc,
    }
}

/// Same tests as in `test/util/vercmptest.sh` in pacman source tree.
#[test]
fn pacman_vercmp_tests() {
    // all similar length, no pkgrel
    assert_eq!(vercmp("1.5.0", "1.5.0"), AbCmp::Same);
    assert_eq!(vercmp("1.5.1", "1.5.0"), AbCmp::ANewer);

    // mixed length
    assert_eq!(vercmp("1.5.1", "1.5"), AbCmp::ANewer);

    // with pkgrel, simple
    assert_eq!(vercmp("1.5.0-1", "1.5.0-1"), AbCmp::Same);
    assert_eq!(vercmp("1.5.0-1", "1.5.0-2"), AbCmp::BNewer);
    assert_eq!(vercmp("1.5.0-1", "1.5.1-1"), AbCmp::BNewer);
    assert_eq!(vercmp("1.5.0-2", "1.5.1-1"), AbCmp::BNewer);

    // with pkgrel, mixed lengths
    assert_eq!(vercmp("1.5-1", "1.5.1-1"), AbCmp::BNewer);
    assert_eq!(vercmp("1.5-2", "1.5.1-1"), AbCmp::BNewer);
    assert_eq!(vercmp("1.5-2", "1.5.1-2"), AbCmp::BNewer);

    // mixed pkgrel inclusion
    assert_eq!(vercmp("1.5", "1.5-1"), AbCmp::Same);
    assert_eq!(vercmp("1.5-1", "1.5"), AbCmp::Same);
    assert_eq!(vercmp("1.1-1", "1.1"), AbCmp::Same);
    assert_eq!(vercmp("1.0-1", "1.1"), AbCmp::BNewer);
    assert_eq!(vercmp("1.1-1", "1.0"), AbCmp::ANewer);

    // alphanumeric versions
    assert_eq!(vercmp("1.5b-1", "1.5-1"), AbCmp::BNewer);
    assert_eq!(vercmp("1.5b", "1.5"), AbCmp::BNewer);
    assert_eq!(vercmp("1.5b-1", "1.5"), AbCmp::BNewer);
    assert_eq!(vercmp("1.5b", "1.5.1"), AbCmp::BNewer);

    // from the manpage
    assert_eq!(vercmp("1.0a", "1.0alpha"), AbCmp::BNewer);
    assert_eq!(vercmp("1.0alpha", "1.0b"), AbCmp::BNewer);
    assert_eq!(vercmp("1.0b", "1.0beta"), AbCmp::BNewer);
    assert_eq!(vercmp("1.0beta", "1.0rc"), AbCmp::BNewer);
    assert_eq!(vercmp("1.0rc", "1.0"), AbCmp::BNewer);

    // going crazy? alpha-dotted versions
    assert_eq!(vercmp("1.5.a", "1.5"), AbCmp::ANewer);
    assert_eq!(vercmp("1.5.b", "1.5.a"), AbCmp::ANewer);
    assert_eq!(vercmp("1.5.1", "1.5.b"), AbCmp::ANewer);

    // alpha dots and dashes
    assert_eq!(vercmp("1.5.b-1", "1.5.b"), AbCmp::Same);
    assert_eq!(vercmp("1.5-1", "1.5.b"), AbCmp::BNewer);

    // same/similar content, differing separators
    assert_eq!(vercmp("2.0", "2_0"), AbCmp::Same);
    assert_eq!(vercmp("2.0_a", "2_0.a"), AbCmp::Same);
    assert_eq!(vercmp("2.0a", "2.0.a"), AbCmp::BNewer);
    assert_eq!(vercmp("2___a", "2_a"), AbCmp::ANewer);

    // epoch included version comparisons
    assert_eq!(vercmp("0:1.0", "0:1.0"), AbCmp::Same);
    assert_eq!(vercmp("0:1.0", "0:1.1"), AbCmp::BNewer);
    assert_eq!(vercmp("1:1.0", "0:1.0"), AbCmp::ANewer);
    assert_eq!(vercmp("1:1.0", "0:1.1"), AbCmp::ANewer);
    assert_eq!(vercmp("1:1.0", "2:1.1"), AbCmp::BNewer);

    // epoch + sometimes present pkgrel
    assert_eq!(vercmp("1:1.0", "0:1.0-1"), AbCmp::ANewer);
    assert_eq!(vercmp("1:1.0-1", "0:1.1-1"), AbCmp::ANewer);

    // epoch included on one version
    assert_eq!(vercmp("0:1.0", "1.0"), AbCmp::Same);
    assert_eq!(vercmp("0:1.0", "1.1"), AbCmp::BNewer);
    assert_eq!(vercmp("0:1.1", "1.0"), AbCmp::ANewer);
    assert_eq!(vercmp("1:1.0", "1.0"), AbCmp::ANewer);
    assert_eq!(vercmp("1:1.0", "1.1"), AbCmp::ANewer);
    assert_eq!(vercmp("1:1.1", "1.1"), AbCmp::ANewer);
}

#[test]
fn test_rpm_vercmp_odd() {
    assert_eq!(vercmp("", ""), AbCmp::Same);
    assert_eq!(vercmp(" ", ""), AbCmp::ANewer);
    assert_eq!(vercmp("", " "), AbCmp::BNewer);
}

/// RPM-style version comparison for two version segments
fn rpm_vercmp(a: &[u8], b: &[u8]) -> AbCmp {
    if a == b {
        return AbCmp::Same;
    }
    let mut a_cur = 0;
    let mut b_cur = 0;
    while a_cur != a.len() && b_cur != b.len() {
        while !a.get(a_cur).is_some_and(u8::is_ascii_alphanumeric) {
            a_cur += 1;
        }
        while !b.get(b_cur).is_some_and(u8::is_ascii_alphanumeric) {
            b_cur += 1;
        }
        match a_cur.cmp(&b_cur) {
            std::cmp::Ordering::Less => return AbCmp::BNewer,
            std::cmp::Ordering::Greater => return AbCmp::ANewer,
            std::cmp::Ordering::Equal => {}
        }
        let mut a_iter = a_cur;
        let mut b_iter = b_cur;
        let isnum = if a.get(a_iter).is_some_and(u8::is_ascii_digit) {
            while a.get(a_iter).is_some_and(u8::is_ascii_digit) {
                a_iter += 1;
            }
            while b.get(b_iter).is_some_and(u8::is_ascii_digit) {
                b_iter += 1;
            }
            true
        } else {
            while a.get(a_iter).is_some_and(u8::is_ascii_alphabetic) {
                a_iter += 1;
            }
            while b.get(b_iter).is_some_and(u8::is_ascii_alphabetic) {
                b_iter += 1;
            }
            false
        };
        if a_cur == a_iter {
            return AbCmp::BNewer;
        } else if b_cur == b_iter {
            return if isnum { AbCmp::ANewer } else { AbCmp::BNewer };
        }
        if isnum {
            let a_len = a_iter - a_cur;
            let b_len = b_iter - b_cur;
            if a_len > b_len {
                return AbCmp::ANewer;
            } else if b_len > a_len {
                return AbCmp::BNewer;
            }
        }
        match a.get(a_cur..a_iter).cmp(&b.get(b_cur..b_iter)) {
            std::cmp::Ordering::Less => return AbCmp::BNewer,
            std::cmp::Ordering::Greater => return AbCmp::ANewer,
            std::cmp::Ordering::Equal => {
                a_cur = a_iter;
                b_cur = b_iter;
            }
        }
    }

    if a_cur == a.len() && b_cur == b.len() {
        AbCmp::Same
    } else if a_cur == a.len() && !b.get(b_cur).is_some_and(u8::is_ascii_alphabetic)
        || a.get(a_cur).is_some_and(u8::is_ascii_alphabetic)
    {
        AbCmp::BNewer
    } else {
        AbCmp::ANewer
    }
}
