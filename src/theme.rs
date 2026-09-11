use chrono::{Datelike, NaiveDate};
use ratatui::style::Color;

/// A month's printed colour, the way a planner prints each month's pages in
/// its own soft shade. Names are the traditional Japanese month names; colours
/// are soft, low-saturation traditional Japanese colours: printed rules should
/// sit quietly behind the writing, so nothing deep or loud is used, and
/// neighbouring months may resemble each other a little.
pub struct Month {
    pub name: &'static str,
    pub reading: &'static str,
    pub colour: &'static str,
    pub rgb: (u8, u8, u8),
}

pub const MONTHS: [Month; 12] = [
    Month {
        name: "睦月",
        reading: "mutsuki",
        colour: "一斤染",
        rgb: (245, 177, 170),
    },
    Month {
        name: "如月",
        reading: "kisaragi",
        colour: "勿忘草色",
        rgb: (137, 195, 235),
    },
    Month {
        name: "弥生",
        reading: "yayoi",
        colour: "撫子色",
        rgb: (238, 187, 203),
    },
    Month {
        name: "卯月",
        reading: "uzuki",
        colour: "若葉色",
        rgb: (185, 208, 139),
    },
    Month {
        name: "皐月",
        reading: "satsuki",
        colour: "藤色",
        rgb: (187, 188, 222),
    },
    Month {
        name: "水無月",
        reading: "minazuki",
        colour: "白群",
        rgb: (131, 204, 210),
    },
    Month {
        name: "文月",
        reading: "fumizuki",
        colour: "浅縹",
        rgb: (132, 185, 203),
    },
    Month {
        name: "葉月",
        reading: "hazuki",
        colour: "刈安色",
        rgb: (245, 229, 107),
    },
    Month {
        name: "長月",
        reading: "nagatsuki",
        colour: "半色",
        rgb: (166, 154, 189),
    },
    Month {
        name: "神無月",
        reading: "kannazuki",
        colour: "洗柿",
        rgb: (242, 201, 172),
    },
    Month {
        name: "霜月",
        reading: "shimotsuki",
        colour: "白茶",
        rgb: (221, 187, 153),
    },
    Month {
        name: "師走",
        reading: "shiwasu",
        colour: "柳鼠",
        rgb: (200, 213, 187),
    },
];

pub fn month(date: NaiveDate) -> &'static Month {
    &MONTHS[date.month0() as usize]
}

impl Month {
    pub fn color(&self) -> Color {
        let (r, g, b) = self.rgb;
        Color::Rgb(r, g, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_month_has_a_name_and_a_distinct_colour() {
        let september = month(NaiveDate::from_ymd_opt(2026, 9, 11).unwrap());
        assert_eq!(september.name, "長月");
        assert_eq!(
            month(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()).name,
            "睦月"
        );
        let mut colours: Vec<_> = MONTHS.iter().map(|m| m.rgb).collect();
        colours.sort();
        colours.dedup();
        assert_eq!(colours.len(), 12);
        assert!(
            MONTHS
                .iter()
                .all(|m| !m.reading.is_empty() && !m.colour.is_empty())
        );
    }
}
