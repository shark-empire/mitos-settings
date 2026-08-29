use std::fs;

#[derive(Debug, Clone)]
pub struct SoundCard {
    pub index: String,
    pub description: String,
}

pub fn list_cards() -> Vec<SoundCard> {
    let Ok(content) = fs::read_to_string("/proc/asound/cards") else { return Vec::new() };
    let mut cards = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        let Some(first) = trimmed.chars().next() else { continue };
        if !first.is_ascii_digit() {
            continue; // continuation lines (indented) describe the previous card
        }
        let Some((idx, rest)) = trimmed.split_once(' ') else { continue };
        let description = rest
            .trim()
            .trim_start_matches('[')
            .split(']')
            .last()
            .unwrap_or(rest)
            .trim_start_matches(':')
            .trim()
            .to_string();
        cards.push(SoundCard { index: idx.to_string(), description });
    }
    cards
}
