use freedesktop_desktop_entry::get_languages_from_env;
use nucleo_matcher::{
    Config, Matcher,
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
};

use crate::{applications::DesktopEntryScanner, history::History};

const MATCH_CONFIDENCE: f32 = 0.80;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchResult {
    pub index: usize,
    pub fuzzy_score: Option<u32>,
    pub frecency: f64,
}

pub fn fuzzy_applications(
    scanner: &DesktopEntryScanner,
    query: &str,
    history: &History,
) -> Vec<SearchResult> {
    let locales = get_languages_from_env();
    if query.trim().is_empty() {
        let mut results = scanner
            .entries()
            .iter()
            .enumerate()
            .map(|(index, entry)| SearchResult {
                index,
                fuzzy_score: None,
                frecency: history.frecency(&entry.appid),
            })
            .collect::<Vec<_>>();
        results.sort_by(|left, right| {
            right
                .frecency
                .total_cmp(&left.frecency)
                .then_with(|| {
                    scanner.entries()[left.index]
                        .name(&locales)
                        .unwrap_or_default()
                        .cmp(
                            &scanner.entries()[right.index]
                                .name(&locales)
                                .unwrap_or_default(),
                        )
                })
                .then_with(|| {
                    scanner.entries()[left.index]
                        .appid
                        .cmp(&scanner.entries()[right.index].appid)
                })
        });
        return results;
    }

    let haystacks = scanner
        .entries()
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let name = entry.name(&locales).unwrap_or_default();
            let comment = entry.comment(&locales).unwrap_or_default();
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

    let matches = pattern.match_list(haystacks.iter().map(String::as_str), &mut matcher);
    let minimum_score = matches
        .first()
        .map(|(_, score)| *score as f32 * MATCH_CONFIDENCE)
        .unwrap_or_default();
    
    let mut results = matches
        .into_iter()
        .filter(|(_, score)| *score as f32 >= minimum_score)
        .filter_map(|(item, score)| {
            let index = item.split_once('\t')?.0.parse::<usize>().ok()?;
            Some(SearchResult {
                index,
                fuzzy_score: Some(score),
                frecency: history.frecency(&scanner.entries()[index].appid),
            })
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| {
        let left_score = left.fuzzy_score.unwrap_or_default() as f64;
        let right_score = right.fuzzy_score.unwrap_or_default() as f64;
        let left_boost = history.decayed_launches(&scanner.entries()[left.index].appid);
        let right_boost = history.decayed_launches(&scanner.entries()[right.index].appid);
        (right_score * (1.0 + right_boost)).total_cmp(&(left_score * (1.0 + left_boost)))
    });
    results
}
