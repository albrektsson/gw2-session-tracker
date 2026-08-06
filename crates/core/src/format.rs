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

/// Formats a ratio stat like KDR to 2 decimal places, e.g. `0.5` -> `"0.50"`.
/// Unlike `format_thousands`, this does not round away the fractional part
/// that a rounding-to-whole-number would collapse (e.g. 1 kill / 2 deaths
/// rounding to a misleading "1").
pub fn format_ratio(value: f64) -> String {
    format!("{value:.2}")
}

/// Formats a raw copper amount (the unit GW2's API reports the "Coin"
/// wallet currency in) against `pattern` (`Config::coin_format`), e.g.
/// pattern `"{g}g {s}s {c}c"` on `4722524.0` -> `"472g 25s 24c"`. 100
/// copper = 1 silver, 100 silver = 1 gold (so 10000 copper = 1 gold). `{g}`
/// gets thousands separators for large values; `{s}`/`{c}` are always
/// unpadded (0-99, no leading zero). Anything in `pattern` outside a
/// `{g}`/`{s}`/`{c}` token is copied through literally; the negative-value
/// `-` sign is always an automatic prefix outside the pattern, never a
/// token. Callers should validate `pattern` with `validate_coin_format`
/// first - an unrecognized token here is silently dropped rather than
/// erroring, since by the time this runs (render time), a malformed
/// pattern should already have been rejected at save time.
pub fn format_coin(total_copper: f64, pattern: &str) -> String {
    let rounded = total_copper.round() as i64;
    let sign = if rounded < 0 { "-" } else { "" };
    let abs = rounded.unsigned_abs();
    let gold = abs / 10_000;
    let silver = (abs % 10_000) / 100;
    let copper = abs % 100;

    let mut result = String::with_capacity(pattern.len());
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if c != '{' {
            result.push(c);
            continue;
        }
        let token: String = chars.by_ref().take_while(|&c| c != '}').collect();
        match token.as_str() {
            "g" => result.push_str(&format_thousands(gold as f64)),
            "s" => result.push_str(&silver.to_string()),
            "c" => result.push_str(&copper.to_string()),
            _ => {}
        }
    }
    format!("{sign}{result}")
}

/// Rejects a `Config::coin_format` pattern with unbalanced `{`/`}` braces
/// or a token name other than `g`/`s`/`c`. Used at settings-save time, not
/// at render time - `format_coin` itself stays permissive so a pattern
/// that was valid when saved never suddenly breaks rendering.
pub fn validate_coin_format(pattern: &str) -> Result<(), String> {
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                let mut token = String::new();
                let mut closed = false;
                for tc in chars.by_ref() {
                    if tc == '}' {
                        closed = true;
                        break;
                    }
                    if tc == '{' {
                        return Err("unbalanced braces in coin format pattern".to_string());
                    }
                    token.push(tc);
                }
                if !closed {
                    return Err("unbalanced braces in coin format pattern".to_string());
                }
                if !matches!(token.as_str(), "g" | "s" | "c") {
                    return Err(format!("unknown coin format token \"{{{token}}}\" (expected {{g}}, {{s}}, or {{c}})"));
                }
            }
            '}' => return Err("unbalanced braces in coin format pattern".to_string()),
            _ => {}
        }
    }
    Ok(())
}

/// Formats a duration in seconds as `HH:MM:SS`, e.g. `3725.0` -> `"01:02:05"`.
/// Negative input clamps to zero; hours grow past two digits for long
/// sessions rather than wrapping.
pub fn format_duration(seconds: f64) -> String {
    let total_secs = seconds.max(0.0).round() as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("{hours:02}:{minutes:02}:{secs:02}")
}

