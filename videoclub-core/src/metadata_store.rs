// metadata_store.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::{info, warn};
use serde::{Deserialize, Serialize};

/// Metadatos persistentes de una película, indexados por ruta de archivo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMetadata {
    /// Título enviado a OMDb para la búsqueda (editable por el usuario).
    /// Inicialmente inferido del nombre de archivo.
    pub search_title: String,

    /// Año enviado a OMDb para la búsqueda (editable por el usuario).
    pub search_year: Option<i32>,

    /// Título de la película (resultado de OMDb o edición manual).
    pub title: Option<String>,

    /// Año de estreno almacenado.
    pub year: Option<i32>,

    /// Sinopsis (de OMDb o editada manualmente).
    pub synopsis: Option<String>,

    /// Ruta local al archivo de póster cacheado.
    pub poster_path: Option<String>,

    /// ID de IMDb (ej: "tt0133093").
    pub imdb_id: Option<String>,

    /// Puntuación de IMDb (ej: "8.8").
    pub imdb_rating: Option<String>,

    /// Género (ej: "Action, Sci-Fi").
    pub genre: Option<String>,

    /// Duración (ej: "136 min").
    pub runtime: Option<String>,

    /// `true` si los metadatos provienen de OMDb o fueron introducidos manualmente.
    /// Las películas con `has_metadata: true` no se re-fetchan en un Refresh Library.
    pub has_metadata: bool,

    /// Ruta local al archivo de subtítulos descargado (`.srt`), si existe.
    pub subtitle_path: Option<String>,

    /// Posición en la que se detuvo la reproducción por última vez.
    #[serde(default)]
    pub last_position: Option<f64>,
}

impl StoredMetadata {
    /// Crea una entrada nueva sin metadatos aún, solo con parámetros de búsqueda.
    pub fn new_pending(search_title: &str, search_year: Option<i32>) -> Self {
        Self {
            search_title: search_title.to_string(),
            search_year,
            title: None,
            year: None,
            synopsis: None,
            poster_path: None,
            imdb_id: None,
            imdb_rating: None,
            genre: None,
            runtime: None,
            has_metadata: false,
            subtitle_path: None,
            last_position: None,
        }
    }
}

/// Store JSON centralizado para metadatos de la biblioteca.
///
/// Persiste en `~/.local/share/videoclub/library.json`.
/// La clave de cada entrada es la ruta absoluta del archivo de video.
pub struct MetadataStore {
    path: PathBuf,
    entries: HashMap<String, StoredMetadata>,
}

impl Default for MetadataStore {
    fn default() -> Self {
        Self {
            path: Self::store_path(),
            entries: HashMap::new(),
        }
    }
}

impl MetadataStore {
    /// Carga el store desde disco, o crea uno vacío si no existe.
    pub fn load() -> Self {
        let path = Self::store_path();

        let entries: HashMap<String, StoredMetadata> = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| {
                    warn!("No se pudo parsear el store de metadatos, iniciando vacío");
                    HashMap::new()
                })
        } else {
            HashMap::new()
        };

        info!(
            "MetadataStore cargado: {} entradas desde {:?}",
            entries.len(),
            path
        );

        Self { path, entries }
    }

    /// Persiste el store completo al disco.
    pub fn save(&self) {
        if let Some(parent) = self.path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                warn!("No se pudo crear directorio del store: {}", e);
                return;
            }
        }

        match serde_json::to_string_pretty(&self.entries) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.path, &content) {
                    warn!("No se pudo guardar el store: {}", e);
                } else {
                    info!("MetadataStore guardado: {} entradas", self.entries.len());
                }
            }
            Err(e) => warn!("Error serializando el store: {}", e),
        }
    }

    /// Devuelve los metadatos almacenados para una ruta de video, si existen.
    pub fn get(&self, video_path: &str) -> Option<&StoredMetadata> {
        self.entries.get(video_path)
    }

    /// Inserta o reemplaza la entrada para una ruta de video.
    pub fn upsert(&mut self, video_path: &str, meta: StoredMetadata) {
        self.entries.insert(video_path.to_string(), meta);
    }

    /// Actualiza los parámetros de búsqueda (sin afectar metadatos ya almacenados).
    /// Si la entrada no existe, la crea con `has_metadata: false`.
    pub fn set_search_params(&mut self, video_path: &str, title: &str, year: Option<i32>) {
        if let Some(entry) = self.entries.get_mut(video_path) {
            entry.search_title = title.to_string();
            entry.search_year = year;
        } else {
            self.entries.insert(
                video_path.to_string(),
                StoredMetadata::new_pending(title, year),
            );
        }
    }

    /// Marca una entrada como sin metadatos, forzando un re-fetch la próxima vez.
    pub fn clear_metadata(&mut self, video_path: &str) {
        if let Some(entry) = self.entries.get_mut(video_path) {
            entry.has_metadata = false;
            entry.title = None;
            entry.year = None;
            entry.synopsis = None;
            entry.poster_path = None;
            entry.imdb_id = None;
            entry.imdb_rating = None;
            entry.genre = None;
            entry.runtime = None;
            entry.subtitle_path = None;
            entry.last_position = None;
        }
    }

    /// Actualiza la última posición de reproducción de una película.
    pub fn set_last_position(&mut self, video_path: &str, position: Option<f64>) {
        if let Some(entry) = self.entries.get_mut(video_path) {
            entry.last_position = position;
        }
    }


    /// Elimina todas las entradas del store y persiste el estado vacío al disco.
    pub fn clear_all(&mut self) {
        self.entries.clear();
        self.save();
    }

    /// Ruta del archivo de store en el sistema de archivos.
    fn store_path() -> PathBuf {
        directories::ProjectDirs::from("com", "vladzur", "videoclub")
            .map(|dirs| dirs.data_dir().join("library.json"))
            .unwrap_or_else(|| PathBuf::from("/tmp/videoclub-library.json"))
    }
}
