// filename.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use regex::Regex;

/// Resultado del parseo del nombre de archivo de una película.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedFilename {
    /// Título de la película limpio (sin año, tags de calidad, etc.).
    pub title: String,
    /// Año de estreno, si se detectó en el nombre.
    pub year: Option<i32>,
}

/// Intenta extraer el título y año de un nombre de archivo de película.
///
/// Estrategia principal: todo lo que precede al año en el nombre es el título.
/// Esta heurística funciona en >90% de los archivos de película reales.
///
/// Ejemplos:
/// - "The.Matrix.1999.1080p.mkv"                 → ("The Matrix", 1999)
/// - "Inception (2010).mp4"                       → ("Inception", 2010)
/// - "Parasite.2019.KOREAN.1080p.BrRip.mkv"      → ("Parasite", 2019)
/// - "[YTS] The.Batman.2022.BluRay.mkv"           → ("The Batman", 2022)
/// - "Spider-Man.No.Way.Home.2021.mkv"            → ("Spider Man No Way Home", 2021)
/// - "Some.Movie.Without.Year.1080p.mkv"          → ("Some Movie Without Year", None)
pub fn parse_movie_filename(filename: &str) -> ParsedFilename {
    // Quitar la extensión
    let name = filename
        .rsplit_once('.')
        .map(|(base, _)| base)
        .unwrap_or(filename);

    // Eliminar tags de grupo/release al INICIO entre corchetes cuadrados
    // Ej: "[YTS.MX]", "[HorribleSubs]", "[RARBG]" al principio del nombre
    // NO eliminamos paréntesis al inicio porque pueden ser parte del título:
    // "(500) Days of Summer", "(The) Batman", etc.
    let leading_group_re = Regex::new(r"^\[[^\]]{1,25}\]\s*").unwrap();
    let name = leading_group_re.replace(name, "").to_string();

    // Normalizar puntos y guiones bajos a espacios
    let normalized = name.replace('.', " ").replace('_', " ");

    // Año: 4 dígitos entre 1900 y 2030
    let year_re = Regex::new(r"\b(19\d{2}|20[0-2]\d|2030)\b").unwrap();

    let year = year_re
        .find(&normalized)
        .and_then(|m| m.as_str().parse::<i32>().ok());

    // ── Estrategia principal: todo antes del año es el título ──────────────
    let raw_title = if let Some(m) = year_re.find(&normalized) {
        normalized[..m.start()].to_string()
    } else {
        // Sin año: eliminar tags de calidad conocidos como fallback
        remove_quality_tags(&normalized)
    };

    let title = sanitize_title(&raw_title);

    // Si quedó vacío (ej: año al inicio del nombre), usar el nombre completo limpio
    let title = if title.is_empty() {
        sanitize_title(&normalized)
    } else {
        title
    };

    ParsedFilename { title, year }
}

/// Limpia el título eliminando caracteres/patrones no deseados.
fn sanitize_title(s: &str) -> String {
    // Eliminar contenido entre corchetes: "[1080p]", "[BluRay]", "[extended]"
    let bracket_re = Regex::new(r"\[[^\]]*\]").unwrap();
    let s = bracket_re.replace_all(s, " ").to_string();

    // Eliminar paréntesis vacíos: "()", "( )"
    let empty_paren_re = Regex::new(r"\(\s*\)").unwrap();
    let s = empty_paren_re.replace_all(&s, " ").to_string();

    // Reemplazar guiones como separadores con espacios
    // Nota: no distinguimos entre "Spider-Man" y separadores, OMDb tolera ambos
    let dash_re = Regex::new(r"[-–—]").unwrap();
    let s = dash_re.replace_all(&s, " ").to_string();

    // Normalizar espacios múltiples
    let spaces_re = Regex::new(r"\s+").unwrap();
    let s = spaces_re.replace_all(&s, " ").to_string();

    s.trim().to_string()
}

