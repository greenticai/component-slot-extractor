//! Regex-based slot extraction for all five [`SlotType`] variants.
//!
//! Each slot definition is processed independently. A definition either:
//!   * matches → produces an `ExtractedSlot` with `source = "regex"` and is added to `filled`
//!   * falls back to `default_value` → `source = "default"`, `confidence = 0.5`
//!   * is required but unmatched → added to `missing`
//!   * is optional and unmatched → ignored
//!
//! Invalid regex patterns are treated as non-matches (never panic).

use chrono::NaiveDate;
use regex::Regex;
use serde_json::{Value, json};

use crate::{ExtractedSlot, SlotDefinition, SlotExtractionInput, SlotExtractionOutput, SlotType};

const SOURCE_REGEX: &str = "regex";
const SOURCE_DEFAULT: &str = "default";
const SOURCE_I18N: &str = "i18n";

const DEFAULT_NUMBER_PATTERN: &str = r"-?\d+(?:\.\d+)?";

/// Date formats tried in order when `pattern` is not provided. ISO 8601 first
/// (unambiguous), then US (`m/d/Y`), then EU (`d/m/Y`).
const DATE_FORMATS: &[&str] = &["%Y-%m-%d", "%m/%d/%Y", "%d/%m/%Y"];

/// Lowercased affirmative / negative tokens by language. Whole-word matched
/// against the utterance. Order does not matter; first language whose token
/// hits wins. Extend by appending to the tables, not by adding new languages
/// inline at call sites.
const AFFIRMATIVES: &[&str] = &[
    "yes",
    "y",
    "true",
    "ok",
    "okay",
    "sure", // en
    "si",
    "sí",
    "verdadero", // es
    "oui",
    "vrai", // fr
    "ja",
    "wahr", // de
    "да",
    "истина", // ru
    "sim",
    "verdadeiro", // pt
    "sì",
    "vero", // it (sì covers es too)
    "waar", // nl
];

const NEGATIVES: &[&str] = &[
    "no", "n", "false", "nope", "nah",   // en
    "falso", // es
    "non", "faux", // fr
    "nein", "falsch", // de
    "нет", "ложь", // ru
    "não",  // pt
    // it: "no" already covered above
    // nl: "nee" + "onwaar"
    "nee", "onwaar",
];

/// Entry point. Maintains insertion order for `filled` / `missing` matching
/// the order of `input.slot_definitions`.
pub(crate) fn extract(input: &SlotExtractionInput) -> SlotExtractionOutput {
    let utterance = input.utterance.as_str();
    let mut slots: Vec<ExtractedSlot> = Vec::new();
    let mut filled: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    for def in &input.slot_definitions {
        let matched = match def.slot_type {
            SlotType::String => extract_string(utterance, def),
            SlotType::Enum => extract_enum(utterance, def),
            SlotType::Number => extract_number(utterance, def),
            SlotType::Date => extract_date(utterance, def),
            SlotType::Boolean => extract_boolean(utterance, def),
        };

        match matched {
            Some(slot) => {
                filled.push(def.name.clone());
                slots.push(slot);
            }
            None => match def.default_value.as_ref() {
                Some(default) => {
                    filled.push(def.name.clone());
                    slots.push(ExtractedSlot {
                        name: def.name.clone(),
                        value: Some(coerce_default(def.slot_type, default)),
                        confidence: 0.5,
                        source: SOURCE_DEFAULT.to_string(),
                    });
                }
                None => {
                    if def.required {
                        missing.push(def.name.clone());
                    }
                }
            },
        }
    }

    let all_required_filled = missing.is_empty();
    SlotExtractionOutput {
        slots,
        filled,
        missing,
        all_required_filled,
    }
}

fn extract_string(utterance: &str, def: &SlotDefinition) -> Option<ExtractedSlot> {
    let pattern = def.pattern.as_deref()?;
    let re = Regex::new(pattern).ok()?;
    let value = first_capture_or_match(&re, utterance)?;
    Some(make_slot(def, json!(value), SOURCE_REGEX))
}

fn extract_enum(utterance: &str, def: &SlotDefinition) -> Option<ExtractedSlot> {
    let values = def.enum_values.as_ref()?;
    let lowered = utterance.to_lowercase();
    // First enum value whose token appears as a whole-word substring wins.
    for value in values {
        if whole_word_match(&lowered, &value.to_lowercase()) {
            return Some(make_slot(def, json!(value), SOURCE_REGEX));
        }
    }
    None
}

fn extract_number(utterance: &str, def: &SlotDefinition) -> Option<ExtractedSlot> {
    let pattern = def.pattern.as_deref().unwrap_or(DEFAULT_NUMBER_PATTERN);
    let re = Regex::new(pattern).ok()?;
    let text = first_capture_or_match(&re, utterance)?;
    let parsed: f64 = text.parse().ok()?;
    let value = serde_json::Number::from_f64(parsed).map(Value::Number)?;
    Some(make_slot(def, value, SOURCE_REGEX))
}

