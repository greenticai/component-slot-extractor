#[cfg(target_arch = "wasm32")]
use std::collections::BTreeMap;

#[cfg(target_arch = "wasm32")]
use greentic_types::cbor::canonical;
#[cfg(target_arch = "wasm32")]
use greentic_types::i18n_text::I18nText;
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::component::v0_6_0::ComponentQaSpec;
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::component::v0_6_0::{
    ComponentDescribe, ComponentInfo, ComponentOperation, ComponentRunInput, ComponentRunOutput,
    schema_hash,
};
#[cfg(target_arch = "wasm32")]
mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "component-v0-v6-v0",
    });
}
#[cfg(target_arch = "wasm32")]
use bindings::exports::greentic::component::{
    component_descriptor, component_i18n,
    component_qa::{self, QaMode},
    component_runtime, component_schema,
};

pub mod i18n;
pub mod i18n_bundle;
pub mod qa;

const COMPONENT_NAME: &str = "component-slot-extractor";
const COMPONENT_ORG: &str = "ai.greentic";
const COMPONENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = ".greentic.wasi")]
static WASI_TARGET_MARKER: [u8; 13] = *b"wasm32-wasip2";

#[cfg(target_arch = "wasm32")]
struct Component;

#[cfg(target_arch = "wasm32")]
impl component_descriptor::Guest for Component {
    fn get_component_info() -> Vec<u8> {
        component_info_cbor()
    }

    fn describe() -> Vec<u8> {
        component_describe_cbor()
    }
}

#[cfg(target_arch = "wasm32")]
impl component_schema::Guest for Component {
    fn input_schema() -> Vec<u8> {
        input_schema_cbor()
    }

    fn output_schema() -> Vec<u8> {
        output_schema_cbor()
    }

    fn config_schema() -> Vec<u8> {
        config_schema_cbor()
    }
}

#[cfg(target_arch = "wasm32")]
impl component_runtime::Guest for Component {
    fn run(input: Vec<u8>, state: Vec<u8>) -> component_runtime::RunResult {
        let value = parse_payload(&input);
        let extraction_input: SlotExtractionInput =
            serde_json::from_value(value).unwrap_or_else(|_| SlotExtractionInput {
                utterance: String::new(),
                slot_definitions: Vec::new(),
            });

        let output = extract_slots(&extraction_input);

        component_runtime::RunResult {
            output: encode_cbor(&output),
            new_state: state,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl component_qa::Guest for Component {
    fn qa_spec(_mode: QaMode) -> Vec<u8> {
        encode_cbor(&qa::qa_spec_empty())
    }

    fn apply_answers(_mode: QaMode, _current_config: Vec<u8>, _answers: Vec<u8>) -> Vec<u8> {
        encode_cbor(&serde_json::json!({}))
    }
}

#[cfg(target_arch = "wasm32")]
impl component_i18n::Guest for Component {
    fn i18n_keys() -> Vec<String> {
        i18n::all_keys()
    }
}

#[cfg(target_arch = "wasm32")]
bindings::export!(Component with_types_in bindings);

// --- Public types ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlotDefinition {
    pub name: String,
    #[serde(rename = "slot_type")]
    pub slot_type: SlotType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotType {
    String,
    Enum,
    Number,
    Date,
    Boolean,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlotExtractionInput {
    pub utterance: String,
    pub slot_definitions: Vec<SlotDefinition>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedSlot {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    pub confidence: f32,
    pub source: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlotExtractionOutput {
    pub slots: Vec<ExtractedSlot>,
    pub filled: Vec<String>,
    pub missing: Vec<String>,
    pub all_required_filled: bool,
}

// --- Public API ---

/// No-op extraction stub (PR 1). Returns all required slots as missing.
/// PR 2 will implement regex-based extraction for all 5 slot types.
pub fn extract_slots(input: &SlotExtractionInput) -> SlotExtractionOutput {
    let missing: Vec<String> = input
        .slot_definitions
        .iter()
        .filter(|def| def.required)
        .map(|def| def.name.clone())
        .collect();

    SlotExtractionOutput {
        slots: Vec::new(),
        filled: Vec::new(),
        missing,
        all_required_filled: false,
    }
}

pub fn describe_payload() -> String {
    serde_json::json!({
        "component": {
            "name": COMPONENT_NAME,
            "org": COMPONENT_ORG,
            "version": COMPONENT_VERSION,
            "world": "greentic:component/component@0.6.0",
            "schemas": {
                "component": "schemas/component.schema.json",
                "input": "schemas/io/input.schema.json",
                "output": "schemas/io/output.schema.json"
            }
        }
    })
    .to_string()
}

// --- WASM-only helpers ---

#[cfg(target_arch = "wasm32")]
fn encode_cbor<T: serde::Serialize>(value: &T) -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(value).expect("encode cbor")
}

#[cfg(target_arch = "wasm32")]
fn parse_payload(input: &[u8]) -> serde_json::Value {
    if let Ok(value) = canonical::from_cbor(input) {
        return value;
    }
    serde_json::from_slice(input).unwrap_or_else(|_| serde_json::json!({}))
}

#[cfg(target_arch = "wasm32")]
fn input_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([
            (
                "utterance".to_string(),
                SchemaIr::String {
                    min_len: Some(0),
                    max_len: None,
                    regex: None,
                    format: None,
                },
            ),
            (
                "slot_definitions".to_string(),
                SchemaIr::Array {
                    items: Box::new(SchemaIr::Object {
                        properties: BTreeMap::from([
                            (
                                "name".to_string(),
                                SchemaIr::String {
                                    min_len: Some(1),
                                    max_len: None,
                                    regex: None,
                                    format: None,
                                },
                            ),
                            (
                                "slot_type".to_string(),
                                SchemaIr::String {
                                    min_len: Some(1),
                                    max_len: None,
                                    regex: None,
                                    format: None,
                                },
                            ),
                        ]),
                        required: vec!["name".to_string(), "slot_type".to_string()],
                        additional: AdditionalProperties::Allow,
                    }),
                    min_items: None,
                    max_items: None,
                },
            ),
        ]),
        required: vec!["utterance".to_string(), "slot_definitions".to_string()],
        additional: AdditionalProperties::Allow,
    }
}

#[cfg(target_arch = "wasm32")]
fn output_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([
            (
                "slots".to_string(),
                SchemaIr::Array {
                    items: Box::new(SchemaIr::Object {
                        properties: BTreeMap::new(),
                        required: Vec::new(),
                        additional: AdditionalProperties::Allow,
                    }),
                    min_items: None,
                    max_items: None,
                },
            ),
            (
                "filled".to_string(),
                SchemaIr::Array {
                    items: Box::new(SchemaIr::String {
                        min_len: None,
                        max_len: None,
                        regex: None,
                        format: None,
                    }),
                    min_items: None,
                    max_items: None,
                },
            ),
            (
                "missing".to_string(),
                SchemaIr::Array {
                    items: Box::new(SchemaIr::String {
                        min_len: None,
                        max_len: None,
                        regex: None,
                        format: None,
                    }),
                    min_items: None,
                    max_items: None,
                },
            ),
            ("all_required_filled".to_string(), SchemaIr::Boolean),
        ]),
        required: vec![
            "slots".to_string(),
            "filled".to_string(),
            "missing".to_string(),
            "all_required_filled".to_string(),
        ],
        additional: AdditionalProperties::Deny,
    }
}

#[cfg(target_arch = "wasm32")]
fn config_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::new(),
        required: Vec::new(),
        additional: AdditionalProperties::Deny,
    }
}

