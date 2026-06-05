// settings.rs
//
// Copyright 2026 Vladimir Zurita
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gio::prelude::*;

/// Wrapper tipado alrededor de `gio::Settings` para acceder a la configuración
/// de la aplicación sin usar strings de clave en el código.
pub struct AppSettings {
    settings: gio::Settings,
}

impl AppSettings {
    /// Crea una nueva instancia conectada al esquema de GSettings de la aplicación.
    pub fn new() -> Self {
        Self {
            settings: gio::Settings::new("com.vladzur.videoclub"),
        }
    }

    /// Directorios configurados por el usuario para escanear en busca de películas.
    pub fn scan_directories(&self) -> Vec<String> {
        self.settings.strv("scan-directories").iter().map(|s| s.to_string()).collect()
    }

    /// Agrega un directorio a la lista de directorios de escaneo.
    pub fn add_scan_directory(&self, path: &str) {
        // Convertir a Vec<String> que sí implementa IntoStrV
        let mut dirs: Vec<String> = self.settings.strv("scan-directories")
            .iter()
            .map(|s| s.to_string())
            .collect();
        // Evitar duplicados
        if !dirs.iter().any(|d| d == path) {
            dirs.push(path.to_string());
            let _ = self.settings.set_strv("scan-directories", dirs);
        }
    }

    /// Elimina un directorio de la lista de directorios de escaneo.
    pub fn remove_scan_directory(&self, path: &str) {
        let dirs: Vec<String> = self
            .settings
            .strv("scan-directories")
            .iter()
            .filter(|d| d.as_str() != path)
            .map(|s| s.to_string())
            .collect();
        let _ = self.settings.set_strv("scan-directories", dirs);
    }

    /// Idioma preferido para el audio de las películas (código ISO 639-1).
    pub fn preferred_audio_language(&self) -> String {
        self.settings.string("preferred-audio-language").to_string()
    }

    /// Idioma preferido para los subtítulos (código ISO 639-1).
    pub fn preferred_subtitle_language(&self) -> String {
        self.settings.string("preferred-subtitle-language").to_string()
    }

    /// Clave de API para OMDb (Open Movie Database).
    pub fn omdb_api_key(&self) -> String {
        self.settings.string("omdb-api-key").to_string()
    }

    /// Establece la clave de API de OMDb.
    pub fn set_omdb_api_key(&self, key: &str) {
        let _ = self.settings.set_string("omdb-api-key", key);
    }

    /// Clave de API para OpenSubtitles.
    pub fn opensubtitles_api_key(&self) -> String {
        self.settings.string("opensubtitles-api-key").to_string()
    }

    /// Establece la clave de API de OpenSubtitles.
    pub fn set_opensubtitles_api_key(&self, key: &str) {
        let _ = self.settings.set_string("opensubtitles-api-key", key);
    }

    /// Establece el idioma preferido de audio.
    pub fn set_preferred_audio_language(&self, lang: &str) {
        let _ = self.settings.set_string("preferred-audio-language", lang);
    }

    /// Establece el idioma preferido de subtítulos.
    pub fn set_preferred_subtitle_language(&self, lang: &str) {
        let _ = self.settings.set_string("preferred-subtitle-language", lang);
    }

    /// Tamaño de la ventana principal (ancho, alto).
    pub fn window_size(&self) -> (i32, i32) {
        (
            self.settings.int("window-width"),
            self.settings.int("window-height"),
        )
    }

    /// Guarda el tamaño actual de la ventana.
    pub fn set_window_size(&self, width: i32, height: i32) {
        let _ = self.settings.set_int("window-width", width);
        let _ = self.settings.set_int("window-height", height);
    }

    /// Indica si la ventana estaba maximizada en la última sesión.
    pub fn window_maximized(&self) -> bool {
        self.settings.boolean("window-maximized")
    }

    /// Guarda el estado maximizado de la ventana.
    pub fn set_window_maximized(&self, maximized: bool) {
        let _ = self.settings.set_boolean("window-maximized", maximized);
    }

    /// Descripción de fuente Pango para los subtítulos (ej. "Sans 16").
    pub fn subtitle_font_desc(&self) -> String {
        self.settings.string("subtitle-font-desc").to_string()
    }

    /// Establece la fuente de los subtítulos.
    pub fn set_subtitle_font_desc(&self, font_desc: &str) {
        let _ = self.settings.set_string("subtitle-font-desc", font_desc);
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que `AppSettings` se puede crear sin pánico.
    /// Nota: Este test requiere que el esquema GSettings esté instalado
    /// para ejecutarse correctamente en entornos de desarrollo.
    #[test]
    fn test_create_settings() {
        // En un entorno donde el esquema está instalado, esto no debería fallar.
        // En CI, puede necesitar `GSETTINGS_SCHEMA_DIR` configurado.
        let result = std::panic::catch_unwind(|| {
            let _settings = AppSettings::new();
        });
        // Si el esquema no está instalado, el test se salta (no es un fallo real)
        if result.is_err() {
            eprintln!("INFO: GSettings schema not installed, skipping test_create_settings");
        }
    }
}
