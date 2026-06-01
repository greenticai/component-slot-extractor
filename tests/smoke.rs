use component_slot_extractor::{
    SlotDefinition, SlotExtractionInput, SlotExtractionOutput, SlotType,
};

#[test]
fn skeleton_returns_all_required_missing() {
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
    assert_eq!(output.missing, vec!["order_id".to_string()]);
    assert!(!output.all_required_filled);
    assert!(output.slots.is_empty());
    assert!(output.filled.is_empty());
}
