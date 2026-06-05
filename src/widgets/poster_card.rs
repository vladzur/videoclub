/* widgets/poster_card.rs
 *
 * Copyright 2026 Vladimir Zurita
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib};

use videoclub_core::movie::MovieObject;

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/vladzur/videoclub/poster_card.ui")]
    pub struct PosterCard {
        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,

        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub subtitle_badge: TemplateChild<gtk::Image>,

        #[template_child]
        pub year_label: TemplateChild<gtk::Label>,

        /// Guardamos los bindings para poder limpiarlos en unbind().
        pub bindings: RefCell<Vec<glib::Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PosterCard {
        const NAME: &'static str = "VideoclubPosterCard";
        type Type = super::PosterCard;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PosterCard {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }
    impl WidgetImpl for PosterCard {}
}

glib::wrapper! {
    pub struct PosterCard(ObjectSubclass<imp::PosterCard>)
        @extends gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PosterCard {
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Vincula un `MovieObject` a este widget y guarda los bindings.
    pub fn bind(&self, movie: &MovieObject) {
        let imp = self.imp();
        let mut bindings = imp.bindings.borrow_mut();

        // Vincular título
        let b1 = movie.bind_property("title", &*imp.title_label, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();

        // Vincular año (0 = desconocido → cadena vacía)
        let b2 = movie.bind_property("year", &*imp.year_label, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .transform_to(|_, year: i32| -> Option<String> {
                if year > 0 { Some(year.to_string()) } else { Some(String::new()) }
            })
            .build();

        // Badge de subtítulos
        let b3 = movie.bind_property("subtitles-ready", &*imp.subtitle_badge, "visible")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();

        bindings.push(b1);
        bindings.push(b2);
        bindings.push(b3);

        // Cargar póster reactivamente: se recarga cuando poster-path cambia
        let _b4 = movie.connect_notify_local(
            Some("poster-path"),
            glib::clone!(
                #[weak(rename_to = card)] self,
                move |movie, _| {
                    card.reload_poster(movie);
                }
            ),
        );
        // Cargar póster inicial
        self.reload_poster(movie);
    }

    /// Limpia los bindings (llamado por la fábrica al reciclar la celda).
    pub fn unbind(&self) {
        let imp = self.imp();
        for binding in imp.bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
        // Resetear a placeholder
        self.set_placeholder();
    }

    /// Carga el póster del MovieObject o muestra el placeholder.
    fn reload_poster(&self, movie: &MovieObject) {
        let poster = movie.poster_path();
        if !poster.is_empty() {
            let file = gio::File::for_path(&poster);
            if let Ok(texture) = gdk::Texture::from_file(&file) {
                self.imp().picture.set_paintable(Some(&texture));
                return;
            }
        }
        self.set_placeholder();
    }

    /// Muestra la imagen de placeholder del recurso bundleado.
    fn set_placeholder(&self) {
        // set_resource() falla silenciosamente con SVG en GTK4.
        // set_file() con URI "resource:///" usa el pipeline completo
        // de GdkPixbuf (incluyendo librsvg) y es la API recomendada.
        let file = gio::File::for_uri(
            "resource:///com/vladzur/videoclub/poster_placeholder.svg"
        );
        self.imp().picture.set_file(Some(&file));
    }
}

impl Default for PosterCard {
    fn default() -> Self {
        Self::new()
    }
}
