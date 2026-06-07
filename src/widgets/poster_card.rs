/* widgets/poster_card.rs
 *
 * Copyright 2026 Vladimir Zurita
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::prelude::*;
use adw::prelude::*;
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

        #[template_child]
        pub info_button: TemplateChild<gtk::Button>,

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

        // Usaremos el GtkButton para abrir un AdwDialog de vista rápida.
        // Esto evita los bugs de Wayland/GTK4 con GtkPopover dentro de GtkGridViews.
        imp.info_button.connect_clicked(glib::clone!(
            #[weak(rename_to = card)] self,
            #[weak] movie,
            move |btn| {
                println!("==> Opening Quick Info Dialog for movie: {}", movie.title());

                let dialog = adw::Dialog::new();
                dialog.set_content_width(320);
                
                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);
                vbox.set_margin_top(18);
                vbox.set_margin_bottom(18);
                vbox.set_margin_start(18);
                vbox.set_margin_end(18);

                let title_label = gtk::Label::new(Some(&movie.title()));
                title_label.set_halign(gtk::Align::Start);
                title_label.set_wrap(true);
                let attrs = gtk::pango::AttrList::new();
                attrs.insert(gtk::pango::AttrFloat::new_scale(1.4));
                attrs.insert(gtk::pango::AttrInt::new_weight(gtk::pango::Weight::Bold));
                title_label.set_attributes(Some(&attrs));

                let mut parts = Vec::new();
                if movie.year() > 0 { parts.push(movie.year().to_string()); }
                if !movie.rating().is_empty() { parts.push(format!("⭐ {}", movie.rating())); }
                if !movie.runtime().is_empty() { parts.push(movie.runtime()); }
                if !movie.genre().is_empty() { parts.push(movie.genre()); }
                let meta_label = gtk::Label::new(Some(&parts.join(" • ")));
                meta_label.set_halign(gtk::Align::Start);
                meta_label.set_wrap(true);
                meta_label.add_css_class("dim-label");

                let synopsis_label = gtk::Label::new(Some(&movie.synopsis()));
                synopsis_label.set_halign(gtk::Align::Start);
                synopsis_label.set_wrap(true);
                synopsis_label.set_lines(6);
                synopsis_label.set_ellipsize(gtk::pango::EllipsizeMode::End);

                let more_details_button = gtk::Button::with_label("Full Details");
                more_details_button.set_margin_top(12);
                more_details_button.add_css_class("suggested-action");

                vbox.append(&title_label);
                vbox.append(&meta_label);
                vbox.append(&synopsis_label);
                vbox.append(&more_details_button);
                
                dialog.set_child(Some(&vbox));

                // Si hacen clic en full details, cerramos este dialog rápido y abrimos el completo
                more_details_button.connect_clicked(glib::clone!(
                    #[weak] dialog,
                    #[weak] card,
                    #[weak] movie,
                    move |_| {
                        dialog.close();
                        card.show_full_details_dialog(&movie);
                    }
                ));

                if let Some(window) = btn.root().and_downcast::<gtk::Window>() {
                    dialog.present(Some(&window));
                }
            }
        ));


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

    /// Muestra el diálogo de detalles completos con AdwDialog.
    fn show_full_details_dialog(&self, movie: &MovieObject) {
        let dialog = adw::Dialog::new();
        dialog.set_title(&movie.title());
        dialog.set_content_width(500);
        dialog.set_content_height(600);

        let toolbar_view = adw::ToolbarView::new();
        let header = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header);

        let scroll = gtk::ScrolledWindow::new();
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        let box_layout = gtk::Box::new(gtk::Orientation::Vertical, 16);
        box_layout.set_margin_top(24);
        box_layout.set_margin_bottom(24);
        box_layout.set_margin_start(24);
        box_layout.set_margin_end(24);

        // Título principal
        let title_label = gtk::Label::new(Some(&movie.title()));
        title_label.set_halign(gtk::Align::Start);
        title_label.set_wrap(true);
        title_label.add_css_class("title-1");

        // Metadatos
        let mut meta_parts = Vec::new();
        if movie.year() > 0 { meta_parts.push(movie.year().to_string()); }
        if !movie.rating().is_empty() { meta_parts.push(format!("⭐ {}", movie.rating())); }
        if !movie.runtime().is_empty() { meta_parts.push(movie.runtime()); }
        if !movie.genre().is_empty() { meta_parts.push(movie.genre()); }
        
        let meta_label = gtk::Label::new(Some(&meta_parts.join(" • ")));
        meta_label.set_halign(gtk::Align::Start);
        meta_label.set_wrap(true);
        meta_label.add_css_class("dim-label");

        // Sinopsis completa
        let synopsis_label = gtk::Label::new(Some(&movie.synopsis()));
        synopsis_label.set_halign(gtk::Align::Start);
        synopsis_label.set_wrap(true);
        synopsis_label.set_selectable(true);

        box_layout.append(&title_label);
        box_layout.append(&meta_label);
        
        let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
        box_layout.append(&sep);

        box_layout.append(&synopsis_label);

        scroll.set_child(Some(&box_layout));
        toolbar_view.set_content(Some(&scroll));
        dialog.set_child(Some(&toolbar_view));

        if let Some(root) = self.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
            dialog.present(Some(&root));
        }
    }
}

impl Default for PosterCard {
    fn default() -> Self {
        Self::new()
    }
}
