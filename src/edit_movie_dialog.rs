/* edit_movie_dialog.rs
 *
 * Copyright 2026 Vladimir Zurita
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::path::Path;

use gtk::prelude::*;
use gtk::glib;
use adw::prelude::*;
use glib::subclass::prelude::*;
use gettextrs::gettext;

use videoclub_core::movie::MovieObject;
use videoclub_core::settings::AppSettings;
use crate::window::VideoclubWindow;

/// Construye y devuelve el diálogo de edición de metadatos de una película.
///
/// Layout:
///   ┌─ Edit Movie ───────────────────────────────┐
///   │ ── Search ──────────────────────────────── │
///   │  Title  [______________________]           │
///   │  Year   [____]  [Fetch from OMDb]          │
///   ├────────────────────────────────────────────┤
///   │ ── Metadata ────────────────────────────── │
///   │  Title    [______________________]         │
///   │  Year     [____]                           │
///   │  Synopsis [______________________]         │
///   │           [______________________]         │
///   ├────────────────────────────────────────────┤
///   │                       [Cancel]  [Save]     │
///   └────────────────────────────────────────────┘
pub fn build_edit_movie_dialog(
    movie: &MovieObject,
    window: &VideoclubWindow,
) -> adw::Dialog {
    let dialog = adw::Dialog::new();
    dialog.set_title(&gettext("Edit — {}").replace("{}", &movie.title()));
    dialog.set_content_width(480);

    // ── Contenido principal ───────────────────────────────────────────────
    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);

    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content_box.set_margin_top(16);
    content_box.set_margin_bottom(16);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);

    // ── Grupo: información del archivo ───────────────────────────────────────
    let file_group = adw::PreferencesGroup::new();
    file_group.set_title(&gettext("File Info"));

    // Nombre real del archivo de video
    let video_path = movie.video_path();
    let video_filename = Path::new(&video_path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("—");
    let filename_row = adw::ActionRow::new();
    filename_row.set_title(&gettext("Video File"));
    let filename_label = gtk::Label::new(Some(video_filename));
    filename_label.set_selectable(true);
    filename_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    filename_label.add_css_class("dim-label");
    filename_row.add_suffix(&filename_label);
    file_group.add(&filename_row);

    // Archivo de subtítulos (si existe)
    let sub_info = resolve_subtitle_file(&video_path, &movie.title());
    let subtitle_row = adw::ActionRow::new();
    subtitle_row.set_title(&gettext("Subtitles"));
    let subtitle_label = gtk::Label::new(Some(&sub_info));
    subtitle_label.set_selectable(true);
    subtitle_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    subtitle_label.add_css_class(if sub_info.starts_with("✓") { "accent" } else { "dim-label" });
    subtitle_row.add_suffix(&subtitle_label);
    file_group.add(&subtitle_row);

    // Botón para descargar subtítulos
    let download_subs_btn = gtk::Button::with_label(&gettext("Download Subtitles"));
    download_subs_btn.set_halign(gtk::Align::End);
    download_subs_btn.set_margin_top(4);
    download_subs_btn.set_sensitive(!movie.subtitles_ready());
    content_box.append(&download_subs_btn);

    content_box.append(&file_group);

    // ── Separador ─────────────────────────────────────────────────────────
    let sep0 = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep0.set_margin_top(12);
    sep0.set_margin_bottom(4);
    content_box.append(&sep0);

    // ── Grupo: parámetros de búsqueda ─────────────────────────────────────
    let search_group = adw::PreferencesGroup::new();
    search_group.set_title(&gettext("Search Parameters"));
    search_group.set_description(
        Some(&gettext("These values are sent to OMDb. Edit them to refine the search."))
    );

    let search_title_row = adw::EntryRow::new();
    search_title_row.set_title(&gettext("Search Title"));
    search_title_row.set_text(&movie.title());
    search_group.add(&search_title_row);

    let search_year_row = adw::EntryRow::new();
    search_year_row.set_title(&gettext("Search Year"));
    if movie.year() > 0 {
        search_year_row.set_text(&movie.year().to_string());
    }
    search_group.add(&search_year_row);

    // Botón "Fetch from OMDb"
    let fetch_btn = gtk::Button::with_label(&gettext("Fetch from OMDb"));
    fetch_btn.set_halign(gtk::Align::End);
    fetch_btn.add_css_class("suggested-action");
    fetch_btn.set_margin_top(8);

    content_box.append(&search_group);
    content_box.append(&fetch_btn);

    // ── Separador ─────────────────────────────────────────────────────────
    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.set_margin_top(16);
    sep.set_margin_bottom(16);
    content_box.append(&sep);

    // ── Grupo: metadatos almacenados ──────────────────────────────────────
    let meta_group = adw::PreferencesGroup::new();
    meta_group.set_title(&gettext("Stored Metadata"));
    meta_group.set_description(
        Some(&gettext("Editable directly. Press Save to persist without re-fetching."))
    );

    let title_row = adw::EntryRow::new();
    title_row.set_title(&gettext("Title"));
    title_row.set_text(&movie.title());
    meta_group.add(&title_row);

    let year_row = adw::EntryRow::new();
    year_row.set_title(&gettext("Year"));
    if movie.year() > 0 {
        year_row.set_text(&movie.year().to_string());
    }
    meta_group.add(&year_row);

    let synopsis_row = adw::EntryRow::new();
    synopsis_row.set_title(&gettext("Synopsis"));
    synopsis_row.set_text(&movie.synopsis());
    meta_group.add(&synopsis_row);

    // IMDb ID (solo lectura para referencia)
    let imdb_row = adw::ActionRow::new();
    imdb_row.set_title(&gettext("IMDb ID"));
    let imdb_value = gtk::Label::new(Some(&movie.imdb_id()));
    imdb_value.add_css_class("dim-label");
    imdb_row.add_suffix(&imdb_value);
    meta_group.add(&imdb_row);

    content_box.append(&meta_group);

    // ── Fila de botones ───────────────────────────────────────────────────
    let btn_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    btn_row.set_halign(gtk::Align::End);
    btn_row.set_margin_top(16);

    let cancel_btn = gtk::Button::with_label(&gettext("Cancel"));
    let save_btn = gtk::Button::with_label(&gettext("Save"));
    save_btn.add_css_class("suggested-action");

    btn_row.append(&cancel_btn);
    btn_row.append(&save_btn);
    content_box.append(&btn_row);

    toolbar_view.set_content(Some(&content_box));
    dialog.set_child(Some(&toolbar_view));

    // ── Conexiones ────────────────────────────────────────────────────────

    // Cancel
    cancel_btn.connect_clicked(glib::clone!(
        #[weak] dialog,
        move |_| { dialog.close(); }
    ));

    // Fetch from OMDb: actualiza los parámetros de búsqueda en el store y re-enriquece
    fetch_btn.connect_clicked(glib::clone!(
        #[weak] movie,
        #[weak] window,
        #[weak] dialog,
        #[weak] search_title_row,
        #[weak] search_year_row,
        #[weak] title_row,
        #[weak] year_row,
        #[weak] synopsis_row,
        #[weak] imdb_row,
        move |_| {
            let search_title = search_title_row.text().to_string();
            let search_year = search_year_row.text().to_string()
                .parse::<i32>().ok();

            // Guardar overrides en el store antes de enriquecer
            {
                let imp = window.imp();
                let mut store = imp.metadata_store.borrow_mut();
                store.set_search_params(&movie.video_path(), &search_title, search_year);
            }

            // Limpiar has_metadata para forzar re-fetch
            {
                let imp = window.imp();
                imp.metadata_store.borrow_mut().clear_metadata(&movie.video_path());
            }
            movie.set_has_metadata(false);

            // Lanzar enriquecimiento
            window.enrich_single_movie(&movie);

            // Actualizar campos del dialog cuando cambien las propiedades del movie
            // (el enriquecimiento es async, cuando termina notifica via GObject properties)
            movie.connect_notify_local(Some("title"), glib::clone!(
                #[weak] title_row,
                move |m, _| title_row.set_text(&m.title())
            ));
            movie.connect_notify_local(Some("year"), glib::clone!(
                #[weak] year_row,
                move |m, _| {
                    if m.year() > 0 { year_row.set_text(&m.year().to_string()); }
                }
            ));
            movie.connect_notify_local(Some("synopsis"), glib::clone!(
                #[weak] synopsis_row,
                move |m, _| synopsis_row.set_text(&m.synopsis())
            ));
            movie.connect_notify_local(Some("imdb-id"), glib::clone!(
                #[weak] imdb_row,
                move |m, _| {
                    // Re-build the label since ActionRow suffix doesn't have a direct text setter
                    let _ = imdb_row.title(); // keep reference alive
                    imdb_row.set_title(&m.imdb_id());
                }
            ));

            dialog.close();
        }
    ));

    // Download Subtitles: descarga subtítulos para esta película
    download_subs_btn.connect_clicked(glib::clone!(
        #[weak] movie,
        #[weak] window,
        #[weak] download_subs_btn,
        #[weak] subtitle_label,
        move |_| {
            download_subs_btn.set_sensitive(false);
            download_subs_btn.set_label(&gettext("Downloading…"));

            window.download_subtitles_single(&movie, glib::clone!(
                #[weak] movie,
                #[weak] download_subs_btn,
                #[weak] subtitle_label,
                move |success| {
                    if success {
                        let video_path = movie.video_path();
                        let info = resolve_subtitle_file(&video_path, &movie.title());
                        subtitle_label.set_text(&info);
                        subtitle_label.remove_css_class("dim-label");
                        subtitle_label.add_css_class("accent");
                        download_subs_btn.set_sensitive(false);
                        download_subs_btn.set_label(&gettext("Downloaded ✓"));
                    } else {
                        download_subs_btn.set_label(&gettext("Retry"));
                        download_subs_btn.set_sensitive(true);
                    }
                }
            ));
        }
    ));

    // Save: persiste los campos editados manualmente en el store y MovieObject
    save_btn.connect_clicked(glib::clone!(
        #[weak] movie,
        #[weak] window,
        #[weak] dialog,
        #[weak] title_row,
        #[weak] year_row,
        #[weak] synopsis_row,
        #[weak] search_title_row,
        #[weak] search_year_row,
        move |_| {
            let new_title = title_row.text().to_string();
            let new_year = year_row.text().to_string().parse::<i32>().unwrap_or(0);
            let new_synopsis = synopsis_row.text().to_string();

            let new_search_title = search_title_row.text().to_string();
            let new_search_year = search_year_row.text().to_string().parse::<i32>().ok();

            // Actualizar MovieObject
            movie.set_title(new_title.as_str());
            movie.set_year(new_year);
            movie.set_synopsis(new_synopsis.as_str());
            movie.set_has_metadata(true);

            // Persistir en el store
            {
                let imp = window.imp();
                let mut store = imp.metadata_store.borrow_mut();
                let video_path = movie.video_path();
                let existing = store.get(&video_path).cloned();

                let updated = videoclub_core::metadata_store::StoredMetadata {
                    search_title: if !new_search_title.is_empty() {
                        new_search_title
                    } else {
                        existing.as_ref().map(|e| e.search_title.clone()).unwrap_or_else(|| new_title.clone())
                    },
                    search_year: new_search_year.or_else(|| existing.as_ref().and_then(|e| e.search_year)),
                    title: Some(new_title),
                    year: if new_year > 0 { Some(new_year) } else { None },
                    synopsis: if new_synopsis.is_empty() { None } else { Some(new_synopsis) },
                    poster_path: existing.as_ref().and_then(|e| e.poster_path.clone()),
                    imdb_id: {
                        let id = movie.imdb_id();
                        if id.is_empty() { None } else { Some(id) }
                    },
                    imdb_rating: existing.as_ref().and_then(|e| e.imdb_rating.clone()),
                    genre: existing.as_ref().and_then(|e| e.genre.clone()),
                    runtime: existing.as_ref().and_then(|e| e.runtime.clone()),
                    has_metadata: true,
                    subtitle_path: existing.as_ref().and_then(|e| e.subtitle_path.clone()),
                    last_position: existing.as_ref().and_then(|e| e.last_position),
                };
                store.upsert(&video_path, updated);
                store.save();
            }

            dialog.close();
        }
    ));

    dialog
}

/// Busca el archivo de subtítulos para un video y devuelve una cadena descriptiva.
///
/// Prioriza el idioma preferido configurado en Ajustes.
/// Devuelve `"✓ subtitles.srt"` si se encuentra, o `"Not available"` si no.
fn resolve_subtitle_file(video_path: &str, movie_title: &str) -> String {
    let path = Path::new(video_path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let lang = AppSettings::new().preferred_subtitle_language();

    // Intentar con el idioma preferido
    let mut candidates = vec![parent.join(format!("{}.{}.srt", stem, lang))];
    if !movie_title.is_empty() {
        candidates.push(parent.join(format!("{}.{}.srt", movie_title, lang)));
    }
    
    for candidate in candidates {
        if candidate.exists() {
            return format!("✓ {}", candidate.file_name().unwrap().to_string_lossy());
        }
    }

    // Fallback: buscar cualquier .srt con el mismo stem o title
    if let Ok(entries) = std::fs::read_dir(parent) {
        let prefix_stem = format!("{}.", stem);
        let prefix_title = if movie_title.is_empty() { String::new() } else { format!("{}.", movie_title) };
        
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".srt") {
                if name_str.starts_with(&prefix_stem) || name_str == format!("{}.srt", stem) ||
                   (!movie_title.is_empty() && (name_str.starts_with(&prefix_title) || name_str == format!("{}.srt", movie_title))) {
                    return format!("✓ {}", name_str);
                }
            }
        }
    }

    gettext("Not available")
}
