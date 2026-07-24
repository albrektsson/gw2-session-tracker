/// Formats a rounded value with thousands separators, e.g. `1234567.0` ->
/// `"1,234,567"`, `-4200.0` -> `"-4,200"`.
pub fn format_thousands(value: f64) -> String {
    let rounded = value.round() as i64;
    let sign = if rounded < 0 { "-" } else { "" };
    let digits = rounded.unsigned_abs().to_string();

    let mut grouped = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }

    format!("{sign}{}", grouped.chars().rev().collect::<String>())
}

/// Formats a raw copper amount (the unit GW2's API reports the "Coin"
/// wallet currency in) as `"Xg Ys Zc"`, e.g. `4722524.0` -> `"472g 25s
/// 24c"`. 100 copper = 1 silver, 100 silver = 1 gold (so 10000 copper =
/// 1 gold). The gold portion gets thousands separators for large values;
/// silver/copper are always exactly two digits' worth (0-99).
pub fn format_coin(total_copper: f64) -> String {
    let rounded = total_copper.round() as i64;
    let sign = if rounded < 0 { "-" } else { "" };
    let abs = rounded.unsigned_abs();
    let gold = abs / 10_000;
    let silver = (abs % 10_000) / 100;
    let copper = abs % 100;
    format!("{sign}{}g {silver}s {copper}c", format_thousands(gold as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_zero() {
        assert_eq!(format_thousands(0.0), "0");
    }

    #[test]
    fn formats_small_value_without_separator() {
        assert_eq!(format_thousands(999.0), "999");
    }

    #[test]
    fn formats_exact_thousand() {
        assert_eq!(format_thousands(1000.0), "1,000");
    }

    #[test]
    fn formats_large_multi_group_value() {
        assert_eq!(format_thousands(1234567.0), "1,234,567");
    }

    #[test]
    fn formats_negative_value() {
        assert_eq!(format_thousands(-4200.0), "-4,200");
    }

    #[test]
    fn rounds_before_grouping() {
        assert_eq!(format_thousands(1999.6), "2,000");
    }

    #[test]
    fn format_coin_zero() {
        assert_eq!(format_coin(0.0), "0g 0s 0c");
    }

    #[test]
    fn format_coin_copper_only() {
        assert_eq!(format_coin(33.0), "0g 0s 33c");
    }

    #[test]
    fn format_coin_silver_and_copper() {
        assert_eq!(format_coin(233.0), "0g 2s 33c");
    }

    #[test]
    fn format_coin_full_breakdown() {
        // 472g 25s 24c = 472*10000 + 25*100 + 24
        assert_eq!(format_coin(4_722_524.0), "472g 25s 24c");
    }

    #[test]
    fn format_coin_large_gold_gets_thousands_separator() {
        assert_eq!(format_coin(123_456_789.0), "12,345g 67s 89c");
    }

    #[test]
    fn format_coin_negative_value() {
        // a session that net-spent more than it earned
        assert_eq!(format_coin(-233.0), "-0g 2s 33c");
    }
}
