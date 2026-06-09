// player/pipeline.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::gdk;
use gtk::prelude::*;
use gstreamer::prelude::*;
use videoclub_core::{error, info, warn};

use super::events::PlaybackState;

/// Representa el pipeline de reproducción de GStreamer.
pub struct PlaybackPipeline {
    /// Elemento principal del pipeline.
    playbin: gstreamer::Element,
    /// Estado actual del pipeline.
    state: std::cell::Cell<PlaybackState>,
    /// Sink de video (gtk4paintablesink si está disponible).
    video_sink: Option<gstreamer::Element>,
}

impl PlaybackPipeline {
    /// Crea un nuevo pipeline de reproducción listo para usar.
    ///
    /// Intenta configurar `gtk4paintablesink` como sink de video
    /// para integración nativa con GTK.
    pub fn new() -> Result<Self, String> {
        gstreamer::init()
            .map_err(|e| format!("Failed to initialize GStreamer: {}", e))?;

        // Usamos playbin en lugar de playbin3 porque playbin (v2) tiene un soporte
        // mucho más robusto para inyectar subtítulos externos en caliente con suburi
        // sin necesidad de manipular GstStreamCollection manualmente.
        let playbin = gstreamer::ElementFactory::make("playbin")
            .build()
            .map_err(|e| format!("Failed to create playbin: {}", e))?;

        // Banderas para asegurar que el pipeline construya video y audio.
        // Omitimos intencionalmente 'text' para que GStreamer NO renderice subtítulos embebidos 
        // ni auto-detectados, garantizando que solo nuestra capa GTK nativa los muestre.
        playbin.set_property_from_str("flags", "video+audio+soft-volume");

        // Intentar configurar gtk4paintablesink para integración nativa con GTK4
        let video_sink = match gstreamer::ElementFactory::make("gtk4paintablesink").build() {
            Ok(sink) => {
                playbin.set_property("video-sink", &sink);
                info!("Usando gtk4paintablesink como sink de video");
                Some(sink)
            }
            Err(_) => {
                warn!("gtk4paintablesink no disponible, usando sink por defecto");
                None
            }
        };

        // Configurar audio-sink explícitamente para evitar problemas en Flatpak.
        // autoaudiosink detecta PulseAudio/PipeWire automáticamente.
        match gstreamer::ElementFactory::make("autoaudiosink").build() {
            Ok(audio_sink) => {
                playbin.set_property("audio-sink", &audio_sink);
                info!("Audio-sink configurado: autoaudiosink");
            }
            Err(e) => {
                warn!("No se pudo crear autoaudiosink: {} — el audio puede no funcionar", e);
            }
        }

        Ok(Self {
            playbin,
            state: std::cell::Cell::new(PlaybackState::Stopped),
            video_sink,
        })
    }

    /// Carga un archivo de video desde una ruta local.
    pub fn load_file(&self, path: &str) -> Result<(), String> {
        let uri = gtk::gio::File::for_path(path).uri();
        self.playbin.set_property("uri", uri.as_str());
        info!("Archivo cargado: {}", uri);
        Ok(())
    }

    /// Inicia o reanuda la reproducción.
    pub fn play(&self) -> Result<(), String> {
        self.playbin
            .set_state(gstreamer::State::Playing)
            .map_err(|e| format!("Failed to set Playing state: {}", e))?;
        self.state.set(PlaybackState::Playing);
        Ok(())
    }

    /// Pausa la reproducción.
    pub fn pause(&self) -> Result<(), String> {
        self.playbin
            .set_state(gstreamer::State::Paused)
            .map_err(|e| format!("Failed to set Paused state: {}", e))?;
        self.state.set(PlaybackState::Paused);
        Ok(())
    }

    /// Alterna entre reproducción y pausa.
    pub fn toggle_play_pause(&self) -> Result<(), String> {
        match self.state.get() {
            PlaybackState::Playing => self.pause(),
            _ => self.play(),
        }
    }

    /// Devuelve el estado actual de reproducción.
    pub fn state(&self) -> PlaybackState {
        self.state.get()
    }

    /// Detiene la reproducción y vuelve al estado Ready.
    pub fn stop(&self) -> Result<(), String> {
        self.playbin
            .set_state(gstreamer::State::Ready)
            .map_err(|e| format!("Failed to set Ready state: {}", e))?;
        self.state.set(PlaybackState::Stopped);
        Ok(())
    }

    /// Busca a una posición específica en segundos.
    pub fn seek(&self, seconds: u64) -> Result<(), String> {
        let position = gstreamer::ClockTime::from_seconds(seconds);
        self.playbin
            .seek_simple(
                gstreamer::SeekFlags::FLUSH | gstreamer::SeekFlags::KEY_UNIT,
                position,
            )
            .map_err(|e| format!("Seek failed: {}", e))?;
        Ok(())
    }

    pub fn set_volume(&self, volume: f64) {
        self.playbin.set_property("volume", volume.clamp(0.0, 1.0));
    }

    /// Devuelve la posición actual de reproducción en segundos, o 0.
    pub fn position_seconds(&self) -> f64 {
        self.playbin
            .query_position::<gstreamer::ClockTime>()
            .map(|t| t.seconds() as f64)
            .unwrap_or(0.0)
    }

    /// Devuelve la duración total del medio en segundos, o 0.
    pub fn duration_seconds(&self) -> f64 {
        self.playbin
            .query_duration::<gstreamer::ClockTime>()
            .map(|t| t.seconds() as f64)
            .unwrap_or(0.0)
    }

    pub fn video_paintable(&self) -> Option<gdk::Paintable> {
        self.video_sink
            .as_ref()
            .and_then(|sink| sink.property::<Option<gdk::Paintable>>("paintable"))
    }
}

impl Drop for PlaybackPipeline {
    fn drop(&mut self) {
        if let Err(e) = self.playbin.set_state(gstreamer::State::Null) {
            error!("Error al detener pipeline en Drop: {}", e);
        }
    }
}
