use nucleo_matcher::{
    Config, Matcher,
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
};

use crate::applications::{DesktopEntry, DesktopEntryScanner};

pub fn fuzzy_applications<'a>(
    scanner: &'a DesktopEntryScanner,
    query: &str,
) -> Vec<&'a DesktopEntry> {
    if query.trim().is_empty() {
        return scanner.entries().iter().collect();
    }

    let locales: &[&str] = &[];
    let haystacks = scanner
        .entries()
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let name = entry.name(locales).unwrap_or_default();
            let comment = entry.comment(locales).unwrap_or_default();
            format!("{index}\t{name}\t{comment}")
        })
        .collect::<Vec<_>>();
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::new(
        query,
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
    );

    let indexes = pattern
        .match_list(haystacks.iter().map(String::as_str), &mut matcher)
        .into_iter()
        .filter_map(|(item, _score)| item.split_once('\t')?.0.parse().ok())
        .collect::<Vec<usize>>();
    indexes
        .into_iter()
        .filter_map(|index| scanner.entries().get(index))
        .collect()
}
