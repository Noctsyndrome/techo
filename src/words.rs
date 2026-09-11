use chrono::{Datelike, NaiveDate};

/// Short public-domain lines that rotate daily. A `words.txt` in the journal
/// directory (one line per entry) replaces this list.
const BUILTIN: [&str; 40] = [
    "Each day is a little life. — Schopenhauer",
    "The sun is new each day. — Heraclitus",
    "Well begun is half done. — Aristotle",
    "One today is worth two tomorrows. — Franklin",
    "Little strokes fell great oaks. — Franklin",
    "Well done is better than well said. — Franklin",
    "Lost time is never found again. — Franklin",
    "Adopt the pace of nature: her secret is patience. — Emerson",
    "Finish each day and be done with it. — Emerson",
    "Nothing great was ever achieved without enthusiasm. — Emerson",
    "Write it on your heart that every day is the best day in the year. — Emerson",
    "It is not enough to be busy. The question is: what are we busy about? — Thoreau",
    "Go confidently in the direction of your dreams. — Thoreau",
    "Forever is composed of nows. — Dickinson",
    "I dwell in possibility. — Dickinson",
    "Hope is the thing with feathers. — Dickinson",
    "Happiness, not in another place but this place, not for another hour but this hour. — Whitman",
    "Keep your face toward the sunshine, and shadows fall behind you. — Whitman",
    "To see a world in a grain of sand. — Blake",
    "The mountains are calling and I must go. — Muir",
    "In every walk with nature one receives far more than he seeks. — Muir",
    "Do what you can, with what you have, where you are. — T. Roosevelt",
    "Rest is not idleness. — Lubbock",
    "Let us cultivate our garden. — Voltaire",
    "Confine yourself to the present. — Marcus Aurelius",
    "Begin at once to live, and count each day as a separate life. — Seneca",
    "It is not that we have a short time to live, but that we waste much of it. — Seneca",
    "Life is short, art long. — Hippocrates",
    "Know thyself. — Delphic maxim",
    "No one steps in the same river twice. — Heraclitus",
    "The journey of a thousand miles begins with a single step. — Laozi",
    "Wherever you go, go with all your heart. — Confucius",
    "It does not matter how slowly you go, as long as you do not stop. — Confucius",
    "Everything has beauty, but not everyone sees it. — Confucius",
    "Fall seven times, stand up eight. — Japanese proverb",
    "Little by little, one travels far. — Spanish proverb",
    "Sit quietly, doing nothing; spring comes, and the grass grows by itself. — Zen saying",
    "Patience is bitter, but its fruit is sweet. — Rousseau",
    "All that we see or seem is but a dream within a dream. — Poe",
    "Tomorrow is a new day. — proverb",
];

pub fn for_date(custom: &[String], date: NaiveDate) -> String {
    let index = date.ordinal0() as usize + date.year().unsigned_abs() as usize * 7;
    if custom.is_empty() {
        BUILTIN[index % BUILTIN.len()].to_string()
    } else {
        custom[index % custom.len()].clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotates_daily_and_prefers_custom_lines() {
        let a = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        let b = NaiveDate::from_ymd_opt(2026, 9, 12).unwrap();
        assert_ne!(for_date(&[], a), for_date(&[], b));
        assert_eq!(for_date(&[], a), for_date(&[], a));
        let custom = vec!["one".to_string(), "two".to_string()];
        assert!(custom.contains(&for_date(&custom, a)));
        assert!(BUILTIN.iter().all(|line| line.chars().count() <= 96));
    }
}
