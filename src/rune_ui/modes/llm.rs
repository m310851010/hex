//! The Modes pane's Processing section: which service rewrites dictation,
//! the AI rewriting switch, and the direct LLM endpoint's fields. Kept in
//! its own file so `modes.rs` stays under the source-size cap.

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use rune::{El, col, row, text};
use rune_kit::{Switch, TextArea, TextInput};

use crate::app_settings::{DictationMode, LlmProvider, LlmSettings};
use crate::desktop::i18n::tr;
use crate::desktop::opencode_catalog::OpenCodeFeature;

use super::super::Ui;
use super::super::controls::{
    ButtonKind, DESCRIPTION, METRICS, Paint, button, note, section, setting_row,
};
use super::super::models::{self, Target};
use super::{FIELD_WIDTH, Selection, mode, update_mode};

pub(super) fn processing(
    ui: &Ui,
    p: Paint,
    selection: Selection,
    current: &DictationMode,
    layer: &mut Vec<El>,
) -> El {
    if super::super::platform_settings::opencode_managed() {
        let row = super::super::controls::managed_row(
            p,
            "processing-enabled",
            None,
            tr("OpenCode transformation"),
        );
        return section(p, "processing", tr("Processing"), vec![row]);
    }
    let llm = &ui.settings.llm;
    let direct = llm.provider.direct();
    let enabled = current.post_processing.enabled;
    // A direct provider needs no running OpenCode service, so its switch is
    // never held back by the catalog's availability.
    let can_toggle = enabled || direct || ui.models.available();
    let toggle = Switch::new(enabled)
        .disabled(!can_toggle)
        .on_toggle(|ui: &mut Ui| {
            let enabled = !mode(&ui.settings, ui.modes.selected)
                .post_processing
                .enabled;
            update_mode(ui, |mode| mode.post_processing.enabled = enabled);
        })
        .build(p.k)
        .key("processing-toggle")
        .label(tr("AI rewriting"));
    let reason = (!can_toggle)
        .then(|| models::unavailable_reason(ui, OpenCodeFeature::Transformation))
        .flatten();
    let providers: Vec<&str> = LlmProvider::ALL
        .iter()
        .map(|provider| tr(provider.label()))
        .collect();
    let selected = LlmProvider::ALL
        .iter()
        .position(|provider| *provider == llm.provider)
        .unwrap_or_default();
    let provider_pick = super::super::controls::segmented(
        p,
        "llm-provider",
        &providers,
        selected,
        |ui: &mut Ui, index| {
            let provider = LlmProvider::ALL[index];
            let key = ui.settings.llm.api_key.clone();
            ui.update(|settings| settings.llm = LlmSettings::for_provider(provider, &key));
            ui.models.direct_models.clear();
            ui.models.direct_error = None;
            ui.models.direct_test = None;
        },
    );
    let mut rows = vec![
        setting_row(
            p,
            "llm-provider",
            tr("Provider"),
            Some(tr("OpenCode service, or a direct endpoint you configure")),
            provider_pick,
        ),
        setting_row(p, "processing-enabled", tr("AI rewriting"), reason, toggle),
    ];
    if direct {
        rows.push(setting_row(
            p,
            "llm-base-url",
            tr("Base URL"),
            None,
            TextInput::new(&ui.modes.editor.base_url)
                .label(tr("Base URL"))
                .placeholder("https://api.openai.com/v1")
                .width(Some(FIELD_WIDTH))
                .on_edit(|ui: &mut Ui| {
                    ui.modes.editor.llm_touched = true;
                    &mut ui.modes.editor.base_url
                })
                .build(p.k)
                .key("llm-base-url-field"),
        ));
        rows.push(setting_row(
            p,
            "llm-api-key",
            tr("API key"),
            Some(tr("Stored in settings.json as plain text")),
            TextInput::new(&ui.modes.editor.api_key)
                .secure()
                .label(tr("API key"))
                .placeholder("sk-…")
                .width(Some(FIELD_WIDTH))
                .on_edit(|ui: &mut Ui| {
                    ui.modes.editor.llm_touched = true;
                    &mut ui.modes.editor.api_key
                })
                .build(p.k)
                .key("llm-api-key-field"),
        ));
        let model_field = TextInput::new(&ui.modes.editor.model)
            .label(tr("Model"))
            .placeholder(tr("Model id"))
            .width(Some(FIELD_WIDTH))
            .on_edit(|ui: &mut Ui| {
                ui.modes.editor.llm_touched = true;
                &mut ui.modes.editor.model
            })
            .build(p.k)
            .key("llm-model-field");
        // A chevron beside the hand-typed field opens a searchable list of
        // the endpoint's model ids, once fetched. The Combobox owns the
        // chevron as its trigger, so the popover anchors beside the field.
        let chevron = button(
            p,
            "llm-model-chevron",
            "⌄",
            ButtonKind::Secondary,
            |_: &mut Ui, _| {},
        )
        .disabled(ui.models.direct_models.is_empty());
        let options: Vec<_> = ui
            .models
            .direct_models
            .iter()
            .enumerate()
            .map(|(index, model)| rune_kit::ComboboxOption::new(index.to_string(), model.clone()))
            .collect();
        let current = ui
            .models
            .direct_models
            .iter()
            .position(|model| *model == ui.modes.editor.model.text.trim())
            .map(|index| index.to_string());
        let picker = rune_kit::Combobox::new(
            "llm-model-picker",
            &ui.models.direct_picker,
            &options,
            |ui: &mut Ui| &mut ui.models.direct_picker,
        )
        .trigger(chevron)
        .label(tr("Model"))
        .current(current.as_deref())
        .width(320.0)
        .max_height(360.0)
        .search_placeholder(tr("Search models"))
        .empty_message(tr("No models match"))
        .placement(rune_kit::Placement::BottomEnd)
        .on_choose(|ui: &mut Ui, id: &str, _: &mut rune::Tx| {
            if let Some(model) = id
                .parse::<usize>()
                .ok()
                .and_then(|index| ui.models.direct_models.get(index))
            {
                ui.modes.editor.model = rune::TextEdit::new(model.clone());
                ui.modes.editor.llm_touched = true;
                ui.models.direct_picker.close();
                ui.flush_drafts();
            }
        })
        .build(p.k, layer);
        let model_control = row()
            .key("llm-model-control")
            .gap(6.0)
            .center_y()
            .child(model_field)
            .child(picker);
        rows.push(setting_row(
            p,
            "llm-model",
            tr("Model"),
            None,
            model_control,
        ));
        rows.push(
            row()
                .key("llm-actions")
                .gap(8.0)
                .px(METRICS.row_px)
                .child(button(
                    p,
                    "llm-fetch-models",
                    tr("Fetch models"),
                    ButtonKind::Secondary,
                    |ui: &mut Ui, _| fetch_direct_models(ui),
                ))
                .child(button(
                    p,
                    "llm-test",
                    tr("Test connection"),
                    ButtonKind::Secondary,
                    |ui: &mut Ui, _| test_direct_llm(ui),
                )),
        );
        if let Some(error) = &ui.models.direct_error {
            rows.push(note("llm-error", error, p.danger()));
        }
        if let Some(answer) = &ui.models.direct_test {
            rows.push(note("llm-test-answer", answer, p.muted()));
        }
    }
    if enabled {
        if !direct {
            rows.extend(models::rows(ui, p, Target::Mode(selection), layer));
        }
        rows.push(setting_row(
            p,
            "deadline",
            tr("Deadline"),
            Some(tr("Seconds")),
            TextInput::new(&ui.modes.editor.deadline)
                .label(tr("Deadline"))
                .placeholder("30")
                .width(Some(96.0))
                .on_edit(|ui: &mut Ui| &mut ui.modes.editor.deadline)
                .build(p.k)
                .key("deadline-field"),
        ));
        rows.push(
            col()
                .key("instructions")
                .px(METRICS.row_px)
                .py(8.0)
                .gap(6.0)
                .child(
                    text(tr("Instructions"))
                        .key("title")
                        .font(p.k.ty.ui)
                        .color(p.text()),
                )
                .child(
                    text(tr(
                        "System instructions for rewriting dictation in this mode",
                    ))
                    .key("description")
                    .font(DESCRIPTION)
                    .color(p.muted()),
                )
                .child(
                    TextArea::new(&ui.modes.editor.prompt)
                        .placeholder(tr("Describe how the model should rewrite your dictation…"))
                        .rows(4)
                        .width(None)
                        .on_edit(|ui: &mut Ui| &mut ui.modes.editor.prompt)
                        .build(p.k)
                        .key("prompt"),
                ),
        );
    }
    if !direct && let Some(notice) = models::unavailable(ui, p, OpenCodeFeature::Transformation) {
        rows.push(notice);
    }
    section(p, "processing", tr("Processing"), rows)
}

/// Asks the configured endpoint for its model ids. Bounded to two seconds,
/// so a stalled endpoint never holds the window up.
fn fetch_direct_models(ui: &mut Ui) {
    ui.flush_drafts();
    // The window may have been opened before `hex settings` changed the
    // provider; read the saved config, not a stale in-memory copy.
    let settings = crate::app_settings::AppSettings::saved_llm();
    ui.models.direct_error = None;
    match crate::dictation::llm_client::list_models(&settings, Duration::from_secs(2)) {
        Ok(models) => {
            ui.models.direct_models = models;
            ui.models.direct_error = None;
        }
        Err(error) => ui.models.direct_error = Some(format!("{error:#}")),
    }
}

/// Sends a minimal prompt to confirm the endpoint and key work.
fn test_direct_llm(ui: &mut Ui) {
    ui.flush_drafts();
    let settings = crate::app_settings::AppSettings::saved_llm();
    let cancelled = AtomicBool::new(false);
    ui.models.direct_test = Some(
        match crate::dictation::llm_client::generate(
            "Reply with exactly: OK",
            &settings,
            Duration::from_secs(5),
            &cancelled,
        ) {
            Ok(_) => tr("Connection works").to_string(),
            Err(error) => format!("{error:#}"),
        },
    );
}