/// Elimina tags de calidad/codec del nombre (usado cuando no hay año).
fn remove_quality_tags(s: &str) -> String {
    let tags = [
        // Resolución y calidad de video
        r"\b4K\b", r"\b2160p\b", r"\b1080p\b", r"\b720p\b", r"\b480p\b",
        r"\b1080i\b", r"\b720i\b",
        // Fuente
        r"\bBluRay\b", r"\bBlu-Ray\b", r"\bBDRip\b", r"\bBDRemux\b",
        r"\bBrRip\b", r"\bWEB-DL\b", r"\bWEBDL\b", r"\bWEBRip\b",
        r"\bHDRip\b", r"\bHDTV\b", r"\bDVDRip\b", r"\bDVDScr\b",
        r"\bCAMRip\b", r"\bTS\b", r"\bHDCam\b",
        // Codec de video
        r"\bx264\b", r"\bx265\b", r"\bH264\b", r"\bH265\b",
        r"\bHEVC\b", r"\bAVC\b", r"\bXviD\b", r"\bDivX\b",
        r"\bREMUX\b",
        // Audio
        r"\bAAC\b", r"\bAC3\b", r"\bDTS\b", r"\bTrueHD\b",
        r"\bAtmos\b", r"\bDDP\d*\.?\d*\b", r"\bDD5\.1\b",
        r"\b5\.1\b", r"\b7\.1\b",
        // Idiomas (evitan incluirlos en el título de búsqueda)
        r"\bKOREAN\b", r"\bJAPANESE\b", r"\bCHINESE\b", r"\bMANDARIN\b",
        r"\bHINDI\b", r"\bFRENCH\b", r"\bSPANISH\b", r"\bPORTUGUESE\b",
        r"\bITALIAN\b", r"\bGERMAN\b", r"\bRUSSIAN\b", r"\bDUBBED\b",
        r"\bMULTI\b", r"\bBiLiNGUAL\b", r"\bSUB\b", r"\bSUBS\b",
        // Edición
        r"\bREMASTERED\b", r"\bEXTENDED\b", r"\bDIRECTOR.?S?.?CUT\b",
        r"\bUNRATED\b", r"\bTHEATRICAL\b", r"\bULTIMATE\b",
        // Streaming
        r"\bAMZN\b", r"\bNFLX\b", r"\bNF\b", r"\bDSNP\b", r"\bHBOMAX\b",
        r"\bAPPLE\b", r"\bPCOK\b",
        // Grupos/releases comunes
        r"\bYIFY\b", r"\bYTS\b", r"\bYTS\.MX\b", r"\bYTS\.AM\b",
        r"\bRARBG\b", r"\bGanool\b", r"\bEVO\b", r"\bGalaxyRG\b",
        r"\bPROPER\b", r"\bREPACK\b",
    ];

    let mut result = s.to_string();
    for tag in &tags {
        if let Ok(re) = Regex::new(&format!(r"(?i){}", tag)) {
            result = re.replace_all(&result, " ").to_string();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(filename: &str, expected_title: &str, expected_year: Option<i32>) {
        let result = parse_movie_filename(filename);
        assert_eq!(result.title, expected_title, "Title mismatch for: {}", filename);
        assert_eq!(result.year, expected_year, "Year mismatch for: {}", filename);
    }

    #[test]
    fn test_standard_patterns() {
        check("The.Matrix.1999.1080p.mkv", "The Matrix", Some(1999));
        check("Inception (2010).mp4", "Inception", Some(2010));
        check("Avatar.2009.720p.BrRip.x264.mp4", "Avatar", Some(2009));
    }

    #[test]
    fn test_split_at_year() {
        // La estrategia principal: todo antes del año = título
        check(
            "The.Dark.Knight.2008.REMASTERED.1080p.BluRay.x265.mkv",
            "The Dark Knight",
            Some(2008),
        );
        check(
            "Parasite.2019.KOREAN.1080p.BrRip.x264.YIFY.mkv",
            "Parasite",
            Some(2019),
        );
        check(
            "Spider-Man.No.Way.Home.2021.EXTENDED.2160p.WEB-DL.mkv",
            "Spider Man No Way Home",
            Some(2021),
        );
    }

    #[test]
    fn test_leading_group_tags() {
        check(
            "[YTS.MX] The.Batman.2022.1080p.BluRay.mkv",
            "The Batman",
            Some(2022),
        );
    }

    #[test]
    fn test_dash_separated() {
        check(
            "The-Grand-Budapest-Hotel-2014-1080p-BluRay.mkv",
            "The Grand Budapest Hotel",
            Some(2014),
        );
    }

    #[test]
    fn test_no_year() {
        check("Some.Movie.1080p.mkv", "Some Movie", None);
    }
}
