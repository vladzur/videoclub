// tmdb.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use bytes::Bytes;
use log::{debug, error, info};
use reqwest::Client;
use serde::Deserialize;

/// Cliente HTTP para la API de TheMovieDB (TMDb).
pub struct TmdbClient {
    http: Client,
    api_key: String,
    base_url: &'static str,
    image_base_url: String,
    poster_size: String,
}

#[derive(Debug, Deserialize)]
pub struct TmdbSearchResult {
    pub id: u64,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub release_date: Option<String>,
    pub poster_path: Option<String>,
    #[serde(default)]
    pub vote_average: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TmdbSearchResponse {
    results: Vec<TmdbSearchResult>,
}

#[derive(Debug, Deserialize)]
pub struct TmdbMovieDetail {
    pub id: u64,
    pub title: String,
    pub overview: Option<String>,
    pub release_date: Option<String>,
    pub poster_path: Option<String>,
    #[serde(default)]
    pub vote_average: f64,
    #[serde(default)]
    pub genres: Vec<TmdbGenre>,
    pub runtime: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct TmdbGenre {
    #[allow(dead_code)]
    pub id: u32,
    #[allow(dead_code)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct TmdbImageConfig {
    base_url: String,
    poster_sizes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbConfiguration {
    images: TmdbImageConfig,
}

impl TmdbClient {
    /// Crea un nuevo cliente de TMDb con la API key proporcionada.
    pub async fn new(api_key: String) -> Result<Self, String> {
        let http = Client::builder()
            .user_agent("Videoclub/1.2.0")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        info!("Inicializando cliente TMDb...");

        let config = Self::fetch_config(&http, &api_key).await?;

        let poster_size = if config.images.poster_sizes.contains(&"w342".to_string()) {
            "w342".to_string()
        } else if config.images.poster_sizes.contains(&"w500".to_string()) {
            "w500".to_string()
        } else {
            config.images.poster_sizes.last().cloned().unwrap_or_else(|| "original".to_string())
        };

        info!("TMDb configurado: base={}, posters={}", config.images.base_url, poster_size);

        Ok(Self {
            http,
            api_key,
            base_url: "https://api.themoviedb.org/3",
            image_base_url: config.images.base_url,
            poster_size,
        })
    }

    /// Busca películas por título y año opcional.
    pub async fn search_movie(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<TmdbSearchResult>, String> {
        let url = format!("{}/search/movie", self.base_url);
        debug!("Buscando película: {} (año: {:?})", query, year);

        let mut params = vec![
            ("api_key", self.api_key.as_str()),
            ("query", query),
            ("language", "es-ES"),
        ];

        let year_str;
        if let Some(y) = year {
            year_str = y.to_string();
            params.push(("year", &year_str));
        }

        let response = self
            .http
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!("TMDb search failed: {} - {}", status, body);
            return Err(format!("TMDb API error: {}", status));
        }

        let body: TmdbSearchResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        info!("Encontrados {} resultados para '{}'", body.results.len(), query);
        Ok(body.results)
    }

    /// Obtiene los detalles completos de una película por su ID de TMDb.
    pub async fn get_movie_details(&self, tmdb_id: u64) -> Result<TmdbMovieDetail, String> {
        let url = format!("{}/movie/{}", self.base_url, tmdb_id);
        debug!("Obteniendo detalles de película ID={}", tmdb_id);

        let response = self
            .http
            .get(&url)
            .query(&[
                ("api_key", self.api_key.as_str()),
                ("language", "es-ES"),
            ])
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("TMDb API error: {}", response.status()));
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Descarga los bytes de un póster desde TMDb.
    pub async fn download_poster(&self, poster_path: &str) -> Result<Bytes, String> {
        let url = format!("{}{}{}", self.image_base_url, self.poster_size, poster_path);
        debug!("Descargando póster: {}", url);

        let response = self
            .http
            .get(&url)
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

    /// Obtiene la configuración de imágenes desde la API de TMDb.
    async fn fetch_config(http: &Client, api_key: &str) -> Result<TmdbConfiguration, String> {
        let url = "https://api.themoviedb.org/3/configuration";
        let response = http
            .get(url)
            .query(&[("api_key", api_key)])
            .send()
            .await
            .map_err(|e| format!("Failed to fetch config: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Config API error: {}", response.status()));
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse config: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_search_results() {
        let json = r#"{
            "results": [
                {
                    "id": 603,
                    "title": "The Matrix",
                    "overview": "A computer hacker learns about the true nature of reality.",
                    "release_date": "1999-03-31",
                    "poster_path": "/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg",
                    "vote_average": 8.2
                }
            ]
        }"#;

        let response: TmdbSearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].title.as_deref(), Some("The Matrix"));
        assert_eq!(response.results[0].id, 603);
    }

    #[test]
    fn test_deserialize_minimal_result() {
        let json = r#"{
            "results": [
                {
                    "id": 123
                }
            ]
        }"#;

        let response: TmdbSearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].id, 123);
    }
}
