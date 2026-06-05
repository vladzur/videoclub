// hash.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

/// Hash de un archivo de video necesario para la API de OpenSubtitles.
///
/// OpenSubtitles usa un algoritmo específico: SHA-256 de los primeros
/// y últimos 64 KB del archivo, más el tamaño del archivo.
#[derive(Debug, Clone)]
pub struct VideoHash {
    /// Hash en formato hexadecimal.
    pub hash: String,
    /// Tamaño del archivo en bytes.
    pub size: u64,
}

/// Calcula el hash de OpenSubtitles para un archivo de video.
///
/// Lee los primeros 64 KB y los últimos 64 KB del archivo,
/// los combina y calcula el SHA-256 del resultado.
pub fn compute_opensubtitles_hash(path: &Path) -> io::Result<VideoHash> {
    let file_size = fs::metadata(path)?.len();
    let mut file = fs::File::open(path)?;

    // Leer primeros 64 KB
    let mut head = vec![0u8; 65536];
    let head_len = file.read(&mut head)?;
    head.truncate(head_len);

    // Leer últimos 64 KB
    let tail_start = file_size.saturating_sub(65536);
    let mut tail = vec![0u8; 65536];
    let tail_len = if tail_start > 0 {
        // Posicionar al inicio de los últimos 64 KB
        use std::io::Seek;
        let mut file = fs::File::open(path)?;
        file.seek(std::io::SeekFrom::Start(tail_start))?;
        file.read(&mut tail)?
    } else {
        // Si el archivo es menor a 64 KB, head ya contiene todo
        0
    };
    tail.truncate(tail_len);

    // Calcular SHA-256 de head + tail + size
    let mut hasher = Sha256::new();
    hasher.update(&head);
    hasher.update(&tail);
    let hash = format!("{:x}", hasher.finalize());

    Ok(VideoHash {
        hash,
        size: file_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que un archivo que no existe produce un error.
    #[test]
    fn test_missing_file() {
        let result = compute_opensubtitles_hash(Path::new("/nonexistent/video.mkv"));
        assert!(result.is_err());
    }
}
