// subtitles.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use log::{debug, info};
use reqwest::Client;
use serde::Deserialize;

use crate::hash::VideoHash;

/// Cliente HTTP para la API de OpenSubtitles.
pub struct SubtitlesClient {
    http: Client,
    api_key: String,
    user_agent: String,
    base_url: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct SubtitleResult {
    pub id: String,
    pub language: String,
    pub filename: String,
    pub download_url: String,
    #[serde(default)]
    pub rating: f64,
}

#[derive(Debug, Deserialize)]
struct SubtitlesSearchResponse {
    data: Vec<SubtitleData>,
}

#[derive(Debug, Deserialize)]
struct SubtitleData {
    id: String,
    attributes: SubtitleAttributes,
}

#[derive(Debug, Deserialize)]
struct SubtitleAttributes {
    language: String,
    #[allow(dead_code)]
    release: Option<String>,
    #[serde(rename = "download-count")]
    #[allow(dead_code)]
    download_count: Option<u64>,
    #[serde(rename = "new-download-count")]
    #[allow(dead_code)]
    new_download_count: Option<u64>,
    files: Vec<SubtitleFile>,
}

#[derive(Debug, Deserialize)]
struct SubtitleFile {
    file_name: Option<String>,
    file_id: Option<u64>,
}

impl SubtitlesClient {
    /// Crea un nuevo cliente de OpenSubtitles.
    pub fn new(api_key: String) -> Result<Self, String> {
        let http = Client::builder()
            .user_agent("Videoclub/0.1.0")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            http,
            api_key,
            user_agent: "Videoclub/0.1.0".to_string(),
            base_url: "https://api.opensubtitles.com/api/v1",
        })
    }

    /// Busca subtítulos por hash de archivo de video (coincidencia exacta).
    pub async fn search_by_hash(
        &self,
        hash: &VideoHash,
        language: &str,
    ) -> Result<Vec<SubtitleResult>, String> {
        let url = format!("{}/subtitles", self.base_url);
        debug!(
            "Buscando subtítulos por hash: {} (tamaño: {}, idioma: {})",
            hash.hash, hash.size, language
        );

        let response = self
            .http
            .get(&url)
            .header("Api-Key", &self.api_key)
            .header("User-Agent", &self.user_agent)
            .query(&[
                ("moviehash", hash.hash.as_str()),
                ("moviebytesize", &hash.size.to_string()),
                ("languages", &language.to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("OpenSubtitles API error: {}", response.status()));
        }

        let body: SubtitlesSearchResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let results = self.map_results(body.data);
        info!(
            "Encontrados {} subtítulos por hash para idioma '{}'",
            results.len(),
            language
        );

        Ok(results)
    }

    /// Busca subtítulos por nombre de película (búsqueda por texto).
    pub async fn search_by_name(
        &self,
        query: &str,
        language: &str,
        year: Option<i32>,
    ) -> Result<Vec<SubtitleResult>, String> {
        let url = format!("{}/subtitles", self.base_url);
        debug!("Buscando subtítulos por nombre: {} ({:?})", query, year);

        let mut params: Vec<(&str, String)> = vec![
            ("query", query.to_string()),
            ("languages", language.to_string()),
        ];

        let year_str;
        if let Some(y) = year {
            year_str = y.to_string();
            params.push(("year", year_str));
        }

        let response = self
            .http
            .get(&url)
            .header("Api-Key", &self.api_key)
            .header("User-Agent", &self.user_agent)
            .query(&params)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("OpenSubtitles API error: {}", response.status()));
        }

        let body: SubtitlesSearchResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let results = self.map_results(body.data);
        info!(
            "Encontrados {} subtítulos por nombre para '{}'",
            results.len(),
            query
        );

        Ok(results)
    }

    /// Descarga el contenido de un archivo de subtítulos.
    pub async fn download_subtitle(&self, download_url: &str) -> Result<String, String> {
        debug!("Descargando subtítulo: {}", download_url);

        let response = self
            .http
            .get(download_url)
            .header("Api-Key", &self.api_key)
            .header("User-Agent", &self.user_agent)
            .send()
            .await
            .map_err(|e| format!("Download request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to download subtitle: {}",
                response.status()
            ));
        }

        response
            .text()
            .await
            .map_err(|e| format!("Failed to read subtitle text: {}", e))
    }

    /// Convierte los datos de la respuesta de la API a resultados tipados.
    fn map_results(&self, data: Vec<SubtitleData>) -> Vec<SubtitleResult> {
        data.into_iter()
            .filter_map(|item| {
                let attrs = item.attributes;
                let file = attrs.files.first()?;
                let file_id = file.file_id?;

                Some(SubtitleResult {
                    id: item.id,
                    language: attrs.language,
                    filename: file
                        .file_name
                        .clone()
                        .unwrap_or_else(|| format!("subtitle_{}.srt", file_id)),
                    download_url: format!(
                        "https://api.opensubtitles.com/api/v1/download/{}",
                        file_id
                    ),
                    rating: 0.0,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_search_results() {
        let json = r#"{
            "data": [
                {
                    "id": "abc123",
                    "attributes": {
                        "language": "es",
                        "release": "The.Matrix.1999.1080p",
                        "download-count": 1000,
                        "new-download-count": 500,
                        "files": [
                            {
                                "file_name": "The.Matrix.1999.1080p.es.srt",
                                "file_id": 456
                            }
                        ]
                    }
                }
            ]
        }"#;

        let response: SubtitlesSearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].attributes.language, "es");
        assert_eq!(
            response.data[0].attributes.files[0].file_id,
            Some(456)
        );
    }
}
