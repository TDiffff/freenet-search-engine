pub mod app_card;
pub mod app_directory;
pub mod docs_page;
pub mod footer;
pub mod index_view;
pub mod onboarding;
pub mod search_bar;
pub mod search_results;
pub mod settings;

pub fn truncate_key(key: &str, max: usize) -> String {
    if key.len() <= max {
        return key.to_string();
    }
    let half = max / 2;
    let start_end = key
        .char_indices()
        .take_while(|(i, _)| *i < half)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    let tail_start = key
        .char_indices()
        .rev()
        .take_while(|(i, _)| key.len() - *i <= half)
        .last()
        .map(|(i, _)| i)
        .unwrap_or(key.len());
    format!("{}...{}", &key[..start_end], &key[tail_start..])
}
