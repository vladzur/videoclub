// player/events.rs
//
// Copyright 2026 Vladimir Zurita
//
// SPDX-License-Identifier: GPL-3.0-or-later

/// Eventos que el pipeline de reproducción envía al hilo principal de GTK.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PlayerEvent {
    StateChanged(PlaybackState),
    PositionUpdated(u64),
    DurationChanged(u64),
    Buffering(i32),
    Error(String),
    EndOfStream,
    SubtitleTrack(u32, String),
    AudioTrack(u32, String),
}

/// Estados posibles del pipeline de reproducción.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}
