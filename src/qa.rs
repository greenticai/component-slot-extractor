use std::collections::BTreeMap;

use greentic_types::schemas::component::v0_6_0::{ComponentQaSpec, QaMode};

/// Returns an empty QA spec. This component requires no setup configuration.
pub fn qa_spec_empty() -> ComponentQaSpec {
    ComponentQaSpec {
        mode: QaMode::Default,
        title: greentic_types::i18n_text::I18nText::new("component.name", None),
        description: Some(greentic_types::i18n_text::I18nText::new(
            "component.description",
            None,
        )),
        questions: Vec::new(),
        defaults: BTreeMap::new(),
    }
}
