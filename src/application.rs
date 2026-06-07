/* application.rs
 *
 * Copyright 2026 Vladimir Zurita
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gettextrs::gettext;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib};
use log::info;

use crate::config::VERSION;
use crate::preferences_dialog::build_preferences_dialog;
use crate::VideoclubWindow;
use videoclub_core::catalog::MovieCatalog;
use videoclub_core::settings::AppSettings;

mod imp {
    use super::*;

    pub struct VideoclubApplication {
        pub catalog: MovieCatalog,
        pub settings: AppSettings,
    }

    impl Default for VideoclubApplication {
        fn default() -> Self {
            Self {
                catalog: MovieCatalog::new(),
                settings: AppSettings::new(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VideoclubApplication {
        const NAME: &'static str = "VideoclubApplication";
        type Type = super::VideoclubApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for VideoclubApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
        }
    }

    impl ApplicationImpl for VideoclubApplication {
        fn activate(&self) {
            let application = self.obj();

            let window = if let Some(w) = application.active_window() {
                w.downcast::<VideoclubWindow>().unwrap()
            } else {
                let window = VideoclubWindow::new(&*application);

                // Siempre mostramos el catálogo real (vacío al inicio).
                // Si hay directorios guardados en GSettings, los re-escaneamos
                // automáticamente para restaurar el catálogo de la sesión anterior.
                window.set_catalog_store(self.catalog.store(), false);

                let saved_dirs = self.settings.scan_directories();
                if !saved_dirs.is_empty() {
                    info!("Re-escaneando {} directorio(s) guardado(s)", saved_dirs.len());
                    for dir in saved_dirs {
                        window.scan_directory(dir);
                    }
                }

                window
            };

            window.present();
        }

        fn startup(&self) {
            self.parent_startup();

            // Cargar CSS
            let provider = gtk::CssProvider::new();
            provider.load_from_resource("/com/vladzur/videoclub/style.css");
            if let Some(display) = gdk::Display::default() {
                gtk::style_context_add_provider_for_display(
                    &display,
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }

            info!("Aplicación iniciada");
        }
    }

    impl GtkApplicationImpl for VideoclubApplication {}
    impl AdwApplicationImpl for VideoclubApplication {}
}

glib::wrapper! {
    pub struct VideoclubApplication(ObjectSubclass<imp::VideoclubApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl VideoclubApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .property("resource-base-path", "/com/vladzur/videoclub")
            .build()
    }

    /// Devuelve la API key de OMDb configurada en Preferencias (GSettings).
    pub fn omdb_api_key(&self) -> String {
        self.imp().settings.omdb_api_key()
    }

    /// Devuelve la API key de OpenSubtitles configurada en Preferencias (GSettings).
    pub fn opensubtitles_api_key(&self) -> String {
        self.imp().settings.opensubtitles_api_key()
    }

    /// Devuelve el idioma preferido para subtítulos configurado en Preferencias (GSettings).
    pub fn preferred_subtitle_language(&self) -> String {
        self.imp().settings.preferred_subtitle_language()
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        let fullscreen_action = gio::ActionEntry::builder("fullscreen")
            .activate(move |app: &Self, _, _| {
                if let Some(window) = app.active_window() {
                    if window.is_fullscreen() {
                        window.unfullscreen();
                    } else {
                        window.fullscreen();
                    }
                }
            })
            .build();
        let preferences_action = gio::ActionEntry::builder("preferences")
            .activate(move |app: &Self, _, _| app.show_preferences())
            .build();
        self.add_action_entries([quit_action, about_action, fullscreen_action, preferences_action]);
        self.set_accels_for_action("app.quit", &["<control>q"]);
        self.set_accels_for_action("app.fullscreen", &["F11"]);
        self.set_accels_for_action("app.preferences", &["<control>comma"]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name("Videoclub")
            .application_icon("com.vladzur.videoclub")
            .developer_name("Vladimir Zurita")
            .version(VERSION)
            .developers(vec!["Vladimir Zurita"])
            .translator_credits(&gettext("translator-credits"))
            .copyright("© 2026 Vladimir Zurita")
            .build();

        about.present(Some(&window));
    }

    fn show_preferences(&self) {
        let dialog = build_preferences_dialog();
        dialog.present(self.active_window().as_ref());
    }

}
