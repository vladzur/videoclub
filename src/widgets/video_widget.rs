/* widgets/video_widget.rs
 *
 * Copyright 2026 Vladimir Zurita
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;
use gtk::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, gio};

use crate::player::controller::PlaybackController;
use crate::player::events::PlaybackState;

#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

fn decode_srt_bytes(bytes: &[u8]) -> String {
    // Intentar decodificar como UTF-8 puro primero
    if let Ok(utf8) = std::str::from_utf8(bytes) {
        return utf8.to_string();
    }
    
    // Si falla, es muy probable que sea Latin-1 / Windows-1252 (muy común en subtítulos en español).
    // Para los caracteres del español (á, é, í, ó, ú, ñ, ¿, ¡), ISO-8859-1 mapea directamente
    // a los primeros 256 code points de Unicode, así que una conversión directa byte -> char funciona.
    bytes.iter().map(|&b| b as char).collect()
}

fn parse_srt_time(time_str: &str) -> u64 {
    let clean_str = time_str.replace(',', ".");
    let parts: Vec<&str> = clean_str.split(':').collect();
    if parts.len() == 3 {
        let h: u64 = parts[0].trim().parse().unwrap_or(0);
        let m: u64 = parts[1].trim().parse().unwrap_or(0);
        let s_parts: Vec<&str> = parts[2].trim().split('.').collect();
        let s: u64 = s_parts[0].parse().unwrap_or(0);
        let ms: u64 = if s_parts.len() > 1 { s_parts[1].parse().unwrap_or(0) } else { 0 };
        return (h * 3600 + m * 60 + s) * 1000 + ms;
    }
    0
}

fn parse_srt(contents: &str) -> Vec<SubtitleEntry> {
    let mut entries = Vec::new();
    let blocks = contents.replace("\r\n", "\n");
    for block in blocks.split("\n\n") {
        let mut lines = block.lines();
        let _id = lines.next();
        if let Some(time_line) = lines.next() {
            let parts: Vec<&str> = time_line.split(" --> ").collect();
            if parts.len() == 2 {
                let start = parse_srt_time(parts[0]);
                let end = parse_srt_time(parts[1]);
                let text = lines.collect::<Vec<&str>>().join("\n");
                
                let safe_text = text.replace("<font", "<span").replace("</font>", "</span>");
                entries.push(SubtitleEntry { start_ms: start, end_ms: end, text: safe_text });
            }
        }
    }
    entries
}

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/vladzur/videoclub/video_widget.ui")]
    pub struct VideoWidget {
        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,

        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,

        #[template_child]
        pub controls_revealer: TemplateChild<gtk::Revealer>,

        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub play_icon: TemplateChild<gtk::Image>,

        #[template_child]
        pub progress_scale: TemplateChild<gtk::Scale>,

        #[template_child]
        pub time_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub video_header_bar: TemplateChild<adw::HeaderBar>,

        #[template_child]
        pub fullscreen_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub volume_button: TemplateChild<gtk::ScaleButton>,

        #[template_child]
        pub subtitle_button: TemplateChild<gtk::MenuButton>,

        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,

        pub controller: RefCell<Option<PlaybackController>>,
        pub updating_scale: std::cell::Cell<bool>,
        pub player_window: RefCell<Option<gtk::Window>>,
        pub fullscreen_idle_ticks: std::cell::Cell<u32>,
        pub last_mouse_pos: std::cell::Cell<(f64, f64)>,

        pub video_path: RefCell<String>,
        pub preferred_subtitle_lang: RefCell<String>,
        pub subtitle_paths: RefCell<Vec<String>>,
        pub parsed_subtitles: RefCell<Vec<SubtitleEntry>>,
        pub subtitle_timer: RefCell<Option<gtk::glib::SourceId>>,
        pub current_subtitle_text: RefCell<String>,
        pub subtitle_offset_ms: std::cell::Cell<i64>,
        pub subtitle_font: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VideoWidget {
        const NAME: &'static str = "VideoclubVideoWidget";
        type Type = super::VideoWidget;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }



    impl ObjectImpl for VideoWidget {
        fn constructed(&self) {
            self.parent_constructed();
            
            let obj = self.obj();
            let timer_id = gtk::glib::timeout_add_local(
                std::time::Duration::from_millis(100), 
                glib::clone!(
                    #[weak] obj,
                    #[upgrade_or] gtk::glib::ControlFlow::Break,
                    move || {
                        obj.update_native_subtitles();
                        gtk::glib::ControlFlow::Continue
                    }
                )
            );
            self.subtitle_timer.replace(Some(timer_id));
        }

        fn dispose(&self) {
            if let Some(timer) = self.subtitle_timer.borrow_mut().take() {
                timer.remove();
            }
            if let Some(ctrl) = self.controller.borrow().as_ref() {
                let _ = ctrl.stop();
            }
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }
    impl WidgetImpl for VideoWidget {}
}

glib::wrapper! {
    pub struct VideoWidget(ObjectSubclass<imp::VideoWidget>)
        @extends gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl VideoWidget {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn setup_player(&self, controller: PlaybackController, video_path: &str, movie_title: &str) {
        let imp = self.imp();

        if let Some(paintable) = controller.paintable() {
            imp.picture.set_paintable(Some(&paintable));
        }

        imp.video_path.replace(video_path.to_string());
        imp.controller.replace(Some(controller));

        imp.play_button.connect_clicked(glib::clone!(
            #[weak(rename_to = widget)] self,
            move |_| {
                let imp = widget.imp();
                if let Some(ctrl) = imp.controller.borrow().as_ref() {
                    let _ = ctrl.toggle_play_pause();
                }
                widget.update_play_icon();
            }
        ));

        imp.progress_scale.connect_change_value(glib::clone!(
            #[weak(rename_to = widget)] self,
            #[upgrade_or] glib::Propagation::Proceed,
            move |_, _, value| {
                let imp = widget.imp();
                if !imp.updating_scale.get() {
                    if let Some(ctrl) = imp.controller.borrow().as_ref() {
                        let _ = ctrl.seek_seconds(value);
                    }
                }
                glib::Propagation::Proceed
            }
        ));

        glib::timeout_add_local(Duration::from_millis(500), glib::clone!(
            #[weak(rename_to = widget)] self,
            #[upgrade_or] glib::ControlFlow::Break,
            move || {
                widget.update_position();
                glib::ControlFlow::Continue
            }
        ));

        imp.fullscreen_button.connect_clicked(glib::clone!(
            #[weak(rename_to = widget)] self,
            move |_| {
                widget.toggle_fullscreen();
            }
        ));

        imp.volume_button.set_icons(&[
            "audio-volume-muted-symbolic",
            "audio-volume-high-symbolic",
            "audio-volume-low-symbolic",
            "audio-volume-medium-symbolic",
        ]);
        imp.volume_button.set_value(1.0);
        imp.volume_button.connect_value_changed(glib::clone!(
            #[weak(rename_to = widget)] self,
            move |_, value| {
                widget.set_volume(value);
            }
        ));

        self.populate_subtitle_selector(movie_title);
    }

    fn populate_subtitle_selector(&self, movie_title: &str) {
        let imp = self.imp();
        let video_path = imp.video_path.borrow();
        
        if video_path.is_empty() {
            return;
        }

        let srt_files = scan_subtitle_files(&video_path, movie_title);

        if srt_files.is_empty() {
            imp.subtitle_button.set_visible(false);
            return;
        }

        let action_group = gio::SimpleActionGroup::new();
        self.insert_action_group("subtitle", Some(&action_group));

        let action = gio::SimpleAction::new_stateful(
            "set",
            Some(&String::static_variant_type()),
            &String::new().to_variant(),
        );

        action.connect_activate(glib::clone!(
            #[weak(rename_to = video_widget)] self,
            move |action, parameter| {
                if let Some(variant) = parameter {
                    action.change_state(variant);
                    if let Some(path) = variant.get::<String>() {
                        let imp = video_widget.imp();
                    if path.is_empty() {
                        imp.parsed_subtitles.borrow_mut().clear();
                        imp.subtitle_label.set_visible(false);
                        imp.subtitle_offset_ms.set(0); // Reiniciar offset al desactivar
                    } else {
                        match std::fs::read(&path) {
                            Ok(bytes) => {
                                let contents = decode_srt_bytes(&bytes);
                                let subs = parse_srt(&contents);
                                println!("GTK Subtitles: Parseados {} bloques de '{}'", subs.len(), path);
                                *imp.parsed_subtitles.borrow_mut() = subs;
                                imp.subtitle_offset_ms.set(0); // Reiniciar offset al cargar nuevo archivo
                            }
                            Err(e) => {
                                println!("GTK Subtitles: ERROR fatal al leer srt '{}': {}", path, e);
                            }
                        }
                    }
                }
            }
        }));
        action_group.add_action(&action);

        let delay_add = gio::SimpleAction::new("delay_add", None);
        delay_add.connect_activate(glib::clone!(
            #[weak(rename_to = widget)] self,
            move |_, _| {
                let imp = widget.imp();
                let new_offset = imp.subtitle_offset_ms.get() + 100;
                imp.subtitle_offset_ms.set(new_offset);
                println!("GTK Subtitles: Retraso ajustado a {}ms", new_offset);
            }
        ));
        action_group.add_action(&delay_add);

        let delay_sub = gio::SimpleAction::new("delay_sub", None);
        delay_sub.connect_activate(glib::clone!(
            #[weak(rename_to = widget)] self,
            move |_, _| {
                let imp = widget.imp();
                let new_offset = imp.subtitle_offset_ms.get() - 100;
                imp.subtitle_offset_ms.set(new_offset);
                println!("GTK Subtitles: Retraso ajustado a {}ms", new_offset);
            }
        ));
        action_group.add_action(&delay_sub);

        let delay_reset = gio::SimpleAction::new("delay_reset", None);
        delay_reset.connect_activate(glib::clone!(
            #[weak(rename_to = widget)] self,
            move |_, _| {
                widget.imp().subtitle_offset_ms.set(0);
                println!("GTK Subtitles: Sincronización restablecida (0ms)");
            }
        ));
        action_group.add_action(&delay_reset);

        let menu = gio::Menu::new();
        let section = gio::Menu::new();

        let none_item = gio::MenuItem::new(Some(&gettextrs::gettext("None")), Some("subtitle.set"));
        none_item.set_action_and_target_value(Some("subtitle.set"), Some(&String::new().to_variant()));
        section.append_item(&none_item);

        let preferred = imp.preferred_subtitle_lang.borrow();
        let mut auto_select_path: Option<String> = None;

        for srt in srt_files.iter() {
            let label_text = srt
                .language_code
                .as_deref()
                .and_then(|c| if c.is_empty() { None } else { Some(c) })
                .and_then(crate::preferences_dialog::language_code_to_name)
                .unwrap_or_else(|| srt.filename.clone());

            let item = gio::MenuItem::new(Some(&label_text), Some("subtitle.set"));
            item.set_action_and_target_value(Some("subtitle.set"), Some(&srt.path.to_variant()));
            section.append_item(&item);

            if auto_select_path.is_none() {
                if srt.language_code.as_deref() == Some(&*preferred) {
                    auto_select_path = Some(srt.path.clone());
                } else if preferred.is_empty() {
                    auto_select_path = Some(srt.path.clone());
                }
            }
        }
        
        if auto_select_path.is_none() && !srt_files.is_empty() {
            auto_select_path = Some(srt_files[0].path.clone());
        }

        menu.append_section(None, &section);

        let sync_section = gio::Menu::new();
        let delay_sub_item = gio::MenuItem::new(Some("Adelantar subtítulos (-100ms)"), Some("subtitle.delay_sub"));
        let delay_add_item = gio::MenuItem::new(Some("Atrasar subtítulos (+100ms)"), Some("subtitle.delay_add"));
        let delay_reset_item = gio::MenuItem::new(Some("Restablecer sincronización"), Some("subtitle.delay_reset"));
        sync_section.append_item(&delay_sub_item);
        sync_section.append_item(&delay_add_item);
        sync_section.append_item(&delay_reset_item);
        menu.append_section(Some("Sincronización"), &sync_section);

        imp.subtitle_button.set_menu_model(Some(&menu));

        if let Some(path) = auto_select_path {
            action.activate(Some(&path.to_variant()));
        }

        imp.subtitle_button.set_visible(true);
    }

    fn update_native_subtitles(&self) {
        let imp = self.imp();
        if let Some(ctrl) = imp.controller.borrow().as_ref() {
            if ctrl.state() == PlaybackState::Playing {
                let pos_ms = ctrl.position_seconds() * 1000.0;
                let effective_pos_ms = pos_ms - imp.subtitle_offset_ms.get() as f64;
                
                let subs = imp.parsed_subtitles.borrow();
                let mut active_text = String::new();
                
                for sub in subs.iter() {
                    if effective_pos_ms >= sub.start_ms as f64 && effective_pos_ms <= sub.end_ms as f64 {
                        active_text = sub.text.clone();
                        break;
                    }
                    if sub.start_ms as f64 > effective_pos_ms {
                        break;
                    }
                }
                
                let mut current = imp.current_subtitle_text.borrow_mut();
                if *current != active_text {
                    *current = active_text.clone();
                    if !active_text.is_empty() {
                        let font = imp.subtitle_font.borrow();
                        let markup = format!("<span background=\"#000000A0\" foreground=\"white\" font_desc=\"{}\"><b>{}</b></span>", font, active_text);
                        imp.subtitle_label.set_markup(&markup);
                        imp.subtitle_label.set_visible(true);
                        // Imprime en terminal solo cuando cambia el subtítulo para evitar spam
                        println!("GTK Subtitles -> [{}]", active_text.replace('\n', " | "));
                    } else {
                        imp.subtitle_label.set_visible(false);
                        imp.subtitle_label.set_label("");
                    }
                }
            }
        }
    }

    pub fn set_preferred_subtitle_language(&self, lang: &str) {
        self.imp().preferred_subtitle_lang.replace(lang.to_string());
    }

    pub fn set_subtitle_font(&self, font_desc: &str) {
        self.imp().subtitle_font.replace(font_desc.to_string());
    }

    pub fn on_pointer_motion(&self, x: f64, y: f64) {
        let imp = self.imp();
        let (last_x, last_y) = imp.last_mouse_pos.get();
        if (x - last_x).abs() < 2.0 && (y - last_y).abs() < 2.0 {
            return;
        }
        imp.last_mouse_pos.set((x, y));
        imp.fullscreen_idle_ticks.set(0);
        imp.controls_revealer.set_reveal_child(true);
        self.set_cursor(None);
    }

    fn update_position(&self) {
        let imp = self.imp();
        let borrow = imp.controller.borrow();
        let Some(ctrl) = borrow.as_ref() else { return };

        let pos = ctrl.position_seconds();
        let dur = ctrl.duration_seconds();

        if dur > 0.0 {
            imp.updating_scale.set(true);
            imp.progress_scale.set_range(0.0, dur);
            imp.progress_scale.set_value(pos);
            imp.updating_scale.set(false);
        }

        imp.time_label.set_label(&format!(
            "{} / {}",
            format_time(pos),
            format_time(dur)
        ));

        self.update_play_icon();

        let is_fullscreen = imp.player_window.borrow()
            .as_ref()
            .map(|w| w.is_fullscreen())
            .unwrap_or(false);

        if is_fullscreen {
            let ticks = imp.fullscreen_idle_ticks.get();
            if ticks >= 6 {
                if imp.controls_revealer.reveals_child() {
                    imp.controls_revealer.set_reveal_child(false);
                    self.set_cursor_from_name(Some("none"));
                }
            } else {
                imp.fullscreen_idle_ticks.set(ticks + 1);
            }
        } else {
            imp.fullscreen_idle_ticks.set(0);
        }
    }

    fn update_play_icon(&self) {
        let imp = self.imp();
        let borrow = imp.controller.borrow();
        if let Some(ctrl) = borrow.as_ref() {
            let icon = match ctrl.state() {
                PlaybackState::Playing => "media-playback-pause-symbolic",
                _ => "media-playback-start-symbolic",
            };
            imp.play_button.set_icon_name(icon);
        }
    }

    pub fn set_player_window(&self, window: &gtk::Window) {
        *self.imp().player_window.borrow_mut() = Some(window.clone());
    }

    pub fn clear_player_window(&self) {
        *self.imp().player_window.borrow_mut() = None;
    }

    pub fn stop_playback(&self) {
        let borrow = self.imp().controller.borrow();
        if let Some(ctrl) = borrow.as_ref() {
            let _ = ctrl.stop();
        }
    }

    pub fn set_volume(&self, volume: f64) {
        let borrow = self.imp().controller.borrow();
        if let Some(ctrl) = borrow.as_ref() {
            ctrl.set_volume(volume);
        }
    }

    fn toggle_fullscreen(&self) {
        let borrowed = self.imp().player_window.borrow();
        let Some(window) = borrowed.as_ref() else { return };

        if window.is_fullscreen() {
            window.unfullscreen();
        } else {
            window.fullscreen();
        }
    }

    pub fn update_fullscreen_icon(&self) {
        let imp = self.imp();
        let is_fullscreen = imp.player_window.borrow()
            .as_ref()
            .map(|w| w.is_fullscreen())
            .unwrap_or(false);

        log::debug!("update_fullscreen_icon: is_fullscreen={}", is_fullscreen);

        imp.fullscreen_button.set_icon_name(if is_fullscreen {
            "view-restore-symbolic"
        } else {
            "view-fullscreen-symbolic"
        });

        if is_fullscreen {
            imp.video_header_bar.set_visible(false);
            // Resetear contador para que los controles sean visibles brevemente
            imp.fullscreen_idle_ticks.set(0);
            imp.controls_revealer.set_reveal_child(true);
            self.set_cursor(None);
        } else {
            imp.fullscreen_idle_ticks.set(0);
            imp.controls_revealer.set_reveal_child(true);
            imp.video_header_bar.set_visible(true);
            self.set_cursor(None);
        }
    }
}

impl Default for VideoWidget {
    fn default() -> Self {
        Self::new()
    }
}

/// Archivo de subtítulos encontrado junto a un video.
struct SubtitleFile {
    /// Ruta completa al archivo .srt.
    path: String,
    /// Nombre del archivo (sin ruta) para mostrar.
    filename: String,
    /// Código de idioma extraído del nombre (ej. "es"), o None.
    language_code: Option<String>,
}

/// Escanea el directorio del video en busca de archivos `.srt`
/// con el patrón `{video_stem}.{lang}.srt` o `{movie_title}.{lang}.srt`.
///
/// También detecta archivos `.srt` con cualquier sufijo de idioma.
fn scan_subtitle_files(video_path: &str, movie_title: &str) -> Vec<SubtitleFile> {
    let video = Path::new(video_path);
    let Some(parent) = video.parent() else {
        return Vec::new();
    };
    let stem = video.file_stem().unwrap_or_default().to_string_lossy();

    // Códigos de idioma conocidos (mismos que en preferences_dialog)
    let known_codes = &["es", "en", "fr", "pt", "de", "it"];

    let mut results = Vec::new();
    let mut added_paths = std::collections::HashSet::new();

    // Buscar archivos con el patrón para idiomas conocidos
    for &code in known_codes {
        let mut candidates = vec![parent.join(format!("{}.{}.srt", stem, code))];
        if !movie_title.is_empty() {
            candidates.push(parent.join(format!("{}.{}.srt", movie_title, code)));
        }
        for candidate in candidates {
            if candidate.exists() {
                let path_str = candidate.to_string_lossy().to_string();
                if added_paths.insert(path_str.clone()) {
                    results.push(SubtitleFile {
                        path: path_str,
                        filename: candidate.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        language_code: Some(code.to_string()),
                    });
                }
            }
        }
    }

    // También buscar cualquier otro .srt
    if let Ok(entries) = std::fs::read_dir(parent) {
        let prefix_stem = format!("{}.", stem);
        let prefix_title = if movie_title.is_empty() { String::new() } else { format!("{}.", movie_title) };
        
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".srt") {
                let matches_stem = name.starts_with(&prefix_stem) || name == format!("{}.srt", stem);
                let matches_title = !movie_title.is_empty() && (name.starts_with(&prefix_title) || name == format!("{}.srt", movie_title));
                
                if matches_stem || matches_title {
                    let path_str = entry.path().to_string_lossy().to_string();
                    if added_paths.contains(&path_str) {
                        continue;
                    }
                    
                    let prefix_used = if matches_title && name.starts_with(&prefix_title) {
                        &prefix_title
                    } else if matches_stem && name.starts_with(&prefix_stem) {
                        &prefix_stem
                    } else {
                        ""
                    };
                    
                    let code = if prefix_used.is_empty() {
                        None // Coincidencia exacta sin sufijo de idioma (ej. movie.srt)
                    } else {
                        let middle = &name[prefix_used.len()..];
                        let extracted = middle.strip_suffix(".srt").unwrap_or(middle);
                        if extracted.is_empty() {
                            None
                        } else {
                            Some(extracted.to_string())
                        }
                    };

                    added_paths.insert(path_str.clone());
                    results.push(SubtitleFile {
                        path: path_str,
                        filename: name,
                        language_code: code,
                    });
                }
            }
        }
    }

    results
}

/// Formatea segundos como MM:SS.
fn format_time(secs: f64) -> String {
    let total = secs as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{:02}:{:02}", m, s)
}
