use component_slot_extractor::{
    SlotDefinition, SlotExtractionInput, SlotExtractionOutput, SlotType,
};

#[test]
fn number_pattern_with_capture_group_extracts_the_number() {
    let input = SlotExtractionInput {
        utterance: "I want to refund order 42".into(),
        slot_definitions: vec![SlotDefinition {
            name: "order_id".into(),
            slot_type: SlotType::Number,
            pattern: Some(r"order (\d+)".into()),
            required: true,
            enum_values: None,
            default_value: None,
        }],
    };
    let output: SlotExtractionOutput = component_slot_extractor::extract_slots(&input);
    assert_eq!(output.filled, vec!["order_id".to_string()]);
    assert!(output.missing.is_empty());
    assert!(output.all_required_filled);
    assert_eq!(output.slots.len(), 1);
    assert_eq!(output.slots[0].value, Some(serde_json::json!(42.0)));
    assert_eq!(output.slots[0].source, "regex");
}

#[test]
fn mixed_slots_end_to_end() {
    let input = SlotExtractionInput {
        utterance: "Book a Blue room for 2026-07-04, party of 4, yes please".into(),
        slot_definitions: vec![
            SlotDefinition {
                name: "color".into(),
                slot_type: SlotType::Enum,
                pattern: None,
                required: true,
                enum_values: Some(vec!["Red".into(), "Blue".into(), "Green".into()]),
                default_value: None,
            },
            SlotDefinition {
                name: "when".into(),
                slot_type: SlotType::Date,
                pattern: None,
                required: true,
                enum_values: None,
                default_value: None,
            },
            SlotDefinition {
                name: "party_size".into(),
                slot_type: SlotType::Number,
                pattern: Some(r"party of (\d+)".into()),
                required: true,
                enum_values: None,
                default_value: None,
            },
            SlotDefinition {
                name: "confirm".into(),
                slot_type: SlotType::Boolean,
                pattern: None,
                required: true,
                enum_values: None,
                default_value: None,
            },
        ],
    };
    let output = component_slot_extractor::extract_slots(&input);
    assert!(output.all_required_filled, "missing: {:?}", output.missing);
    let by_name: std::collections::BTreeMap<_, _> = output
        .slots
        .iter()
        .map(|s| (s.name.as_str(), s.value.clone().unwrap()))
        .collect();
    assert_eq!(by_name["color"], serde_json::json!("Blue"));
    assert_eq!(by_name["when"], serde_json::json!("2026-07-04"));
    assert_eq!(by_name["party_size"], serde_json::json!(4.0));
    assert_eq!(by_name["confirm"], serde_json::json!(true));
}
