// catalog.rs
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

use crate::movie::MovieObject;
use gio::prelude::*;

/// Catálogo global de películas.
///
/// Envuelve un `gio::ListStore` que sirve como *Single Source of Truth*
/// para toda la UI. Los widgets de GTK4 se bindean directamente a este modelo.
pub struct MovieCatalog {
    store: gio::ListStore,
}

impl MovieCatalog {
    /// Crea un nuevo catálogo vacío.
    ///
    /// El `ListStore` interno acepta únicamente objetos de tipo `MovieObject`.
    pub fn new() -> Self {
        Self {
            store: gio::ListStore::with_type(MovieObject::static_type()),
        }
    }

    /// Devuelve una referencia al `gio::ListStore` interno.
    ///
    /// Este store puede ser usado directamente como modelo de `GtkGridView`.
    pub fn store(&self) -> &gio::ListStore {
        &self.store
    }

    /// Agrega una película al catálogo.
    pub fn add_movie(&self, movie: &MovieObject) {
        self.store.append(movie);
    }

    /// Elimina una película del catálogo por su posición.
    pub fn remove_movie(&self, position: u32) {
        self.store.remove(position);
    }

    /// Elimina todas las películas del catálogo.
    pub fn clear(&self) {
        self.store.remove_all();
    }

    /// Busca una película por la ruta de su archivo de video.
    ///
    /// Devuelve `None` si no se encuentra ninguna coincidencia.
    pub fn find_by_path(&self, path: &str) -> Option<MovieObject> {
        let n = self.store.n_items();
        for i in 0..n {
            if let Some(movie) = self.store.item(i).and_then(|obj| obj.downcast::<MovieObject>().ok()) {
                if movie.video_path() == path {
                    return Some(movie);
                }
            }
        }
        None
    }

    /// Devuelve la cantidad de películas en el catálogo.
    pub fn len(&self) -> u32 {
        self.store.n_items()
    }

    /// Devuelve `true` si el catálogo está vacío.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for MovieCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movie::MovieObject;

    /// Helper: crea una MovieObject de prueba con una ruta dada.
    fn create_test_movie(path: &str) -> MovieObject {
        MovieObject::from_video_path(path)
    }

    /// Verifica que se pueden agregar y recuperar películas del catálogo.
    #[test]
    fn test_add_and_find() {
        let catalog = MovieCatalog::new();
        assert_eq!(catalog.len(), 0);
        assert!(catalog.is_empty());

        let movie1 = create_test_movie("/tmp/movie1.mkv");
        let movie2 = create_test_movie("/tmp/movie2.mp4");

        catalog.add_movie(&movie1);
        catalog.add_movie(&movie2);

        assert_eq!(catalog.len(), 2);

        let found = catalog.find_by_path("/tmp/movie1.mkv");
        assert!(found.is_some());
        assert_eq!(found.unwrap().title(), "movie1");
    }

    /// Verifica que `find_by_path` devuelve `None` para rutas inexistentes.
    #[test]
    fn test_find_not_found() {
        let catalog = MovieCatalog::new();
        let result = catalog.find_by_path("/nonexistent/file.avi");
        assert!(result.is_none());
    }

    /// Verifica que se puede eliminar una película del catálogo.
    #[test]
    fn test_remove_movie() {
        let catalog = MovieCatalog::new();
        let movie = create_test_movie("/tmp/to_remove.mkv");
        catalog.add_movie(&movie);
        assert_eq!(catalog.len(), 1);

        catalog.remove_movie(0);
        assert_eq!(catalog.len(), 0);
    }

    /// Verifica que `clear` elimina todas las películas.
    #[test]
    fn test_clear() {
        let catalog = MovieCatalog::new();
        catalog.add_movie(&create_test_movie("/tmp/a.mkv"));
        catalog.add_movie(&create_test_movie("/tmp/b.mkv"));
        catalog.add_movie(&create_test_movie("/tmp/c.mkv"));
        assert_eq!(catalog.len(), 3);

        catalog.clear();
        assert_eq!(catalog.len(), 0);
    }
}
