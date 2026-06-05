// scanner.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::path::PathBuf;

use log::{debug, info, warn};
use walkdir::WalkDir;

/// Extensiones de archivo de video reconocidas por el escáner.
const VALID_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov", "webm", "m4v", "wmv"];

/// Resultado del escaneo de un archivo de video individual.
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// Ruta absoluta al archivo de video.
    pub path: PathBuf,
    /// Nombre del archivo (sin la ruta).
    pub filename: String,
    /// Extensión del archivo (sin el punto).
    pub extension: String,
}

/// Motor de escaneo que recorre recursivamente directorios en busca de archivos de video.
///
/// Está diseñado para ejecutarse en un hilo separado mediante `std::thread::spawn`.
/// Los resultados se envían a través de un `std::sync::mpsc::Sender`.
pub struct Scanner {
    /// Directorios configurados para escanear.
    directories: Vec<PathBuf>,
    /// Extensiones válidas.
    valid_extensions: HashSet<&'static str>,
}

impl Scanner {
    /// Crea un nuevo escáner para los directorios especificados.
    pub fn new(directories: Vec<PathBuf>) -> Self {
        Self {
            directories,
            valid_extensions: VALID_EXTENSIONS.iter().copied().collect(),
        }
    }

    /// Ejecuta el escaneo de forma bloqueante en el hilo actual.
    ///
    /// Llama al `callback` por cada archivo de video encontrado.
    /// Diseñado para ejecutarse dentro de `std::thread::spawn`.
    pub fn scan_blocking(&self, mut callback: impl FnMut(ScanResult)) {
        for dir in &self.directories {
            if !dir.exists() {
                warn!("Directorio no encontrado: {:?}", dir);
                continue;
            }

            info!("Escaneando directorio: {:?}", dir);

            for entry in WalkDir::new(dir).follow_links(true).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() && self.is_valid_video(&entry) {
                    let path = entry.path().to_path_buf();
                    let filename = entry.file_name().to_string_lossy().to_string();
                    let extension = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();

                    let result = ScanResult {
                        path: path.clone(),
                        filename,
                        extension,
                    };

                    debug!("Archivo encontrado: {:?}", path);
                    callback(result);
                }
            }
        }

        info!("Escaneo completado");
    }

    /// Verifica si una entrada de walkdir es un archivo de video válido.
    fn is_valid_video(&self, entry: &walkdir::DirEntry) -> bool {
        entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| self.valid_extensions.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false)
    }
}

/// Función de conveniencia: escanea un directorio y devuelve las rutas de video encontradas.
///
/// Está diseñada para ejecutarse dentro de `std::thread::spawn`.
pub fn scan_directory(path: &str) -> Vec<String> {
    let dir = std::path::PathBuf::from(path);
    let scanner = Scanner::new(vec![dir]);
    let mut results = Vec::new();
    scanner.scan_blocking(|r| {
        results.push(r.path.to_string_lossy().into_owned());
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::sync::Mutex;

    /// Crea un directorio temporal con archivos de prueba variados
    /// y verifica que el escáner filtra correctamente.
    #[test]
    fn test_scanner_filters_valid_extensions() {
        let dir = env::temp_dir().join("videoclub_test_scanner_v2");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Crear archivos de video válidos
        fs::write(dir.join("movie1.mp4"), b"fake").unwrap();
        fs::write(dir.join("movie2.mkv"), b"fake").unwrap();
        fs::write(dir.join("movie3.avi"), b"fake").unwrap();

        // Crear archivos no válidos
        fs::write(dir.join("readme.txt"), b"fake").unwrap();
        fs::write(dir.join("poster.jpg"), b"fake").unwrap();
        fs::write(dir.join("subtitle.srt"), b"fake").unwrap();

        let scanner = Scanner::new(vec![dir.clone()]);

        let results = std::sync::Arc::new(Mutex::new(Vec::new()));
        let results_clone = results.clone();

        scanner.scan_blocking(move |result| {
            results_clone.lock().unwrap().push(result);
        });

        let results = results.lock().unwrap();

        // Limpiar
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(results.len(), 3);
        let extensions: Vec<String> = results.iter().map(|r| r.extension.clone()).collect();
        assert!(extensions.contains(&"mp4".to_string()));
        assert!(extensions.contains(&"mkv".to_string()));
        assert!(extensions.contains(&"avi".to_string()));
    }
}
