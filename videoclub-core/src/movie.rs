// movie.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use glib::prelude::*;

mod imp {
    use super::*;
    use glib::subclass::prelude::*;

    /// Estructura interna que contiene los datos de la película.
    /// Las propiedades se exponen como propiedades GObject mediante la macro `#[properties]`.
    ///
    /// NOTA: Se usan `String` e `i32` en lugar de `Option<String>` / `Option<i32>`
    /// porque la macro `#[properties]` de glib 0.20 requiere tipos que implementen
    /// `Property` directamente. Los valores vacíos/cero representan "no disponible".
    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::MovieObject)]
    pub struct MovieObject {
        /// Título de la película (puede ser enriquecido desde TMDb)
        #[property(get, set, name = "title")]
        title: std::cell::RefCell<String>,

        /// Ruta absoluta al archivo de video en el sistema de archivos local.
        /// Cadena vacía si no se ha asignado.
        #[property(get, set, name = "video-path")]
        video_path: std::cell::RefCell<String>,

        /// Ruta local al archivo de póster (descargado de TMDb o cacheado).
        /// Cadena vacía si no hay póster.
        #[property(get, set, name = "poster-path")]
        poster_path: std::cell::RefCell<String>,

        /// Año de estreno de la película. 0 si no se conoce.
        #[property(get, set, name = "year")]
        year: std::cell::RefCell<i32>,

        /// Sinopsis o descripción de la película.
        /// Cadena vacía si no está disponible.
        #[property(get, set, name = "synopsis")]
        synopsis: std::cell::RefCell<String>,

        /// Indica si los subtítulos ya están disponibles localmente.
        #[property(get, set, name = "subtitles-ready")]
        subtitles_ready: std::cell::RefCell<bool>,

        /// Indica si ya se descargaron metadatos desde OMDb.
        #[property(get, set, name = "has-metadata")]
        has_metadata: std::cell::RefCell<bool>,

        /// Hash del archivo de video (requerido por OpenSubtitles para búsqueda exacta).
        /// Cadena vacía si no se ha calculado.
        #[property(get, set, name = "file-hash")]
        file_hash: std::cell::RefCell<String>,

        /// ID de IMDb (ej: "tt0133093"). Cadena vacía si no se conoce.
        #[property(get, set, name = "imdb-id")]
        imdb_id: std::cell::RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MovieObject {
        const NAME: &'static str = "VideoclubMovieObject";
        type Type = super::MovieObject;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MovieObject {}
}

glib::wrapper! {
    /// Objeto GObject que representa una película en el catálogo.
    ///
    /// Expone propiedades dinámicas que pueden ser bindeadas directamente
    /// a widgets de GTK4 mediante `GtkListItemFactory`.
    pub struct MovieObject(ObjectSubclass<imp::MovieObject>);
}

impl MovieObject {
    /// Crea un nuevo MovieObject a partir de la ruta de un archivo de video.
    ///
    /// Extrae el título inicial del nombre del archivo.
    pub fn from_video_path(path: &str) -> Self {
        let filename = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");

        glib::Object::builder()
            .property("title", filename)
            .property("video-path", path)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que se puede crear un MovieObject y acceder a sus propiedades.
    #[test]
    fn test_create_movie_object() {
        let movie = MovieObject::from_video_path("/home/user/movies/The Matrix.mp4");
        assert_eq!(movie.title(), "The Matrix");
        assert_eq!(
            movie.video_path(),
            "/home/user/movies/The Matrix.mp4"
        );
        // Año 0 = desconocido
        assert_eq!(movie.year(), 0);
        assert!(!movie.has_metadata());
        assert!(!movie.subtitles_ready());
    }

    /// Verifica que las propiedades se pueden modificar y leer correctamente.
    #[test]
    fn test_property_get_set() {
        let movie = MovieObject::from_video_path("/tmp/test.mkv");
        movie.set_title("Inception");
        movie.set_year(2010);
        movie.set_synopsis("A mind-bending thriller.");

        assert_eq!(movie.title(), "Inception");
        assert_eq!(movie.year(), 2010);
        assert_eq!(movie.synopsis(), "A mind-bending thriller.");
        assert!(!movie.has_metadata());
    }
}
