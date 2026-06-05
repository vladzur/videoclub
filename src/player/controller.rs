// player/controller.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::rc::Rc;
use gtk::gdk;

use super::pipeline::PlaybackPipeline;
use super::events::PlaybackState;

/// Controlador de reproducción de alto nivel.
///
/// Proporciona una API ergonómica sobre `PlaybackPipeline`.
pub struct PlaybackController {
    pipeline: Rc<PlaybackPipeline>,
}

impl PlaybackController {
    pub fn new() -> Result<Self, String> {
        let pipeline = Rc::new(PlaybackPipeline::new()?);
        Ok(Self { pipeline })
    }

    /// Devuelve el GdkPaintable del video si gtk4paintablesink está disponible.
    pub fn paintable(&self) -> Option<gdk::Paintable> {
        self.pipeline.video_paintable()
    }

    /// Carga un archivo de video.
    pub fn load(&self, path: &str) -> Result<(), String> {
        self.pipeline.load_file(path)
    }

    /// Inicia la reproducción.
    pub fn play(&self) -> Result<(), String> {
        self.pipeline.play()
    }

    /// Pausa la reproducción.
    pub fn pause(&self) -> Result<(), String> {
        self.pipeline.pause()
    }

    /// Alterna entre play y pausa.
    pub fn toggle_play_pause(&self) -> Result<(), String> {
        self.pipeline.toggle_play_pause()
    }

    /// Estado actual del pipeline.
    pub fn state(&self) -> PlaybackState {
        self.pipeline.state()
    }

    /// Detiene la reproducción.
    pub fn stop(&self) -> Result<(), String> {
        self.pipeline.stop()
    }

    /// Seek a una posición en segundos.
    pub fn seek_seconds(&self, seconds: f64) -> Result<(), String> {
        self.pipeline.seek(seconds as u64)
    }

    /// Posición actual en segundos.
    pub fn position_seconds(&self) -> f64 {
        self.pipeline.position_seconds()
    }

    /// Duración total en segundos.
    pub fn duration_seconds(&self) -> f64 {
        self.pipeline.duration_seconds()
    }

    /// Ajusta el volumen (0.0 - 1.0).
    pub fn set_volume(&self, volume: f64) {
        self.pipeline.set_volume(volume);
    }

    /// Configura la fuente de los subtítulos.
    /// `font_desc` es una descripción de fuente Pango (ej. "Sans 18").
    pub fn set_subtitle_font(&self, font_desc: &str) {
        self.pipeline.set_subtitle_font(font_desc);
    }
}