fn extract_date(utterance: &str, def: &SlotDefinition) -> Option<ExtractedSlot> {
    let candidate: String = match def.pattern.as_deref() {
        Some(pattern) => {
            let re = Regex::new(pattern).ok()?;
            first_capture_or_match(&re, utterance)?
        }
        None => find_date_substring(utterance)?,
    };
    let date = DATE_FORMATS
        .iter()
        .find_map(|fmt| NaiveDate::parse_from_str(&candidate, fmt).ok())?;
    let iso = date.format("%Y-%m-%d").to_string();
    Some(make_slot(def, json!(iso), SOURCE_REGEX))
}

/// Returns the first capture group if present, else the full match. Used by
/// every pattern-driven extractor so user patterns like `order (\d+)` extract
/// just the number — not the surrounding tokens.
fn first_capture_or_match(re: &Regex, haystack: &str) -> Option<String> {
    let captures = re.captures(haystack)?;
    captures
        .get(1)
        .or_else(|| captures.get(0))
        .map(|m| m.as_str().to_string())
}

fn extract_boolean(utterance: &str, def: &SlotDefinition) -> Option<ExtractedSlot> {
    let lowered = utterance.to_lowercase();
    for token in AFFIRMATIVES {
        if whole_word_match(&lowered, token) {
            return Some(make_slot(def, json!(true), SOURCE_I18N));
        }
    }
    for token in NEGATIVES {
        if whole_word_match(&lowered, token) {
            return Some(make_slot(def, json!(false), SOURCE_I18N));
        }
    }
    None
}

fn make_slot(def: &SlotDefinition, value: Value, source: &str) -> ExtractedSlot {
    ExtractedSlot {
        name: def.name.clone(),
        value: Some(value),
        confidence: 1.0,
        source: source.to_string(),
    }
}

/// Best-effort default-value coercion to match the slot type. Falls back to a
/// JSON string when parse fails so the caller still sees the configured value.
fn coerce_default(slot_type: SlotType, raw: &str) -> Value {
    match slot_type {
        SlotType::String | SlotType::Enum => json!(raw),
        SlotType::Number => raw
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| json!(raw)),
        SlotType::Date => DATE_FORMATS
            .iter()
            .find_map(|fmt| NaiveDate::parse_from_str(raw, fmt).ok())
            .map(|d| json!(d.format("%Y-%m-%d").to_string()))
            .unwrap_or_else(|| json!(raw)),
        SlotType::Boolean => match raw.to_lowercase().as_str() {
            t if AFFIRMATIVES.contains(&t) => json!(true),
            t if NEGATIVES.contains(&t) => json!(false),
            _ => json!(raw),
        },
    }
}

/// Try the configured `DATE_FORMATS` against likely date substrings.
fn find_date_substring(utterance: &str) -> Option<String> {
    // One regex per format family: numeric dates anywhere in the utterance.
    // We capture broadly then let chrono validate. Slash and dash formats
    // covered; expand here if a new format is added.
    static DATE_SCAN: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
        Regex::new(r"\d{1,4}[-/]\d{1,2}[-/]\d{1,4}").expect("static date scan regex")
    });
    DATE_SCAN.find(utterance).map(|m| m.as_str().to_string())
}

