// player/inhibit.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::{gio, glib};
use gtk::glib::prelude::*;
use log::{info, warn};

/// Inhibidor de screensaver vía D-Bus (`org.freedesktop.ScreenSaver`).
///
/// Llama a `Inhibit()` cuando la reproducción comienza y a `UnInhibit()`
/// cuando termina. La inhibición se mantiene durante la pausa.
/// El fallo en las llamadas D-Bus no interrumpe la reproducción.
pub struct ScreensaverInhibitor {
    /// Conexión al bus de sesión de D-Bus (se obtiene bajo demanda).
    connection: Option<gio::DBusConnection>,
    /// Cookie retornado por `Inhibit()`. `None` si no hay inhibición activa.
    cookie: Option<u32>,
}

impl ScreensaverInhibitor {
    /// Crea un nuevo inhibidor. No se conecta a D-Bus hasta que se requiera
    /// la primera inhibición.
    pub fn new() -> Self {
        Self {
            connection: None,
            cookie: None,
        }
    }

    /// Intenta inhibir el screensaver. Si ya está inhibiendo, no hace nada.
    /// Si falla la conexión D-Bus o la llamada, registra una advertencia
    /// y continúa sin interrumpir la reproducción.
    pub fn inhibit(&mut self) {
        // Idempotente: ya estamos inhibiendo
        if self.cookie.is_some() {
            return;
        }

        // Conexión lazy al bus de sesión
        if self.connection.is_none() {
            match gio::bus_get_sync(
                gio::BusType::Session,
                None::<&gio::Cancellable>,
            ) {
                Ok(conn) => {
                    self.connection = Some(conn);
                }
                Err(e) => {
                    warn!(
                        "No se pudo conectar al bus de sesión D-Bus: {}",
                        e
                    );
                    return;
                }
            }
        }

        let conn = self.connection.as_ref().unwrap();
        let params: glib::Variant = ("Videoclub", "Playing a movie").to_variant();

        match conn.call_sync(
            Some("org.freedesktop.ScreenSaver"),
            "/org/freedesktop/ScreenSaver",
            "org.freedesktop.ScreenSaver",
            "Inhibit",
            Some(&params),
            Some(&glib::VariantTy::new("(u)").unwrap()),
            gio::DBusCallFlags::NONE,
            5000,
            None::<&gio::Cancellable>,
        ) {
            Ok(reply) => {
                self.cookie = Some(reply.child_get::<u32>(0));
                info!("Screensaver inhibido (cookie: {:?})", self.cookie);
            }
            Err(e) => {
                warn!(
                    "Error al inhibir el screensaver ({}). \
                     El escritorio podría no soportar org.freedesktop.ScreenSaver",
                    e
                );
            }
        }
    }

    /// Libera la inhibición del screensaver. Si no hay inhibición activa, no hace nada.
    /// Siempre limpia el cookie internamente, incluso si la llamada D-Bus falla.
    pub fn uninhibit(&mut self) {
        let cookie = match self.cookie.take() {
            Some(c) => c,
            None => return, // No hay inhibición activa
        };

        let conn = match self.connection.as_ref() {
            Some(c) => c,
            None => return, // Sin conexión, nada que liberar
        };

        let params: glib::Variant = (cookie,).to_variant();

        if let Err(e) = conn.call_sync(
            Some("org.freedesktop.ScreenSaver"),
            "/org/freedesktop/ScreenSaver",
            "org.freedesktop.ScreenSaver",
            "UnInhibit",
            Some(&params),
            None::<&glib::VariantTy>,
            gio::DBusCallFlags::NONE,
            5000,
            None::<&gio::Cancellable>,
        ) {
            warn!("Error al liberar la inhibición del screensaver: {}", e);
        } else {
            info!("Inhibición del screensaver liberada (cookie: {})", cookie);
        }
    }
}

impl Drop for ScreensaverInhibitor {
    fn drop(&mut self) {
        // Limpieza defensiva: si el controlador es destruido sin llamar stop(),
        // liberamos la inhibición para no dejar el screensaver bloqueado.
        self.uninhibit();
    }
}
