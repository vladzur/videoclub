// cache.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::{debug, info, warn};

/// Gestor de caché para pósters de películas.
///
/// Almacena las imágenes de pósters descargadas en `~/.cache/videoclub/posters/`
/// para evitar descargas redundantes entre sesiones.
pub struct PosterCache {
    /// Directorio donde se almacenan los pósters.
    cache_dir: PathBuf,
}

impl PosterCache {
    /// Crea un nuevo gestor de caché.
    ///
    /// Crea el directorio de caché si no existe.
    pub fn new() -> io::Result<Self> {
        let cache_dir = directories::ProjectDirs::from("com", "vladzur", "videoclub")
            .map(|dirs| dirs.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/tmp/videoclub-cache"));

        let poster_dir = cache_dir.join("posters");
        fs::create_dir_all(&poster_dir)?;

        info!("Directorio de caché: {:?}", poster_dir);

        Ok(Self {
            cache_dir: poster_dir,
        })
    }

    /// Verifica si un póster está en caché y devuelve su ruta.
    ///
    /// `tmdb_id` es el ID de la película en TMDb.
    pub fn get_cached(&self, tmdb_id: u64) -> Option<PathBuf> {
        let path = self.poster_path(tmdb_id);
        if path.exists() {
            debug!("Póster encontrado en caché: {:?}", path);
            Some(path)
        } else {
            None
        }
    }

    /// Guarda datos de un póster en el caché.
    ///
    /// Devuelve la ruta local donde se almacenó.
    pub fn cache_poster(&self, tmdb_id: u64, data: &[u8]) -> io::Result<PathBuf> {
        let path = self.poster_path(tmdb_id);
        fs::write(&path, data)?;
        debug!("Póster guardado en caché: {:?}", path);
        Ok(path)
    }

    /// Elimina entradas de caché más antiguas que `max_age`.
    pub fn prune(&self, max_age: Duration) -> io::Result<u32> {
        let now = SystemTime::now();
        let mut removed = 0u32;

        let entries = fs::read_dir(&self.cache_dir)?;
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if now.duration_since(modified).unwrap_or(Duration::ZERO) > max_age {
                            if fs::remove_file(entry.path()).is_ok() {
                                removed += 1;
                                debug!("Eliminada entrada de caché antigua: {:?}", entry.path());
                            }
                        }
                    }
                }
            }
        }

        if removed > 0 {
            info!("Se eliminaron {} entradas antiguas del caché", removed);
        }
        Ok(removed)
    }

    /// Construye la ruta para el póster de un TMDb ID.
    fn poster_path(&self, tmdb_id: u64) -> PathBuf {
        self.cache_dir.join(format!("{}.jpg", tmdb_id))
    }
}

impl Drop for PosterCache {
    fn drop(&mut self) {
        // Intentar limpiar entradas de más de 90 días al cerrar
        if let Err(e) = self.prune(Duration::from_secs(90 * 24 * 3600)) {
            warn!("Error al limpiar caché: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que se puede crear el caché y almacenar/recuperar un póster.
    #[test]
    fn test_cache_store_and_retrieve() {
        let cache = PosterCache::new().unwrap();

        let test_data = b"fake-jpeg-data";
        let tmdb_id = 99999;

        // No debe existir inicialmente
        assert!(cache.get_cached(tmdb_id).is_none());

        // Guardar en caché
        let path = cache.cache_poster(tmdb_id, test_data).unwrap();
        assert!(path.exists());

        // Recuperar
        let cached = cache.get_cached(tmdb_id);
        assert!(cached.is_some());

        // Leer datos guardados
        let data = fs::read(&path).unwrap();
        assert_eq!(data, test_data);

        // Limpiar archivo de prueba
        let _ = fs::remove_file(&path);
    }
}
