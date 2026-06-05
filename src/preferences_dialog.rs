/* preferences_dialog.rs
 *
 * Copyright 2026 Vladimir Zurita
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use gtk::{gio};

use videoclub_core::settings::AppSettings;

const LANGUAGE_CODES: &[&str] = &["es", "en", "fr", "pt", "de", "it"];
const LANGUAGE_NAMES: &[&str] = &[
    "Spanish (es)",
    "English (en)",
    "French (fr)",
    "Portuguese (pt)",
    "German (de)",
    "Italian (it)",
];

/// Construye y retorna un `adw::PreferencesDialog` con todos sus widgets,
/// listo para presentar. No usa subclassing — todo es procedural.
pub fn build_preferences_dialog() -> adw::PreferencesDialog {
    let dialog = adw::PreferencesDialog::new();
    dialog.set_title("Preferences");

    dialog.add(&build_apis_page());
    dialog.add(&build_general_page());

    dialog
}

// ─── Página: APIs ─────────────────────────────────────────────────────────────

fn build_apis_page() -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    page.set_title("APIs");
    page.set_icon_name(Some("key-symbolic"));

    page.add(&build_omdb_group());
    page.add(&build_opensubtitles_group());

    page
}

fn build_omdb_group() -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title("OMDb (Open Movie Database)");
    group.set_description(Some("Used to fetch movie metadata, posters, and descriptions."));

    let entry = adw::PasswordEntryRow::new();
    entry.set_title("API Key");
    entry.set_text(&AppSettings::new().omdb_api_key());

    entry.connect_changed(|e| {
        AppSettings::new().set_omdb_api_key(&e.text());
    });

    let link_row = adw::ActionRow::new();
    link_row.set_title("Get a free API key");
    link_row.set_subtitle("omdbapi.com/apikey.aspx");
    link_row.set_activatable(true);
    link_row.add_suffix(
        &gtk::Image::builder()
            .icon_name("adw-external-link-symbolic")
            .build(),
    );
    link_row.connect_activated(|_| {
        gtk::UriLauncher::new("https://www.omdbapi.com/apikey.aspx")
            .launch(None::<&gtk::Window>, None::<&gio::Cancellable>, |_| {});
    });

    group.add(&entry);
    group.add(&link_row);
    group
}

fn build_opensubtitles_group() -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title("OpenSubtitles");
    group.set_description(Some("Used to search and download subtitles for your movies."));

    let entry = adw::PasswordEntryRow::new();
    entry.set_title("API Key");
    entry.set_text(&AppSettings::new().opensubtitles_api_key());

    entry.connect_changed(|e| {
        AppSettings::new().set_opensubtitles_api_key(&e.text());
    });

    let link_row = adw::ActionRow::new();
    link_row.set_title("Get a free API key");
    link_row.set_subtitle("opensubtitles.com/consumers");
    link_row.set_activatable(true);
    link_row.add_suffix(
        &gtk::Image::builder()
            .icon_name("adw-external-link-symbolic")
            .build(),
    );
    link_row.connect_activated(|_| {
        gtk::UriLauncher::new("https://www.opensubtitles.com/consumers")
            .launch(None::<&gtk::Window>, None::<&gio::Cancellable>, |_| {});
    });

    group.add(&entry);
    group.add(&link_row);
    group
}

// ─── Página: General ──────────────────────────────────────────────────────────

fn build_general_page() -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();
    page.set_title("General");
    page.set_icon_name(Some("preferences-system-symbolic"));

    let lang_group = adw::PreferencesGroup::new();
    lang_group.set_title("Language");

    lang_group.add(&build_language_row(
        "Subtitle Language",
        AppSettings::new().preferred_subtitle_language(),
        |idx| {
            if let Some(&code) = LANGUAGE_CODES.get(idx) {
                AppSettings::new().set_preferred_subtitle_language(code);
            }
        },
    ));

    lang_group.add(&build_language_row(
        "Audio Language",
        AppSettings::new().preferred_audio_language(),
        |idx| {
            if let Some(&code) = LANGUAGE_CODES.get(idx) {
                AppSettings::new().set_preferred_audio_language(code);
            }
        },
    ));

    page.add(&lang_group);

    // ─── Grupo: Subtítulos ──────────────────────────────────────────────

    let subs_group = adw::PreferencesGroup::new();
    subs_group.set_title("Subtitles Appearance");

    let font_entry = adw::EntryRow::new();
    font_entry.set_title("Font");
    font_entry.set_text(&AppSettings::new().subtitle_font_desc());
    font_entry.connect_changed(|e| {
        let desc = e.text();
        if !desc.is_empty() {
            AppSettings::new().set_subtitle_font_desc(&desc);
        }
    });

    subs_group.add(&font_entry);
    page.add(&subs_group);

    page
}

fn build_language_row(
    title: &str,
    current: String,
    on_change: impl Fn(usize) + 'static,
) -> adw::ComboRow {
    let model = gtk::StringList::new(LANGUAGE_NAMES);
    let row = adw::ComboRow::new();
    row.set_title(title);
    row.set_model(Some(&model));

    if let Some(idx) = LANGUAGE_CODES.iter().position(|&c| c == current) {
        row.set_selected(idx as u32);
    }

    row.connect_selected_notify(move |r| {
        on_change(r.selected() as usize);
    });

    row
}
