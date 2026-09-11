use chrono::{Datelike, Duration, NaiveDate};

pub const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub fn shift_days(date: NaiveDate, days: i64) -> NaiveDate {
    date.checked_add_signed(Duration::days(days))
        .filter(|d| (1..=9999).contains(&d.year()))
        .unwrap_or(date)
}

pub fn shift_months(date: NaiveDate, months: i32) -> NaiveDate {
    let total = date.year() * 12 + date.month0() as i32 + months;
    let (year, month) = (total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
    if !(1..=9999).contains(&year) {
        return date;
    }
    (1..=date.day())
        .rev()
        .find_map(|day| NaiveDate::from_ymd_opt(year, month, day))
        .unwrap_or(date)
}

pub fn parse_date(value: &str) -> Option<NaiveDate> {
    let date = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()?;
    (value.trim() == date.format("%Y-%m-%d").to_string() && (1..=9999).contains(&date.year()))
        .then_some(date)
}

/// Mean synodic cycle: decorative daily estimate, not an astronomical ephemeris.
/// Reference new moon 2000-01-06 18:14 UTC and 29.530588 days, NASA:
/// https://eclipse.gsfc.nasa.gov/phase/phases1901.html
/// Sample the selected Gregorian date at 12:00 UTC on every platform.
/// Named in words: terminals cannot be given a moon glyph that renders the same
/// everywhere, so the page says the phase rather than drawing it.
pub fn moon(date: NaiveDate) -> &'static str {
    let reference = NaiveDate::from_ymd_opt(2000, 1, 6).unwrap();
    let days = (date - reference).num_days() as f64 + (12.0 - 18.0 - 14.0 / 60.0) / 24.0;
    let phase = days.rem_euclid(29.530588) / 29.530588;
    let index = ((phase * 8.0 + 0.5).floor() as usize) % 8;
    [
        "New moon",
        "Waxing crescent",
        "First quarter",
        "Waxing gibbous",
        "Full moon",
        "Waning gibbous",
        "Last quarter",
        "Waning crescent",
    ][index]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leap_day_month_and_year_boundaries() {
        let d = parse_date("2024-02-29").unwrap();
        assert_eq!(shift_months(d, 12), parse_date("2025-02-28").unwrap());
        assert_eq!(shift_days(d, 1), parse_date("2024-03-01").unwrap());
        assert_eq!(
            shift_months(parse_date("2026-12-31").unwrap(), 1),
            parse_date("2027-01-31").unwrap()
        );
        assert!(parse_date("2025-02-29").is_none());
        assert!(parse_date("2026-2-01").is_none());
    }
    #[test]
    fn reference_phases_match_nasa_january_2000() {
        for (day, name) in [
            (6, "New moon"),
            (14, "First quarter"),
            (21, "Full moon"),
            (28, "Last quarter"),
        ] {
            assert_eq!(moon(NaiveDate::from_ymd_opt(2000, 1, day).unwrap()), name);
        }
    }

    #[test]
    fn contemporary_phases_match_nasa_september_2026() {
        // https://eclipse.gsfc.nasa.gov/phase/phases2001.html
        for (day, name) in [
            (4, "Last quarter"),
            (11, "New moon"),
            (18, "First quarter"),
            (26, "Full moon"),
        ] {
            assert_eq!(moon(NaiveDate::from_ymd_opt(2026, 9, day).unwrap()), name);
        }
    }
}