#[cfg(target_arch = "wasm32")]
fn component_info() -> ComponentInfo {
    ComponentInfo {
        id: format!("{COMPONENT_ORG}.{COMPONENT_NAME}"),
        version: COMPONENT_VERSION.to_string(),
        role: "tool".to_string(),
        display_name: Some(I18nText::new(
            "component.name",
            Some(COMPONENT_NAME.to_string()),
        )),
    }
}

#[cfg(target_arch = "wasm32")]
fn component_describe() -> ComponentDescribe {
    let input = input_schema();
    let output = output_schema();
    let config = config_schema();
    let op_schema_hash = schema_hash(&input, &output, &config).unwrap_or_default();

    ComponentDescribe {
        info: component_info(),
        provided_capabilities: Vec::new(),
        required_capabilities: Vec::new(),
        metadata: BTreeMap::new(),
        operations: vec![ComponentOperation {
            id: "extract_slots".to_string(),
            display_name: Some(I18nText::new("operation.extract_slots.label", None)),
            input: ComponentRunInput { schema: input },
            output: ComponentRunOutput { schema: output },
            defaults: BTreeMap::new(),
            redactions: Vec::new(),
            constraints: BTreeMap::new(),
            schema_hash: op_schema_hash,
        }],
        config_schema: config,
    }
}

#[cfg(target_arch = "wasm32")]
fn component_info_cbor() -> Vec<u8> {
    encode_cbor(&component_info())
}

#[cfg(target_arch = "wasm32")]
fn component_describe_cbor() -> Vec<u8> {
    encode_cbor(&component_describe())
}

#[cfg(target_arch = "wasm32")]
fn input_schema_cbor() -> Vec<u8> {
    encode_cbor(&input_schema())
}

#[cfg(target_arch = "wasm32")]
fn output_schema_cbor() -> Vec<u8> {
    encode_cbor(&output_schema())
}

#[cfg(target_arch = "wasm32")]
fn config_schema_cbor() -> Vec<u8> {
    encode_cbor(&config_schema())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_payload_is_json() {
        let payload = describe_payload();
        let json: serde_json::Value = serde_json::from_str(&payload).expect("valid json");
        assert_eq!(json["component"]["name"], "component-slot-extractor");
    }

    #[test]
    fn extract_slots_returns_all_missing_for_required() {
        let input = SlotExtractionInput {
            utterance: "hello world".into(),
            slot_definitions: vec![
                SlotDefinition {
                    name: "city".into(),
                    slot_type: SlotType::String,
                    pattern: None,
                    required: true,
                    enum_values: None,
                    default_value: None,
                },
                SlotDefinition {
                    name: "optional_field".into(),
                    slot_type: SlotType::Boolean,
                    pattern: None,
                    required: false,
                    enum_values: None,
                    default_value: None,
                },
            ],
        };
        let output = extract_slots(&input);
        assert_eq!(output.missing, vec!["city".to_string()]);
        assert!(output.filled.is_empty());
        assert!(output.slots.is_empty());
        assert!(!output.all_required_filled);
    }

    #[test]
    fn slot_type_serde_roundtrip() {
        let def = SlotDefinition {
            name: "test".into(),
            slot_type: SlotType::Date,
            pattern: Some(r"\d{4}-\d{2}-\d{2}".into()),
            required: true,
            enum_values: None,
            default_value: None,
        };
        let json = serde_json::to_string(&def).expect("serialize");
        let recovered: SlotDefinition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(recovered.slot_type, SlotType::Date);
        assert_eq!(recovered.pattern.as_deref(), Some(r"\d{4}-\d{2}-\d{2}"));
    }
}
