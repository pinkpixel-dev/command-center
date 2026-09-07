//! The live Codex model catalogue.
//!
//! Command Center never ships a Codex model list. The catalogue depends on the
//! connected account's plan, so an invented default would offer models the
//! user cannot reach. If discovery fails, the list is empty and Settings asks
//! for a selection rather than guessing one.

use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

pub const MODEL_LIST_TIMEOUT: Duration = Duration::from_secs(30);

/// How many pages of `model/list` will be followed before stopping. The
/// catalogue is small; this only exists so a broken cursor cannot loop.
pub const MAX_MODEL_PAGES: usize = 10;

/// A model the connected account can actually use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexModel {
    pub id: String,
    pub display_name: String,
    pub is_default: bool,
}

const MAX_ID_LENGTH: usize = 256;
const MAX_NAME_LENGTH: usize = 128;

/// Reads one page of `model/list`, returning the usable models and the cursor
/// for the next page.
pub fn parse_model_page(value: &Value) -> (Vec<CodexModel>, Option<String>) {
    let models = value
        .get("data")
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(parse_model).collect())
        .unwrap_or_default();

    let next = value
        .get("nextCursor")
        .and_then(Value::as_str)
        .filter(|cursor| !cursor.is_empty() && cursor.len() <= 4096)
        .map(str::to_owned);

    (models, next)
}

fn parse_model(entry: &Value) -> Option<CodexModel> {
    // Hidden models are not offered in Codex's own picker, so they are not
    // offered here either.
    if entry.get("hidden").and_then(Value::as_bool) == Some(true) {
        return None;
    }

    let id = entry.get("id").and_then(Value::as_str)?.trim();
    if id.is_empty()
        || id.len() > MAX_ID_LENGTH
        || id.chars().any(char::is_control)
        || id.chars().any(char::is_whitespace)
    {
        return None;
    }

    let display_name = entry
        .get("displayName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty() && name.len() <= MAX_NAME_LENGTH)
        .filter(|name| !name.chars().any(char::is_control))
        .unwrap_or(id)
        .to_owned();

    Some(CodexModel {
        id: id.to_owned(),
        display_name,
        is_default: entry.get("isDefault").and_then(Value::as_bool) == Some(true),
    })
}

/// Removes duplicates and puts the account's default first, so the picker
/// opens on the model Codex itself would have used.
pub fn arrange(mut models: Vec<CodexModel>) -> Vec<CodexModel> {
    let mut seen = std::collections::HashSet::new();
    models.retain(|model| seen.insert(model.id.clone()));
    models.sort_by_key(|model| !model.is_default);
    models
}

/// Whether a saved selection still exists in the live catalogue.
pub fn contains(models: &[CodexModel], model_id: &str) -> bool {
    models.iter().any(|model| model.id == model_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn page() -> Value {
        json!({
            "data": [
                { "id": "gpt-5.6-luna", "displayName": "GPT-5.6 Luna", "isDefault": true },
                { "id": "gpt-5.6-terra", "displayName": "GPT-5.6 Terra" },
            ],
            "nextCursor": null,
        })
    }

    #[test]
    fn a_page_yields_its_models_and_no_cursor_when_it_is_the_last() {
        let (models, cursor) = parse_model_page(&page());

        assert_eq!(cursor, None);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "gpt-5.6-luna");
        assert_eq!(models[0].display_name, "GPT-5.6 Luna");
        assert!(models[0].is_default);
        assert!(!models[1].is_default);
    }

    #[test]
    fn a_cursor_is_carried_forward_when_there_are_more_pages() {
        let value = json!({ "data": [], "nextCursor": "page-2" });
        let (_, cursor) = parse_model_page(&value);
        assert_eq!(cursor.as_deref(), Some("page-2"));
    }

    #[test]
    fn an_empty_or_unreadable_reply_yields_no_models_rather_than_a_guess() {
        for value in [json!({}), json!({ "data": null }), json!("nonsense")] {
            let (models, cursor) = parse_model_page(&value);
            assert!(models.is_empty());
            assert_eq!(cursor, None);
        }
    }

    #[test]
    fn hidden_models_are_not_offered() {
        let value = json!({
            "data": [
                { "id": "visible", "displayName": "Visible" },
                { "id": "internal", "displayName": "Internal", "hidden": true },
            ]
        });

        let (models, _) = parse_model_page(&value);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "visible");
    }

    #[test]
    fn a_model_without_a_display_name_falls_back_to_its_id() {
        let value = json!({ "data": [{ "id": "gpt-5.6-sol" }] });
        let (models, _) = parse_model_page(&value);

        assert_eq!(models[0].display_name, "gpt-5.6-sol");
    }

    #[test]
    fn unusable_model_entries_are_dropped() {
        let value = json!({
            "data": [
                { "id": "" },
                { "id": "has spaces" },
                { "displayName": "no id at all" },
                { "id": "x".repeat(MAX_ID_LENGTH + 1) },
                { "id": "good-model" },
            ]
        });

        let (models, _) = parse_model_page(&value);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "good-model");
    }

    #[test]
    fn the_default_model_is_listed_first() {
        let models = arrange(vec![
            CodexModel { id: "b".into(), display_name: "B".into(), is_default: false },
            CodexModel { id: "a".into(), display_name: "A".into(), is_default: true },
        ]);

        assert_eq!(models[0].id, "a");
        assert_eq!(models[1].id, "b");
    }

    #[test]
    fn duplicate_ids_across_pages_are_collapsed() {
        let models = arrange(vec![
            CodexModel { id: "a".into(), display_name: "A".into(), is_default: false },
            CodexModel { id: "a".into(), display_name: "A again".into(), is_default: false },
        ]);

        assert_eq!(models.len(), 1);
    }

    #[test]
    fn a_saved_selection_can_be_checked_against_the_live_list() {
        let (models, _) = parse_model_page(&page());

        assert!(contains(&models, "gpt-5.6-terra"));
        // A model that disappeared has to be re-chosen, not silently kept.
        assert!(!contains(&models, "gpt-5.6-retired"));
    }
}
