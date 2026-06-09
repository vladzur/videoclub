// enricher.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io;
use std::path::Path;

use crate::{debug, info, warn};

use crate::cache::PosterCache;
use crate::filename::parse_movie_filename;
use crate::hash::compute_opensubtitles_hash;
use crate::metadata_store::StoredMetadata;
use crate::movie::MovieObject;
use crate::omdb::OmdbClient;
use crate::subtitles::SubtitlesClient;

/// Orquestador del enriquecimiento de metadatos.
pub struct MovieEnricher {
    omdb: OmdbClient,
    subtitles: SubtitlesClient,
    cache: PosterCache,
}

impl MovieEnricher {
    pub fn new(omdb: OmdbClient, subtitles: SubtitlesClient) -> io::Result<Self> {
        let cache = PosterCache::new()?;
        Ok(Self { omdb, subtitles, cache })
    }

    /// Enriquece un `MovieObject` con metadatos desde OMDb.
    ///
    /// Si `stored` contiene metadatos previos, se usan los parámetros de búsqueda
    /// que el usuario haya personalizado (`search_title`/`search_year`).
    pub async fn enrich_metadata(
        &self,
        movie: &MovieObject,
        stored: Option<&StoredMetadata>,
    ) -> Result<StoredMetadata, String> {
        let path = movie.video_path();
        let path = Path::new(&path);
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("Unknown");

        // Determinar parámetros de búsqueda:
        // Usar overrides del store si existen, de lo contrario parsear el filename.
        let (search_title, search_year) = if let Some(s) = stored {
            (s.search_title.clone(), s.search_year)
        } else {
            let parsed = parse_movie_filename(filename);
            info!(
                "Enriqueciendo: '{}' → título='{}', año={:?}",
                filename, parsed.title, parsed.year
            );
            (parsed.title, parsed.year)
        };

        // Búsqueda en OMDb:
        //   1. Con año (más preciso)
        //   2. Sin año como fallback (OMDb con t= es estricto con el año)
        let result = match self.omdb.search_movie(&search_title, search_year).await? {
            Some(r) => r,
            None => {
                if search_year.is_some() {
                    match self.omdb.search_movie(&search_title, None).await? {
                        Some(r) => {
                            info!("Encontrado sin año para '{}'", search_title);
                            r
                        }
                        None => {
                            info!("No encontrado en OMDb para '{}'", search_title);
                            // Devolver entrada pendiente con los search params inferidos
                            return Ok(StoredMetadata::new_pending(&search_title, search_year));
                        }
                    }
                } else {
                    info!("No encontrado en OMDb para '{}'", search_title);
                    return Ok(StoredMetadata::new_pending(&search_title, search_year));
                }
            }
        };

        // Actualizar las propiedades del MovieObject
        if let Some(title) = &result.title {
            movie.set_title(title.as_str());
        }
        if let Some(year) = result.year_i32() {
            movie.set_year(year);
        }
        if let Some(plot) = &result.plot {
            if !plot.is_empty() && plot != "N/A" {
                movie.set_synopsis(plot.as_str());
            }
        }
        if let Some(id) = &result.imdb_id {
            movie.set_imdb_id(id.as_str());
        }
        if let Some(rating) = &result.imdb_rating {
            if rating != "N/A" {
                movie.set_rating(rating.as_str());
            }
        }
        if let Some(genre) = &result.genre {
            if genre != "N/A" {
                movie.set_genre(genre.as_str());
            }
        }
        if let Some(runtime) = &result.runtime {
            if runtime != "N/A" {
                movie.set_runtime(runtime.as_str());
            }
        }
        movie.set_has_metadata(true);

        // Descargar y cachear póster
        let poster_path_local = if let Some(url) = result.poster_url() {
            let key = result.cache_key();
            match self.fetch_and_cache_poster(key, url, movie).await {
                Ok(p) => Some(p),
                Err(e) => { warn!("No se pudo descargar el póster: {}", e); None }
            }
        } else {
            None
        };

        debug!("Enriquecimiento completado para '{}'", movie.title());

        // Construir StoredMetadata con el resultado
        Ok(StoredMetadata {
            search_title,
            search_year,
            title: Some(movie.title()),
            year: if movie.year() > 0 { Some(movie.year()) } else { None },
            synopsis: {
                let s = movie.synopsis();
                if s.is_empty() { None } else { Some(s) }
            },
            poster_path: poster_path_local,
            imdb_id: {
                let id = movie.imdb_id();
                if id.is_empty() { None } else { Some(id) }
            },
            imdb_rating: {
                let rating = movie.rating();
                if rating.is_empty() { None } else { Some(rating) }
            },
            genre: {
                let genre = movie.genre();
                if genre.is_empty() { None } else { Some(genre) }
            },
            runtime: {
                let runtime = movie.runtime();
                if runtime.is_empty() { None } else { Some(runtime) }
            },
            has_metadata: true,
            subtitle_path: None,
            last_position: None,
        })
    }