/// Formats a meter count as kilometers with 2 decimals, e.g. `1500.0` ->
/// `"1.50 km"`. Negative input clamps to zero.
pub fn format_distance(meters: f64) -> String {
    format!("{:.2} km", meters.max(0.0) / 1000.0)
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
    fn format_ratio_keeps_fractional_part() {
        assert_eq!(format_ratio(0.5), "0.50");
    }

    #[test]
    fn format_ratio_rounds_to_two_decimals() {
        assert_eq!(format_ratio(1.0 / 3.0), "0.33");
    }

    #[test]
    fn formats_negative_value() {
        assert_eq!(format_thousands(-4200.0), "-4,200");
    }

    #[test]
    fn rounds_before_grouping() {
        assert_eq!(format_thousands(1999.6), "2,000");
    }

    const DEFAULT_COIN_FORMAT: &str = "{g}g {s}s {c}c";

    #[test]
    fn format_coin_zero() {
        assert_eq!(format_coin(0.0, DEFAULT_COIN_FORMAT), "0g 0s 0c");
    }

    #[test]
    fn format_coin_copper_only() {
        assert_eq!(format_coin(33.0, DEFAULT_COIN_FORMAT), "0g 0s 33c");
    }

    #[test]
    fn format_coin_silver_and_copper() {
        assert_eq!(format_coin(233.0, DEFAULT_COIN_FORMAT), "0g 2s 33c");
    }

    #[test]
    fn format_coin_full_breakdown() {
        // 472g 25s 24c = 472*10000 + 25*100 + 24
        assert_eq!(format_coin(4_722_524.0, DEFAULT_COIN_FORMAT), "472g 25s 24c");
    }

    #[test]
    fn format_coin_large_gold_gets_thousands_separator() {
        assert_eq!(format_coin(123_456_789.0, DEFAULT_COIN_FORMAT), "12,345g 67s 89c");
    }

    #[test]
    fn format_coin_negative_value() {
        // a session that net-spent more than it earned
        assert_eq!(format_coin(-233.0, DEFAULT_COIN_FORMAT), "-0g 2s 33c");
    }

    #[test]
    fn format_coin_honors_a_custom_pattern() {
        assert_eq!(format_coin(4_722_524.0, "{g}/{s}/{c}"), "472/25/24");
    }

    #[test]
    fn format_coin_gold_only_pattern_drops_silver_and_copper() {
        assert_eq!(format_coin(4_722_524.0, "{g}g"), "472g");
    }

    #[test]
    fn format_coin_negative_sign_stays_an_automatic_prefix_outside_the_pattern() {
        assert_eq!(format_coin(-33.0, "{c}c"), "-33c");
    }

    #[test]
    fn validate_coin_format_accepts_the_default_pattern() {
        assert!(validate_coin_format(DEFAULT_COIN_FORMAT).is_ok());
    }

    #[test]
    fn validate_coin_format_accepts_a_gold_only_pattern() {
        assert!(validate_coin_format("{g}g").is_ok());
    }

    #[test]
    fn validate_coin_format_rejects_an_unclosed_brace() {
        assert!(validate_coin_format("{g").is_err());
    }

    #[test]
    fn validate_coin_format_rejects_a_stray_closing_brace() {
        assert!(validate_coin_format("g}").is_err());
    }

    #[test]
    fn validate_coin_format_rejects_a_nested_opening_brace() {
        assert!(validate_coin_format("{g{s}").is_err());
    }

    #[test]
    fn validate_coin_format_rejects_an_unknown_token() {
        assert!(validate_coin_format("{sign}").is_err());
    }

    #[test]
    fn format_duration_zero() {
        assert_eq!(format_duration(0.0), "00:00:00");
    }

    #[test]
    fn format_duration_sub_hour() {
        assert_eq!(format_duration(125.0), "00:02:05");
    }

    #[test]
    fn format_duration_exact_hour() {
        assert_eq!(format_duration(3600.0), "01:00:00");
    }

    #[test]
    fn format_duration_multi_hour() {
        assert_eq!(format_duration(3725.0), "01:02:05");
    }

    #[test]
    fn format_duration_grows_past_ninety_nine_hours() {
        assert_eq!(format_duration(100.0 * 3600.0), "100:00:00");
    }

    #[test]
    fn format_duration_rounds_before_formatting() {
        assert_eq!(format_duration(59.6), "00:01:00");
    }

    #[test]
    fn format_duration_clamps_negative_to_zero() {
        assert_eq!(format_duration(-5.0), "00:00:00");
    }

    #[test]
    fn format_distance_zero() {
        assert_eq!(format_distance(0.0), "0.00 km");
    }

    #[test]
    fn format_distance_sub_km() {
        assert_eq!(format_distance(500.0), "0.50 km");
    }

    #[test]
    fn format_distance_exact_km() {
        assert_eq!(format_distance(1000.0), "1.00 km");
    }

    #[test]
    fn format_distance_multi_km() {
        assert_eq!(format_distance(12345.0), "12.35 km");
    }

    #[test]
    fn format_distance_rounds_before_formatting() {
        assert_eq!(format_distance(1236.0), "1.24 km");
    }

    #[test]
    fn format_distance_clamps_negative_to_zero() {
        assert_eq!(format_distance(-500.0), "0.00 km");
    }
}