/// Lowercase `haystack`/`needle` must be supplied by the caller.
fn whole_word_match(haystack: &str, needle: &str) -> bool {
    let nchars: Vec<char> = needle.chars().collect();
    if nchars.is_empty() {
        return false;
    }
    let hchars: Vec<char> = haystack.chars().collect();
    let nlen = nchars.len();
    let hlen = hchars.len();
    if nlen > hlen {
        return false;
    }
    for start in 0..=hlen - nlen {
        if hchars[start..start + nlen] != nchars[..] {
            continue;
        }
        let left_ok = start == 0 || !hchars[start - 1].is_alphanumeric();
        let right_ok = start + nlen == hlen || !hchars[start + nlen].is_alphanumeric();
        if left_ok && right_ok {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(name: &str, slot_type: SlotType) -> SlotDefinition {
        SlotDefinition {
            name: name.to_string(),
            slot_type,
            pattern: None,
            required: true,
            enum_values: None,
            default_value: None,
        }
    }

    fn run(utterance: &str, defs: Vec<SlotDefinition>) -> SlotExtractionOutput {
        extract(&SlotExtractionInput {
            utterance: utterance.to_string(),
            slot_definitions: defs,
        })
    }

    #[test]
    fn string_with_pattern_extracts_capture_group() {
        let mut d = def("city", SlotType::String);
        d.pattern = Some(r"in ([A-Z][a-z]+)".to_string());
        let out = run("I live in Paris with my dog", vec![d]);
        assert_eq!(out.slots.len(), 1);
        assert_eq!(out.slots[0].value, Some(json!("Paris")));
        assert_eq!(out.slots[0].source, "regex");
        assert!(out.all_required_filled);
    }

    #[test]
    fn string_without_pattern_is_missing() {
        let out = run("hello world", vec![def("city", SlotType::String)]);
        assert!(out.missing.contains(&"city".to_string()));
        assert!(!out.all_required_filled);
    }

    #[test]
    fn enum_case_insensitive_whole_word_match() {
        let mut d = def("color", SlotType::Enum);
        d.enum_values = Some(vec!["Red".into(), "Blue".into(), "Green".into()]);
        let out = run("I prefer the BLUE one", vec![d]);
        assert_eq!(out.slots[0].value, Some(json!("Blue")));
    }

    #[test]
    fn enum_no_substring_inside_word_match() {
        let mut d = def("color", SlotType::Enum);
        d.enum_values = Some(vec!["red".into()]);
        // "redirect" contains "red" but should not match as a whole word.
        let out = run("please redirect me", vec![d]);
        assert!(out.missing.contains(&"color".to_string()));
    }

    #[test]
    fn number_default_pattern_extracts_decimal() {
        let out = run(
            "the price is 42.5 dollars",
            vec![def("price", SlotType::Number)],
        );
        assert_eq!(out.slots[0].value, Some(json!(42.5)));
    }

    #[test]
    fn number_negative_integer() {
        let out = run("temperature -7 today", vec![def("temp", SlotType::Number)]);
        assert_eq!(out.slots[0].value, Some(json!(-7.0)));
    }

    #[test]
    fn date_iso_8601() {
        let out = run("meeting on 2026-06-15", vec![def("when", SlotType::Date)]);
        assert_eq!(out.slots[0].value, Some(json!("2026-06-15")));
    }

    #[test]
    fn date_us_format_normalised_to_iso() {
        let out = run("on 06/15/2026", vec![def("when", SlotType::Date)]);
        assert_eq!(out.slots[0].value, Some(json!("2026-06-15")));
    }

    #[test]
    fn boolean_affirmative_english() {
        let out = run("Yes please", vec![def("confirm", SlotType::Boolean)]);
        assert_eq!(out.slots[0].value, Some(json!(true)));
    }

    #[test]
    fn boolean_negative_german() {
        let out = run("Nein, danke", vec![def("confirm", SlotType::Boolean)]);
        assert_eq!(out.slots[0].value, Some(json!(false)));
    }

    #[test]
    fn boolean_negative_french() {
        let out = run("non merci", vec![def("confirm", SlotType::Boolean)]);
        assert_eq!(out.slots[0].value, Some(json!(false)));
    }

    #[test]
    fn boolean_inside_word_does_not_match() {
        // "ya" should not match — only whole-word tokens count.
        let out = run("yacht trip", vec![def("confirm", SlotType::Boolean)]);
        assert!(out.missing.contains(&"confirm".to_string()));
    }

    #[test]
    fn default_value_fills_when_no_match() {
        let mut d = def("confirm", SlotType::Boolean);
        d.default_value = Some("true".to_string());
        let out = run("hello world", vec![d]);
        assert_eq!(out.slots.len(), 1);
        assert_eq!(out.slots[0].value, Some(json!(true)));
        assert_eq!(out.slots[0].source, "default");
        assert_eq!(out.slots[0].confidence, 0.5);
        assert!(out.all_required_filled);
    }

    #[test]
    fn optional_unmatched_is_neither_filled_nor_missing() {
        let mut d = def("city", SlotType::String);
        d.required = false;
        d.pattern = Some(r"in ([A-Z][a-z]+)".to_string());
        let out = run("nothing matches here", vec![d]);
        assert!(out.filled.is_empty());
        assert!(out.missing.is_empty());
        assert!(out.slots.is_empty());
        assert!(out.all_required_filled);
    }

    #[test]
    fn invalid_pattern_is_treated_as_no_match() {
        let mut d = def("x", SlotType::String);
        d.pattern = Some("[unterminated".to_string());
        let out = run("anything", vec![d]);
        assert!(out.missing.contains(&"x".to_string()));
    }

    #[test]
    fn multiple_slots_preserve_definition_order() {
        let mut city = def("city", SlotType::String);
        city.pattern = Some(r"in (\w+)".to_string());
        let out = run(
            "stay in Rome for 3 days yes please",
            vec![
                city,
                def("days", SlotType::Number),
                def("confirm", SlotType::Boolean),
            ],
        );
        assert_eq!(
            out.filled,
            vec![
                "city".to_string(),
                "days".to_string(),
                "confirm".to_string()
            ]
        );
        assert_eq!(out.slots[0].value, Some(json!("Rome")));
        assert_eq!(out.slots[1].value, Some(json!(3.0)));
        assert_eq!(out.slots[2].value, Some(json!(true)));
    }
}
