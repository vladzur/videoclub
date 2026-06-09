// omdb.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use bytes::Bytes;
use crate::{debug, error, info};
use reqwest::Client;
use serde::Deserialize;

/// Cliente HTTP para la API de OMDb (Open Movie Database).
/// Una sola llamada devuelve todos los metadatos y la URL del póster.
pub struct OmdbClient {
    http: Client,
    api_key: String,
}

/// Resultado de una búsqueda en OMDb.
#[derive(Debug, Deserialize)]
pub struct OmdbResult {
    #[serde(rename = "Title")]
    pub title: Option<String>,

    #[serde(rename = "Year")]
    pub year: Option<String>,

    #[serde(rename = "Plot")]
    pub plot: Option<String>,

    /// URL directa del póster (o "N/A" si no existe).
    #[serde(rename = "Poster")]
    pub poster: Option<String>,

    #[serde(rename = "imdbRating")]
    pub imdb_rating: Option<String>,

    /// ID de IMDb, ej: "tt0133093".
    #[serde(rename = "imdbID")]
    pub imdb_id: Option<String>,

    #[serde(rename = "Genre")]
    pub genre: Option<String>,

    #[serde(rename = "Runtime")]
    pub runtime: Option<String>,

    /// "True" si se encontró la película, "False" si no.
    #[serde(rename = "Response")]
    pub response: String,

    /// Mensaje de error cuando Response == "False".
    #[serde(rename = "Error")]
    pub error: Option<String>,
}

impl OmdbResult {
    /// Indica si OMDb encontró la película.
    pub fn is_found(&self) -> bool {
        self.response == "True"
    }

    /// Clave numérica para el caché de pósters, derivada del imdbID.
    /// "tt0133093" → 133093
    pub fn cache_key(&self) -> u64 {
        self.imdb_id
            .as_deref()
            .and_then(|id| id.trim_start_matches('t').parse::<u64>().ok())
            .unwrap_or(0)
    }

    /// Devuelve la URL del póster solo si es válida (no "N/A" ni vacía).
    pub fn poster_url(&self) -> Option<&str> {
        self.poster.as_deref().filter(|u| !u.is_empty() && *u != "N/A")
    }

    /// Extrae el año como i32 desde el campo "Year" (puede ser "1999" o "1999–2001").
    pub fn year_i32(&self) -> Option<i32> {
        self.year
            .as_deref()
            .and_then(|y| y.split('–').next())
            .and_then(|y| y.trim().parse().ok())
    }
}

impl OmdbClient {
    /// Crea un nuevo cliente de OMDb. No requiere llamada de configuración previa.
    pub fn new(api_key: String) -> Result<Self, String> {
        let http = Client::builder()
            .user_agent("Videoclub/1.2.0")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        info!("Cliente OMDb inicializado");
        Ok(Self { http, api_key })
    }

    /// Busca una película por título y año opcional.
    /// Devuelve `Ok(Some(result))` si se encontró, `Ok(None)` si no existe.
    pub async fn search_movie(
        &self,
        title: &str,
        year: Option<i32>,
    ) -> Result<Option<OmdbResult>, String> {
        let url = "https://www.omdbapi.com/";
        debug!("Buscando en OMDb: '{}' (año: {:?})", title, year);

        let mut params = vec![
            ("apikey", self.api_key.as_str()),
            ("t", title),
            ("type", "movie"),
            ("plot", "full"),
        ];

        let year_str;
        if let Some(y) = year {
            year_str = y.to_string();
            params.push(("y", &year_str));
        }

        let response = self
            .http
            .get(url)
            .query(&params)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!("OMDb request failed: {} - {}", status, body);
            return Err(format!("OMDb API error: {}", status));
        }

        let result: OmdbResult = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse OMDb response: {}", e))?;

        if result.is_found() {
            info!(
                "Encontrado en OMDb: {:?} ({})",
                result.title,
                result.imdb_id.as_deref().unwrap_or("?")
            );
            Ok(Some(result))
        } else {
            info!(
                "No encontrado en OMDb para '{}': {:?}",
                title,
                result.error
            );
            Ok(None)
        }
    }

    /// Descarga los bytes de un póster desde su URL directa.
    pub async fn download_poster(&self, url: &str) -> Result<Bytes, String> {
        debug!("Descargando póster: {}", url);

        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to download poster: {}", response.status()));
        }

        response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read poster data: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_from_imdb_id() {
        let result = OmdbResult {
            title: None,
            year: None,
            plot: None,
            poster: None,
            imdb_rating: None,
            imdb_id: Some("tt0133093".to_string()),
            genre: None,
            runtime: None,
            response: "True".to_string(),
            error: None,
        };
        assert_eq!(result.cache_key(), 133093);
    }

    #[test]
    fn test_poster_url_na() {
        let result = OmdbResult {
            title: None,
            year: None,
            plot: None,
            poster: Some("N/A".to_string()),
            imdb_rating: None,
            imdb_id: None,
            genre: None,
            runtime: None,
            response: "True".to_string(),
            error: None,
        };
        assert!(result.poster_url().is_none());
    }

    #[test]
    fn test_year_i32() {
        let result = OmdbResult {
            title: None,
            year: Some("1999".to_string()),
            plot: None,
            poster: None,
            imdb_rating: None,
            imdb_id: None,
            genre: None,
            runtime: None,
            response: "True".to_string(),
            error: None,
        };
        assert_eq!(result.year_i32(), Some(1999));
    }
}