    /// Busca y descarga subtítulos para una película.
    /// Delega en la función standalone `download_subtitles_for_movie`.
    pub async fn download_subtitles(
        &self,
        movie: &MovieObject,
        language: &str,
    ) -> Result<Option<String>, String> {
        download_subtitles_for_movie(&self.subtitles, movie, language).await
    }

    /// Descarga un póster y lo cachea localmente. Devuelve la ruta local.
    async fn fetch_and_cache_poster(
        &self,
        cache_key: u64,
        poster_url: &str,
        movie: &MovieObject,
    ) -> Result<String, String> {
        if let Some(cached) = self.cache.get_cached(cache_key) {
            let path = cached.to_string_lossy().to_string();
            movie.set_poster_path(path.clone());
            return Ok(path);
        }

        let data = self.omdb.download_poster(poster_url).await?;
        let local_path = self
            .cache
            .cache_poster(cache_key, &data)
            .map_err(|e| format!("Error al cachear póster: {}", e))?;

        let path = local_path.to_string_lossy().to_string();
        movie.set_poster_path(path.clone());
        Ok(path)
    }
}

/// Busca y descarga subtítulos para una película usando un cliente de OpenSubtitles.
///
/// Función standalone para permitir su uso sin necesidad de un `MovieEnricher`
/// completo (útil para descargas puntuales desde la UI).
pub async fn download_subtitles_for_movie(
    subtitles: &SubtitlesClient,
    movie: &MovieObject,
    language: &str,
) -> Result<Option<String>, String> {
    let path = movie.video_path();
    if path.is_empty() {
        return Ok(None);
    }

    let video_path = Path::new(&path);

    let hash = match compute_opensubtitles_hash(video_path) {
        Ok(h) => h,
        Err(_) => {
            debug!("No se pudo calcular hash, buscando por nombre");
            let title = movie.title();
            let query = if title.is_empty() {
                video_path.file_stem().and_then(|f| f.to_str()).unwrap_or("")
            } else {
                &title
            };
            let results = subtitles
                .search_by_name(query, language, limited_year(movie.year()))
                .await?;
            if let Some(best) = results.first() {
                return download_and_save_subtitle(
                    subtitles, best, video_path, language, movie,
                )
                .await;
            }
            return Ok(None);
        }
    };

    debug!("Hash calculado: {} (tamaño: {})", hash.hash, hash.size);

    let results = subtitles.search_by_hash(&hash, language).await?;
    let results = if results.is_empty() {
        info!("Sin resultados por hash, buscando por nombre...");
        let title = movie.title();
        let query = if title.is_empty() {
            video_path.file_stem().and_then(|f| f.to_str()).unwrap_or("")
        } else {
            &title
        };
        subtitles
            .search_by_name(query, language, limited_year(movie.year()))
            .await?
    } else {
        results
    };

    if let Some(best) = results.first() {
        download_and_save_subtitle(subtitles, best, video_path, language, movie).await
    } else {
        info!("No se encontraron subtítulos en '{}'", language);
        Ok(None)
    }
}

/// Descarga el contenido de un subtítulo y lo guarda junto al archivo de video.
async fn download_and_save_subtitle(
    subtitles: &SubtitlesClient,
    result: &crate::subtitles::SubtitleResult,
    video_path: &Path,
    language: &str,
    movie: &MovieObject,
) -> Result<Option<String>, String> {
    info!("Descargando subtítulo: {}", result.filename);
    let content = subtitles.download_subtitle(result.file_id).await?;
    let srt_path = video_path.with_extension(format!("{}.srt", language));
    std::fs::write(&srt_path, &content)
        .map_err(|e| format!("Error al guardar subtítulo: {}", e))?;
    movie.set_subtitles_ready(true);
    info!("Subtítulo guardado en: {:?}", srt_path);
    Ok(Some(srt_path.to_string_lossy().to_string()))
}

fn limited_year(year: i32) -> Option<i32> {
    if year > 0 { Some(year) } else { None }
}
