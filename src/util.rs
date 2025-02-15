use std::path::Path;

/// Filters out items from the package file list that are fully contained by the next item
/// (e.g. `/usr/bin`) is removed if the next item is `/usr/bin/cat`
pub fn deduped_files(list: &[Box<str>]) -> impl Iterator<Item = &str> {
    list.array_windows()
        .filter_map(|[a, b]| {
            let a: &str = a;
            let b: &str = b;
            let retain = !path_contains_other_path(b.as_ref(), a.as_ref());
            (retain).then_some(a)
        })
        .chain(list.last().map(|s| &**s))
}

fn path_contains_other_path(haystack: &Path, needle: &Path) -> bool {
    haystack.parent() == Some(needle)
}
