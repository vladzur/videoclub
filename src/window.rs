/* window.rs
 *
 * Copyright 2026 Vladimir Zurita
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::cell::{Cell, RefCell};

use gtk::prelude::*;

use adw::subclass::prelude::*;
use adw::prelude::AdwDialogExt;
use gtk::{gdk, gio, glib};
use gettextrs::gettext;

use videoclub_core::{error, info, warn};
use videoclub_core::metadata_store::{MetadataStore, StoredMetadata};
use videoclub_core::movie::MovieObject;
use crate::player::controller::PlaybackController;
use crate::widgets::video_widget::VideoWidget;

/// Modo de ordenamiento del catálogo de películas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    /// Sin ordenamiento (orden de escaneo original).
    None,
    /// Orden alfabético A–Z.
    TitleAsc,
    /// Orden alfabético Z–A.
    TitleDesc,
    /// Año más reciente primero.
    YearDesc,
    /// Año más antiguo primero.
    YearAsc,
}

mod movie_sorter {
    use super::SortMode;
    use gtk::glib;
    use gtk::glib::prelude::*;
    use gtk::glib::subclass::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::Ordering;
    use videoclub_core::movie::MovieObject;

    mod imp {
        use super::*;
        use std::cell::RefCell;

        #[derive(Debug)]
        pub struct MovieSorter {
            pub sort_mode: RefCell<SortMode>,
        }

        impl Default for MovieSorter {
            fn default() -> Self {
                Self {
                    sort_mode: RefCell::new(SortMode::default()),
                }
            }
        }

        #[glib::object_subclass]
        impl ObjectSubclass for MovieSorter {
            const NAME: &'static str = "VideoclubMovieSorter";
            type Type = super::MovieSorter;
            type ParentType = gtk::Sorter;
        }

        impl ObjectImpl for MovieSorter {}

        impl SorterImpl for MovieSorter {
            fn compare(&self, item1: &glib::Object, item2: &glib::Object) -> Ordering {
                let movie1 = item1.downcast_ref::<MovieObject>();
                let movie2 = item2.downcast_ref::<MovieObject>();

                let (movie1, movie2) = match (movie1, movie2) {
                    (Some(m1), Some(m2)) => (m1, m2),
                    _ => return Ordering::Equal,
                };

                let has1 = movie1.has_metadata();
                let has2 = movie2.has_metadata();

                // Películas sin metadatos siempre al final
                if has1 && !has2 {
                    return Ordering::Smaller;
                }
                if !has1 && has2 {
                    return Ordering::Larger;
                }

                let mode = *self.sort_mode.borrow();

                match mode {
                    SortMode::None => Ordering::Equal,
                    SortMode::TitleAsc => {
                        let t1 = movie1.title().to_lowercase();
                        let t2 = movie2.title().to_lowercase();
                        t1.cmp(&t2).into()
                    }
                    SortMode::TitleDesc => {
                        let t1 = movie1.title().to_lowercase();
                        let t2 = movie2.title().to_lowercase();
                        t2.cmp(&t1).into()
                    }
                    SortMode::YearDesc => {
                        let y1 = movie1.year();
                        let y2 = movie2.year();
                        y2.cmp(&y1).into()
                    }
                    SortMode::YearAsc => {
                        let y1 = movie1.year();
                        let y2 = movie2.year();
                        y1.cmp(&y2).into()
                    }
                }
            }

            fn order(&self) -> gtk::SorterOrder {
                gtk::SorterOrder::Total
            }
        }
    }

    glib::wrapper! {
        pub struct MovieSorter(ObjectSubclass<imp::MovieSorter>)
            @extends gtk::Sorter;
    }

    impl MovieSorter {
        pub fn new(mode: SortMode) -> Self {
            let sorter: Self = glib::Object::new();
            sorter.imp().sort_mode.replace(mode);
            sorter
        }
    }
}

impl Default for SortMode {
    fn default() -> Self {
        SortMode::None
    }
}

use movie_sorter::MovieSorter;

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate, Default)]
    #[template(resource = "/com/vladzur/videoclub/window.ui")]
    pub struct VideoclubWindow {
        #[template_child]
        pub movie_grid: TemplateChild<gtk::GridView>,

        #[template_child]
        pub content_stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,

        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        #[template_child]
        pub search_button: TemplateChild<gtk::ToggleButton>,

        #[template_child]
        pub sort_button: TemplateChild<gtk::MenuButton>,

        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,

        #[template_child]
        pub add_folder_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub refresh_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub empty_add_folder_button: TemplateChild<gtk::Button>,

        /// El ListStore del catálogo (asignado desde la Application).
        pub catalog_store: RefCell<Option<gio::ListStore>>,

        /// El filtro de texto para la búsqueda.
        pub string_filter: RefCell<Option<gtk::StringFilter>>,

        /// Modelo de ordenamiento (envuelve el FilterListModel).
        pub sort_model: RefCell<Option<gtk::SortListModel>>,

        /// True mientras se muestran datos de prueba (antes del primer escaneo real).
        pub showing_test_data: Cell<bool>,

        /// Si es true, se lanza enrich_all_movies al terminar scan_directory.
        /// Activado por Refresh y Add Folder. Desactivado por defecto (no fetch al iniciar).
        pub enrich_after_scan: Cell<bool>,

        /// Referencia fuerte a la ventana del reproductor activa (singleton).
        pub active_video_window: RefCell<Option<gtk::Window>>,

        /// Referencia fuerte al VideoWidget activo.
        /// Necesaria para que los WeakRef en sus closures no expiren antes de tiempo.
        pub active_video_widget: RefCell<Option<VideoWidget>>,

        /// Store persistente de metadatos de la biblioteca.
        pub metadata_store: RefCell<MetadataStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VideoclubWindow {
        const NAME: &'static str = "VideoclubWindow";
        type Type = super::VideoclubWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VideoclubWindow {
        fn constructed(&self) {
            self.parent_constructed();
            // Cargar el store de metadatos al inicio
            *self.metadata_store.borrow_mut() = MetadataStore::load();
            let obj = self.obj();
            obj.setup_search_bar();
            obj.setup_sort_button();
            obj.setup_video_drop_target();
            obj.connect_folder_buttons();
        }
    }

    impl WidgetImpl for VideoclubWindow {}
    impl WindowImpl for VideoclubWindow {}
    impl ApplicationWindowImpl for VideoclubWindow {}
    impl AdwApplicationWindowImpl for VideoclubWindow {}
}

glib::wrapper! {
    pub struct VideoclubWindow(ObjectSubclass<imp::VideoclubWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl VideoclubWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    // ─── Catálogo ────────────────────────────────────────────────────────────

    /// Asigna el ListStore del catálogo y conecta la grilla.
    /// Llamado desde `VideoclubApplication::activate()`.
    pub fn set_catalog_store(&self, store: &gio::ListStore, is_test_data: bool) {
        let imp = self.imp();
        imp.catalog_store.replace(Some(store.clone()));
        imp.showing_test_data.set(is_test_data);

        // Filtro de texto sobre la propiedad "title"
        let title_expr = gtk::PropertyExpression::new(
            MovieObject::static_type(),
            None::<&gtk::Expression>,
            "title",
        );
        let filter = gtk::StringFilter::new(Some(title_expr));
        filter.set_match_mode(gtk::StringFilterMatchMode::Substring);
        filter.set_ignore_case(true);
        imp.string_filter.replace(Some(filter.clone()));

        // Cadena: ListStore → FilterListModel → SortListModel → SingleSelection → GridView
        let filter_model = gtk::FilterListModel::new(
            Some(store.clone()),
            Some(filter),
        );
        let sort_model = gtk::SortListModel::new(
            Some(filter_model),
            None::<gtk::Sorter>,
        );
        imp.sort_model.replace(Some(sort_model.clone()));
        let selection = gtk::SingleSelection::new(Some(sort_model));
        imp.movie_grid.set_model(Some(&selection));

        // Fábrica de PosterCards
        self.setup_factory();

        // Señal de activación (clic en tarjeta)
        imp.movie_grid.connect_activate(glib::clone!(
            #[weak(rename_to = win)] self,
            move |grid, pos| {
                let model = grid.model().unwrap();
                if let Some(obj) = model.item(pos) {
                    if let Ok(movie) = obj.downcast::<MovieObject>() {
                        win.open_player(&movie);
                    }
                }
            }
        ));

        self.update_content_stack();
    }

    /// Actualiza el stack según si hay películas o no.
    pub fn update_content_stack(&self) {
        let imp = self.imp();
        let has_items = imp.catalog_store.borrow()
            .as_ref()
            .map(|s| s.n_items() > 0)
            .unwrap_or(false);

        imp.content_stack.set_visible_child_name(if has_items { "catalog" } else { "empty" });
    }

    // ─── Fábrica de tarjetas ──────────────────────────────────────────────────

    fn setup_factory(&self) {
        let imp = self.imp();

        let factory = gtk::SignalListItemFactory::new();

        // setup: crear el widget vacío
        factory.connect_setup(|_, list_item| {
            let item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let card = crate::widgets::poster_card::PosterCard::new();
            item.set_child(Some(&card));
        });

        // bind: vincular MovieObject a la PosterCard y añadir right-click
        factory.connect_bind(glib::clone!(
            #[weak(rename_to = win)] self,
            move |_, list_item| {
                let item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                let movie = item.item()
                    .and_downcast::<MovieObject>()
                    .expect("El item debe ser MovieObject");
                let card = item.child()
                    .and_downcast::<crate::widgets::poster_card::PosterCard>()
                    .expect("El child debe ser PosterCard");
                card.bind(&movie);

                // Gesto de click secundario (right-click) → menú contextual
                let gesture = gtk::GestureClick::new();
                gesture.set_button(3);
                gesture.connect_released(glib::clone!(
                    #[weak] win,
                    #[weak] movie,
                    #[weak] card,
                    move |gesture, _, x, y| {
                        gesture.set_state(gtk::EventSequenceState::Claimed);
                        win.show_movie_context_menu(&movie, &card, x, y);
                    }
                ));
                card.add_controller(gesture);
            }
        ));

        // unbind: limpiar bindings al reciclar la celda
        factory.connect_unbind(|_, list_item| {
            let item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            if let Some(card) = item.child().and_downcast::<crate::widgets::poster_card::PosterCard>() {
                card.unbind();
            }
        });

        imp.movie_grid.set_factory(Some(&factory));
    }

    /// Muestra el menú contextual (click derecho) sobre la tarjeta de una película.
    fn show_movie_context_menu(
        &self,
        movie: &MovieObject,
        card: &crate::widgets::poster_card::PosterCard,
        x: f64,
        y: f64,
    ) {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.set_margin_top(4);
        vbox.set_margin_bottom(4);
        vbox.set_margin_start(4);
        vbox.set_margin_end(4);

        let edit_btn = gtk::Button::new();
        let edit_label = gtk::Label::new(Some(&gettext("Edit Metadata")));
        edit_label.set_halign(gtk::Align::Start);
        edit_btn.set_child(Some(&edit_label));
        edit_btn.set_has_frame(false);

        let popover = gtk::Popover::new();
        popover.set_child(Some(&vbox));
        popover.set_parent(card);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));

        edit_btn.connect_clicked(glib::clone!(
            #[weak(rename_to = win)] self,
            #[weak] movie,
            #[weak] popover,
            move |_| {
                popover.popdown();
                win.open_edit_movie_dialog(&movie);
            }
        ));

        vbox.append(&edit_btn);
        popover.popup();
    }

    /// Abre el diálog de edición de metadatos para una película.
    fn open_edit_movie_dialog(&self, movie: &MovieObject) {
        let dialog = crate::edit_movie_dialog::build_edit_movie_dialog(
            movie,
            self,
        );
        dialog.present(Some(self));
    }

    // ─── Reproductor ─────────────────────────────────────────────────────────

    /// Abre el reproductor (singleton). Cierra el anterior si ya hay uno abierto.
    fn open_player(&self, movie: &MovieObject) {
        let path = movie.video_path();
        if path.is_empty() {
            warn!("La película '{}' no tiene ruta de video", movie.title());
            return;
        }

        // ── Singleton: cerrar reproductor anterior si existe ────────────────
        // IMPORTANTE: clonar el ref fuera del borrow ANTES de llamar close(),
        // porque close() dispara close-request sincrónicamente y ese handler
        // necesita borrow_mut() sobre el mismo RefCell.
        let existing = self.imp().active_video_window.borrow().clone();
        if let Some(win) = existing {
            win.close();
        }

        let controller = match PlaybackController::new() {
            Ok(c) => c,
            Err(e) => { error!("PlaybackController: {}", e); return; }
        };
        if let Err(e) = controller.load(&path) {
            error!("Error cargando '{}': {}", path, e); return;
        }

        let video_widget = VideoWidget::new();

        // Aplicar la fuente de subtítulos configurada en Preferencias a nuestra capa GTK nativa
        if let Some(app) = self.application() {
            if let Ok(app) = app.downcast::<crate::application::VideoclubApplication>() {
                let font = app.imp().settings.subtitle_font_desc();
                video_widget.set_subtitle_font(&font);
            }
        }

        video_widget.set_hexpand(true);
        video_widget.set_vexpand(true);
        let _ = controller.play();
        // Pasar idioma preferido ANTES de setup_player para auto-selección
        if let Some(app) = self.application() {
            if let Ok(app) = app.downcast::<crate::application::VideoclubApplication>() {
                let lang = app.imp().settings.preferred_subtitle_language();
                video_widget.set_preferred_subtitle_language(&lang);
            }
        }

        video_widget.setup_player(controller, &path, &movie.title());

        // ── adw::Window: integración correcta con libadwaita ──────────────────
        // gtk::Window + AdwHeaderBar = doble frame (CSD propio del WM + AdwHeaderBar)
        // adw::Window = sin doble frame; AdwToolbarView/AdwHeaderBar integran nativamente
        // La referencia fuerte en active_video_window mantiene viva la ventana.
        let video_window = adw::Window::builder()
            .title(movie.title())
            .default_width(1280)
            .default_height(720)
            .content(&video_widget)
            .build();

        // Pasar referencia al widget para que el botón fullscreen funcione
        video_widget.set_player_window(video_window.upcast_ref::<gtk::Window>());

        // Guardar referencias fuertes (ventana + widget) para mantenerlos vivos
        self.imp().active_video_window.replace(Some(video_window.clone().upcast::<gtk::Window>()));
        self.imp().active_video_widget.replace(Some(video_widget.clone()));

        // Detener pipeline y limpiar referencias al cerrar
        video_window.connect_close_request(glib::clone!(
            #[weak] video_widget,
            #[weak(rename_to = catalog)] self,
            #[upgrade_or] glib::Propagation::Proceed,
            move |_| {
                video_widget.stop_playback();         // ← detener GStreamer
                video_widget.clear_player_window();   // ← romper ciclo de referencia
                catalog.imp().active_video_widget.replace(None);
                catalog.imp().active_video_window.replace(None);
                glib::Propagation::Proceed
            }
        ));

        // Conectar botón fullscreen directamente con referencia fuerte a video_window
        video_widget.imp().fullscreen_button.connect_clicked(glib::clone!(
            #[strong] video_window,
            #[weak] video_widget,
            move |_| {
                if video_window.is_fullscreen() {
                    video_window.unfullscreen();
                } else {
                    video_window.fullscreen();
                }
                video_widget.update_fullscreen_icon();
            }
        ));

        // Sincronizar icono cuando el WM confirma el cambio de fullscreen
        video_window.connect_notify_local(
            Some("fullscreened"),
            glib::clone!(
                #[weak] video_widget,
                move |_, _| { video_widget.update_fullscreen_icon(); }
            ),
        );

        // Atajo F11
        let key_ctrl = gtk::EventControllerKey::new();
        key_ctrl.connect_key_pressed(glib::clone!(
            #[strong] video_window,
            move |_, key, _, _| {
                if key == gtk::gdk::Key::F11 {
                    if video_window.is_fullscreen() { video_window.unfullscreen(); }
                    else { video_window.fullscreen(); }
                    return glib::Propagation::Stop;
                }
                if key == gtk::gdk::Key::Escape && video_window.is_fullscreen() {
                    video_window.unfullscreen();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        video_window.add_controller(key_ctrl);

        // Motion controller en la VENTANA (no en widgets hijos):
        // adw::Window recibe TODOS los eventos de mouse sin importar qué widget está encima.
        // GtkOverlay con overlaid children bloquea eventos al overlay, por eso se conecta aquí.
        let motion_ctrl = gtk::EventControllerMotion::new();
        motion_ctrl.connect_motion(glib::clone!(
            #[weak] video_widget,
            move |_, x, y| {
                video_widget.on_pointer_motion(x, y);
            }
        ));
        video_window.add_controller(motion_ctrl);

        video_window.present();
    }

    /// Carga y reproduce un archivo de video directamente (desde drag-and-drop).
    pub fn load_video(&self, path: &str) {
        // Crear MovieObject temporal para reutilizar open_player
        let movie = MovieObject::from_video_path(path);
        self.open_player(&movie);
    }

    // ─── Búsqueda ─────────────────────────────────────────────────────────────

    fn setup_search_bar(&self) {
        let imp = self.imp();

        let search_button = (*imp.search_button).clone();
        let search_bar_clone = (*imp.search_bar).clone();

        // Vincular el ToggleButton al SearchBar
        imp.search_bar.connect_search_mode_enabled_notify(
            glib::clone!(
                #[weak] search_button,
                move |bar| {
                    search_button.set_active(bar.is_search_mode());
                }
            )
        );
        imp.search_button.connect_toggled(
            glib::clone!(
                #[weak] search_bar_clone,
                move |btn| {
                    search_bar_clone.set_search_mode(btn.is_active());
                }
            )
        );

        // Conectar el SearchEntry al filtro
        imp.search_entry.connect_search_changed(
            glib::clone!(
                #[weak(rename_to = win)] self,
                move |entry| {
                    let imp = win.imp();
                    let text = entry.text().to_string();
                    let borrowed = imp.string_filter.borrow();
                    if let Some(filter) = borrowed.as_ref() {
                        filter.set_search(Some(&text));
                    }
                }
            )
        );
    }

    // ─── Ordenamiento ───────────────────────────────────────────────────────────

    fn setup_sort_button(&self) {
        let imp = self.imp();

        // Grupo de acciones para el menú de ordenamiento
        let action_group = gio::SimpleActionGroup::new();
        self.insert_action_group("sort", Some(&action_group));

        let actions: [(&str, SortMode); 5] = [
            ("none", SortMode::None),
            ("title-asc", SortMode::TitleAsc),
            ("title-desc", SortMode::TitleDesc),
            ("year-desc", SortMode::YearDesc),
            ("year-asc", SortMode::YearAsc),
        ];

        for (name, mode) in actions {
            let action = gio::SimpleAction::new(name, None);
            action.connect_activate(glib::clone!(
                #[weak(rename_to = win)] self,
                move |_, _| {
                    win.apply_sort(mode);
                }
            ));
            action_group.add_action(&action);
        }

        // Construir menú
        let menu = gio::Menu::new();
        let section = gio::Menu::new();

        let none_item = gio::MenuItem::new(
            Some(&gettext("Default Order")),
            Some("sort.none"),
        );
        section.append_item(&none_item);

        let title_asc_item = gio::MenuItem::new(
            Some(&gettext("Title A → Z")),
            Some("sort.title-asc"),
        );
        section.append_item(&title_asc_item);

        let title_desc_item = gio::MenuItem::new(
            Some(&gettext("Title Z → A")),
            Some("sort.title-desc"),
        );
        section.append_item(&title_desc_item);

        let year_desc_item = gio::MenuItem::new(
            Some(&gettext("Year (Newest First)")),
            Some("sort.year-desc"),
        );
        section.append_item(&year_desc_item);

        let year_asc_item = gio::MenuItem::new(
            Some(&gettext("Year (Oldest First)")),
            Some("sort.year-asc"),
        );
        section.append_item(&year_asc_item);

        menu.append_section(None, &section);
        imp.sort_button.set_menu_model(Some(&menu));
    }

    /// Aplica el modo de ordenamiento seleccionado al SortListModel.
    fn apply_sort(&self, mode: SortMode) {
        let sorter = MovieSorter::new(mode);
        if let Some(sort_model) = self.imp().sort_model.borrow().as_ref() {
            if mode == SortMode::None {
                sort_model.set_sorter(None::<&gtk::Sorter>);
            } else {
                sort_model.set_sorter(Some(&sorter));
            }
        }
    }

    // ─── Escaneo de directorios ───────────────────────────────────────────────

    fn connect_folder_buttons(&self) {
        // Botón en la cabecera
        self.imp().add_folder_button.connect_clicked(
            glib::clone!(#[weak(rename_to = win)] self, move |_| win.pick_folder())
        );
        // Botón en la pantalla vacía
        self.imp().empty_add_folder_button.connect_clicked(
            glib::clone!(#[weak(rename_to = win)] self, move |_| win.pick_folder())
        );
        // Botón de refresh: re-escanear biblioteca completa
        self.imp().refresh_button.connect_clicked(
            glib::clone!(#[weak(rename_to = win)] self, move |_| win.refresh_library())
        );
    }

    /// Re-escanea todos los directorios guardados en GSettings.
    /// Limpia el catálogo actual antes de empezar.
    fn refresh_library(&self) {
        let imp = self.imp();

        // Limpiar catálogo actual
        if let Some(store) = imp.catalog_store.borrow().as_ref() {
            store.remove_all();
        }
        imp.showing_test_data.set(false);
        self.update_content_stack();

        // Obtener directorios guardados
        let dirs = match self.application()
            .and_downcast::<crate::application::VideoclubApplication>()
        {
            Some(app) => app.imp().settings.scan_directories(),
            None => return,
        };

        if dirs.is_empty() {
            info!("Refresh: no hay directorios configurados");
            return;
        }

        info!("Refresh: re-escaneando {} directorio(s)", dirs.len());
        self.imp().enrich_after_scan.set(true);
        for dir in dirs {
            self.scan_directory(dir);
        }
    }

    /// Elimina todas las películas de la biblioteca, los metadatos
    /// y los directorios de escaneo guardados. Deja la aplicación
    /// en estado limpio como si fuera la primera ejecución.
    pub fn clear_library(&self) {
        let imp = self.imp();

        // Limpiar catálogo visible
        if let Some(store) = imp.catalog_store.borrow().as_ref() {
            store.remove_all();
        }
        imp.showing_test_data.set(false);
        self.update_content_stack();

        // Limpiar metadatos persistentes (library.json)
        imp.metadata_store.borrow_mut().clear_all();

        // Limpiar directorios de escaneo en GSettings
        if let Some(app) = self
            .application()
            .and_downcast::<crate::application::VideoclubApplication>()
        {
            app.imp().settings.clear_scan_directories();
        }

        info!("Biblioteca limpiada completamente");
    }

    /// Abre el diálogo de carpeta y escanea en un hilo.
    fn pick_folder(&self) {
        let dialog = gtk::FileDialog::new();
        dialog.set_title(&gettext("Select Movie Folder"));

        dialog.select_folder(
            Some(self),
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = win)] self,
                move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            win.imp().enrich_after_scan.set(true);
                            win.scan_directory(path.to_string_lossy().into_owned());
                        }
                    }
                }
            ),
        );
    }

    /// Escanea un directorio en un hilo separado y agrega los resultados al catálogo.
    /// También guarda la ruta en GSettings para recordarla entre sesiones.
    pub(crate) fn scan_directory(&self, path: String) {
        info!("Escaneando directorio: {}", path);
        let imp = self.imp();

        // Si se estaban mostrando datos de prueba, limpiar el catálogo
        if imp.showing_test_data.get() {
            if let Some(store) = imp.catalog_store.borrow().as_ref() {
                store.remove_all();
            }
            imp.showing_test_data.set(false);
        }

        // Persistir el directorio en GSettings para re-escanearlo al arrancar
        if let Some(app) = self.application()
            .and_downcast::<crate::application::VideoclubApplication>()
        {
            app.imp().settings.add_scan_directory(&path);
            info!("Directorio guardado en GSettings: {}", path);
        }

        let (sender, receiver) = async_channel::bounded::<String>(32);

        // Escaneo en hilo nativo para no bloquear la UI
        std::thread::spawn(move || {
            use videoclub_core::scanner;
            for file_path in scanner::scan_directory(&path) {
                if sender.send_blocking(file_path).is_err() {
                    break;
                }
            }
        });

        // Recibir resultados en el contexto principal de glib,
        // luego lanzar el enriquecimiento de metadatos
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = win)] self,
            async move {
                // Películas que necesitan enriquecimiento (sin metadatos en store)
                let mut to_enrich: Vec<MovieObject> = Vec::new();

                while let Ok(file_path) = receiver.recv().await {
                    let imp = win.imp();
                    if let Some(catalog) = imp.catalog_store.borrow().as_ref() {
                        let movie = MovieObject::from_video_path(&file_path);

                        // Precargar desde el store si hay metadatos guardados
                        let has_stored = {
                            let store = imp.metadata_store.borrow();
                            if let Some(stored) = store.get(&file_path) {
                                win.apply_stored_to_movie(&movie, stored);
                                stored.has_metadata
                            } else {
                                false
                            }
                        };

                        catalog.append(&movie);
                        if !has_stored && imp.enrich_after_scan.get() {
                            to_enrich.push(movie);
                        }
                    }
                    win.update_content_stack();
                }

                // Escaneo terminado → enriquecer solo si el flag está activo
                if !to_enrich.is_empty() {
                    win.enrich_all_movies(to_enrich);
                }
                win.imp().enrich_after_scan.set(false);
            }
        ));
    }

    /// Aplica un `StoredMetadata` al `MovieObject` correspondiente.
    fn apply_stored_to_movie(&self, movie: &MovieObject, stored: &StoredMetadata) {
        if let Some(title) = &stored.title {
            movie.set_title(title.as_str());
        }
        if let Some(year) = stored.year {
            movie.set_year(year);
        }
        if let Some(synopsis) = &stored.synopsis {
            movie.set_synopsis(synopsis.as_str());
        }
        if let Some(poster) = &stored.poster_path {
            movie.set_poster_path(poster.clone());
        }
        if let Some(id) = &stored.imdb_id {
            movie.set_imdb_id(id.as_str());
        }
        if let Some(rating) = &stored.imdb_rating {
            movie.set_rating(rating.as_str());
        }
        if let Some(genre) = &stored.genre {
            movie.set_genre(genre.as_str());
        }
        if let Some(runtime) = &stored.runtime {
            movie.set_runtime(runtime.as_str());
        }
        movie.set_has_metadata(stored.has_metadata);
        // Restaurar estado de subtítulos si el archivo aún existe en disco
        if let Some(sub_path) = &stored.subtitle_path {
            if std::path::Path::new(sub_path).exists() {
                movie.set_subtitles_ready(true);
            }
        }
    }

    /// Busca en el catálogo el MovieObject con la ruta de video dada.
    fn find_movie_by_path(&self, video_path: &str) -> Option<MovieObject> {
        let imp = self.imp();
        let borrowed = imp.catalog_store.borrow();
        let store = borrowed.as_ref()?;
        let n = store.n_items();
        for i in 0..n {
            if let Some(movie) = store.item(i).and_downcast::<MovieObject>() {
                if movie.video_path() == video_path {
                    return Some(movie);
                }
            }
        }
        None
    }

    /// Lanza el enriquecimiento de metadatos de una lista de películas usando tokio + OMDb.
    fn enrich_all_movies(&self, movies: Vec<MovieObject>) {
        let app = self.application()
            .and_downcast::<crate::application::VideoclubApplication>();
        let api_key = app.as_ref()
            .map(|a| a.omdb_api_key())
            .unwrap_or_default();
        let opensubs_key = app.as_ref()
            .map(|a| a.opensubtitles_api_key())
            .unwrap_or_default();
        let preferred_subtitle_language = app.as_ref()
            .map(|a| a.preferred_subtitle_language())
            .unwrap_or_else(|| "es".to_string());

        if api_key.is_empty() {
            info!("Sin API key de OMDb configurada, omitiendo enriquecimiento");
            return;
        }

        info!("Enriqueciendo {} películas con OMDb...", movies.len());

        // Extraer datos del store para cada película (overrides de búsqueda)
        let stored_map: Vec<(String, Option<StoredMetadata>)> = movies.iter()
            .map(|m| {
                let path = m.video_path();
                let stored = self.imp().metadata_store.borrow().get(&path).cloned();
                (path, stored)
            })
            .collect();

        // Canal: el thread envía el StoredMetadata completo al hilo principal
        type EnrichResult = (String, StoredMetadata);
        let (tx, rx) = async_channel::bounded::<EnrichResult>(16);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let Ok(rt) = rt else {
            error!("No se pudo crear runtime tokio");
            return;
        };

        std::thread::spawn(move || {
            rt.block_on(async move {
                use videoclub_core::enricher::MovieEnricher;
                use videoclub_core::omdb::OmdbClient;
                use videoclub_core::subtitles::SubtitlesClient;

                let omdb = match OmdbClient::new(api_key) {
                    Ok(c) => c,
                    Err(e) => { error!("Error al inicializar OMDb: {}", e); return; }
                };
                let has_opensubs = !opensubs_key.is_empty();
                let subtitles = match SubtitlesClient::new(opensubs_key) {
                    Ok(s) => s,
                    Err(e) => { warn!("Error al crear SubtitlesClient: {}", e); return; }
                };
                let enricher = match MovieEnricher::new(omdb, subtitles) {
                    Ok(e) => e,
                    Err(e) => { error!("Error al crear MovieEnricher: {}", e); return; }
                };

                for (video_path, stored) in &stored_map {
                    let tmp_movie = MovieObject::from_video_path(video_path);
                    let mut result = enricher
                        .enrich_metadata(&tmp_movie, stored.as_ref())
                        .await
                        .unwrap_or_else(|e| {
                            warn!("Error enriqueciendo '{}': {}", video_path, e);
                            StoredMetadata::new_pending("unknown", None)
                        });

                    // Descargar subtítulos después del enriquecimiento de metadatos
                    // (necesita que title/year estén establecidos para la búsqueda por nombre)
                    if has_opensubs {
                        result.subtitle_path = enricher
                            .download_subtitles(&tmp_movie, &preferred_subtitle_language)
                            .await
                            .unwrap_or_else(|e| {
                                warn!("Error descargando subtítulos para '{}': {}", video_path, e);
                                None
                            });
                    }

                    if tx.send((video_path.clone(), result)).await.is_err() {
                        break;
                    }
                }
            });
        });

        // Recibir resultados, actualizar MovieObjects y persistir en el store
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = win)] self,
            async move {
                while let Ok((video_path, stored)) = rx.recv().await {
                    // Actualizar el MovieObject real en el catálogo
                    if let Some(movie) = win.find_movie_by_path(&video_path) {
                        win.apply_stored_to_movie(&movie, &stored);
                        // Marcar subtítulos como listos si se descargaron
                        if stored.subtitle_path.is_some() {
                            movie.set_subtitles_ready(true);
                        }
                    }
                    // Guardar en el store
                    win.imp().metadata_store.borrow_mut().upsert(&video_path, stored);
                }
                // Persistir store al disco al finalizar el lote
                win.imp().metadata_store.borrow().save();
                info!("Enriquecimiento completado y store guardado");
            }
        ));
    }

    /// Re-enriquece una sola película (llamado desde el EditMovieDialog).
    pub fn enrich_single_movie(&self, movie: &MovieObject) {
        self.enrich_all_movies(vec![movie.clone()]);
    }

    /// Descarga subtítulos para una sola película (llamado desde el EditMovieDialog).
    /// `on_done` se invoca en el hilo principal con `true` si la descarga tuvo éxito.
    pub fn download_subtitles_single(
        &self,
        movie: &MovieObject,
        on_done: impl FnOnce(bool) + 'static,
    ) {
        let app = self
            .application()
            .and_downcast::<crate::application::VideoclubApplication>();
        let opensubs_key = app
            .as_ref()
            .map(|a| a.opensubtitles_api_key())
            .unwrap_or_default();
        let language = app
            .as_ref()
            .map(|a| a.preferred_subtitle_language())
            .unwrap_or_else(|| "es".to_string());

        if opensubs_key.is_empty() {
            info!("Sin API key de OpenSubtitles configurada, omitiendo descarga de subtítulos");
            glib::spawn_future_local(async move {
                on_done(false);
            });
            return;
        }

        info!("Descargando subtítulos para '{}'...", movie.title());

        // Extraer datos planos antes de mover al thread (MovieObject no es Send)
        let video_path = movie.video_path().to_string();
        let movie_year = movie.year();
        let movie_for_result = movie.clone();

        // Canal: el thread envía el subtitle_path (o None si falló)
        let (tx, rx) = async_channel::bounded::<Option<String>>(1);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let Ok(rt) = rt else {
            error!("No se pudo crear runtime tokio");
            glib::spawn_future_local(async move {
                on_done(false);
            });
            return;
        };

        let video_path_clone = video_path.clone();
        std::thread::spawn(move || {
            rt.block_on(async move {
                use videoclub_core::subtitles::SubtitlesClient;

                // Construir el resultado en todos los caminos para garantizar que tx.send() se ejecute
                let result = {
                    let subtitles = match SubtitlesClient::new(opensubs_key) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            warn!("Error al crear SubtitlesClient: {}", e);
                            None
                        }
                    };

                    match subtitles {
                        Some(subtitles) => {
                            // Crear MovieObject dentro del thread (no cruza el límite Send)
                            let tmp_movie = MovieObject::from_video_path(&video_path_clone);
                            if movie_year > 0 {
                                tmp_movie.set_year(movie_year);
                            }

                            videoclub_core::enricher::download_subtitles_for_movie(
                                &subtitles,
                                &tmp_movie,
                                &language,
                            )
                            .await
                            .unwrap_or_else(|e| {
                                warn!(
                                    "Error descargando subtítulos para '{}': {}",
                                    video_path_clone,
                                    e
                                );
                                None
                            })
                        }
                        None => None,
                    }
                };

                let _ = tx.send(result).await;
            });
        });

        // Recibir resultado y actualizar MovieObject + store en el hilo principal
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = win)] self,
            async move {
                let subtitle_path = rx.recv().await.ok().flatten();

                if let Some(ref path) = subtitle_path {
                    movie_for_result.set_subtitles_ready(true);
                    info!("Subtítulos descargados: {}", path);
                }

                // Actualizar el store con la ruta del subtítulo
                let mut store = win.imp().metadata_store.borrow_mut();
                if let Some(existing) = store.get(&video_path).cloned() {
                    let mut updated = existing.clone();
                    updated.subtitle_path = subtitle_path.clone();
                    store.upsert(&video_path, updated);
                } else if subtitle_path.is_some() {
                    let mut meta = videoclub_core::metadata_store::StoredMetadata::new_pending(
                        &movie_for_result.title(),
                        if movie_for_result.year() > 0 {
                            Some(movie_for_result.year())
                        } else {
                            None
                        },
                    );
                    meta.subtitle_path = subtitle_path.clone();
                    store.upsert(&video_path, meta);
                }
                store.save();

                on_done(subtitle_path.is_some());
            }
        ));
    }

    // ─── Drag-and-Drop ────────────────────────────────────────────────────────

    fn setup_video_drop_target(&self) {
        let drop_target = gtk::DropTarget::new(gio::File::static_type(), gdk::DragAction::COPY);
        drop_target.connect_drop(glib::clone!(
            #[weak(rename_to = win)] self,
            #[upgrade_or] false,
            move |_target, value, _x, _y| {
                if let Ok(file) = value.get::<gio::File>() {
                    if let Some(path) = file.path() {
                        win.load_video(path.to_string_lossy().as_ref());
                        return true;
                    }
                }
                false
            }
        ));
        self.add_controller(drop_target);
    }
}

