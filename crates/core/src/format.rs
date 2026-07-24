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
}
