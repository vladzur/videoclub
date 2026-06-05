/* widgets/video_widget.rs
 *
 * Copyright 2026 Vladimir Zurita
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::cell::RefCell;
use std::time::Duration;

use gtk::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, gio};

use crate::player::controller::PlaybackController;
use crate::player::events::PlaybackState;

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

        pub controller: RefCell<Option<PlaybackController>>,
        pub updating_scale: std::cell::Cell<bool>,
        pub player_window: RefCell<Option<gtk::Window>>,
        /// Contador de ticks de 500ms sin movimiento de mouse en fullscreen.
        pub fullscreen_idle_ticks: std::cell::Cell<u32>,
        /// Última posición registrada del puntero (para filtrar eventos sintéticos de X11).
        pub last_mouse_pos: std::cell::Cell<(f64, f64)>,
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
        fn dispose(&self) {
            // Detener reproducción al destruir el widget
            if let Some(ctrl) = self.controller.borrow().as_ref() {
                let _ = ctrl.stop();
            }
            // Eliminar hijos del template
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

    /// Vincula un `PlaybackController` al widget y conecta todos los controles.
    pub fn setup_player(&self, controller: PlaybackController) {
        let imp = self.imp();

        // Conectar el paintable del video al Picture
        if let Some(paintable) = controller.paintable() {
            imp.picture.set_paintable(Some(&paintable));
        }

        // Guardar el controlador
        imp.controller.replace(Some(controller));

        // --- Botón play/pausa ---
        imp.play_button.connect_clicked(glib::clone!(
            #[weak(rename_to = widget)] self,
            move |_| {
                let imp = widget.imp();
                if let Some(ctrl) = imp.controller.borrow().as_ref() {
                    let _ = ctrl.toggle_play_pause();
                }
                // Actualizar ícono FUERA del borrow para que el Ref sea liberado
                widget.update_play_icon();
            }
        ));

        // --- Barra de progreso: seek al soltar ---
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

        // --- Timer: actualizar posición cada 500 ms ---
        glib::timeout_add_local(Duration::from_millis(500), glib::clone!(
            #[weak(rename_to = widget)] self,
            #[upgrade_or] glib::ControlFlow::Break,
            move || {
                widget.update_position();
                glib::ControlFlow::Continue
            }
        ));

        // --- Botón pantalla completa ---
        imp.fullscreen_button.connect_clicked(glib::clone!(
            #[weak(rename_to = widget)] self,
            move |_| {
                widget.toggle_fullscreen();
            }
        ));

        // --- Control de volumen ---
        // Iconos: [mute, máximo, bajo, medio] — se selecciona según el valor actual
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
    }

    /// Llamar desde el motion controller de la ventana.
    /// Filtra eventos sintéticos de X11 comparando la posición anterior.
    pub fn on_pointer_motion(&self, x: f64, y: f64) {
        let imp = self.imp();
        let (last_x, last_y) = imp.last_mouse_pos.get();
        // Ignorar eventos sintéticos: posición sin cambio real (>2px)
        if (x - last_x).abs() < 2.0 && (y - last_y).abs() < 2.0 {
            return;
        }
        imp.last_mouse_pos.set((x, y));
        imp.fullscreen_idle_ticks.set(0);
        imp.controls_revealer.set_reveal_child(true);
        self.set_cursor(None);
    }


    /// Actualiza la barra de progreso y la etiqueta de tiempo.
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

        // Auto-ocultar controles en fullscreen tras 3s de inactividad (6 ticks * 500ms)
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

    /// Actualiza el ícono del botón play/pausa según el estado actual.
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

    /// Vincula la ventana del reproductor para operaciones de fullscreen.
    pub fn set_player_window(&self, window: &gtk::Window) {
        *self.imp().player_window.borrow_mut() = Some(window.clone());
    }

    /// Limpia la referencia a la ventana (llamar al cerrar para romper ciclo).
    pub fn clear_player_window(&self) {
        *self.imp().player_window.borrow_mut() = None;
    }

    /// Detiene el pipeline GStreamer. Llamar antes de cerrar la ventana.
    pub fn stop_playback(&self) {
        let borrow = self.imp().controller.borrow();
        if let Some(ctrl) = borrow.as_ref() {
            let _ = ctrl.stop();
        }
    }

    /// Ajusta el volumen del pipeline (0.0 = mute, 1.0 = máximo).
    pub fn set_volume(&self, volume: f64) {
        let borrow = self.imp().controller.borrow();
        if let Some(ctrl) = borrow.as_ref() {
            ctrl.set_volume(volume);
        }
    }

    /// Alterna entre pantalla completa y ventana normal.
    fn toggle_fullscreen(&self) {
        let borrowed = self.imp().player_window.borrow();
        log::debug!("toggle_fullscreen: player_window is_some={}", borrowed.is_some());

        let Some(window) = borrowed.as_ref() else {
            log::warn!("toggle_fullscreen: no hay ventana registrada");
            return;
        };

        let currently = window.is_fullscreen();
        log::debug!("toggle_fullscreen: is_fullscreen antes={}", currently);

        if currently {
            window.unfullscreen();
        } else {
            window.fullscreen();
        }

        log::debug!("toggle_fullscreen: is_fullscreen después={}", window.is_fullscreen());
    }

    /// Actualiza el ícono y visibilidad del header bar según el estado de fullscreen.
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

/// Formatea segundos como MM:SS.
fn format_time(secs: f64) -> String {
    let total = secs as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{:02}:{:02}", m, s)
}
