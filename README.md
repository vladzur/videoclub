# Videoclub

![GNOME](https://img.shields.io/badge/GNOME-48-4a86cf?logo=gnome)
![Rust](https://img.shields.io/badge/Rust-1.82+-orange?logo=rust)
![License](https://img.shields.io/badge/License-GPL--3.0--or--later-blue)
![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20Flatpak-lightgrey)

**Videoclub** es una aplicación de escritorio para GNOME que te permite explorar, organizar y reproducir tu colección de películas almacenadas localmente. Escanea directorios del sistema de archivos, descarga metadatos y pósters desde OMDb, obtiene subtítulos desde OpenSubtitles, y reproduce video usando GStreamer con aceleración por hardware.

<p align="center">
  <img src="data/icons/hicolor/scalable/apps/com.vladzur.videoclub.svg" alt="Videoclub icon" width="128">
</p>

---

## ✨ Funcionalidades

### Catálogo de Películas

- **Escaneo recursivo** de directorios configurados por el usuario con detección automática de archivos de video (`.mp4`, `.mkv`, `.avi`, y más).
- **Vista en grilla** con carátulas usando `GtkGridView` y widgets `AdwGridView` responsivos (2–6 columnas según el ancho de ventana).
- **Búsqueda y filtrado** por título en tiempo real con `GtkStringFilter` (búsqueda parcial, sin distinción de mayúsculas).
- **Menú contextual** (clic derecho) sobre las tarjetas con opción de editar metadatos.
- **Drag & drop** — arrastra archivos de video directamente sobre la ventana para reproducirlos al instante.

### Metadatos y Enriquecimiento

- **Parseo inteligente de nombres de archivo** — extrae el título y año usando expresiones regulares (ej: `The.Matrix.1999.1080p.mkv` → título "The Matrix", año 1999).
- **Búsqueda en OMDb** — obtiene título oficial, año, sinopsis, póster, IMDb ID, género y duración.
- **Búsqueda en TMDb** — cliente alternativo para obtener metadatos desde The Movie Database.
- **Descarga y caché de pósters** en `~/.cache/videoclub/posters/` con limpieza automática de entradas antiguas (90 días).
- **Enriquecimiento asíncrono** — las búsquedas HTTP nunca bloquean la interfaz; se ejecutan en hilos separados con `tokio`.
- **Edición manual de metadatos** — diálogo dedicado para corregir o personalizar título, año, sinopsis y parámetros de búsqueda.
- **Persistencia de metadatos** en `~/.local/share/videoclub/library.json` para preservar los datos entre sesiones.

### Subtítulos

- **Búsqueda por hash SHA-256** — calcula el hash del archivo de video para una coincidencia exacta en OpenSubtitles.
- **Búsqueda por nombre** — fallback automático cuando el hash no produce resultados.
- **Descarga automática** de archivos `.srt` junto al video original (ej: `Movie.1999.es.srt`).
- **Detección de subtítulos existentes** — el diálogo de edición muestra el estado del archivo de subtítulos asociado.
- **Configuración de idioma preferido** para subtítulos y audio (español, inglés, francés, portugués, alemán, italiano).
- **Personalización de fuente** de subtítulos desde Preferencias (descripción Pango, ej: `Sans 18`, `DejaVu Serif Bold 20`).

### Reproductor de Video

- **Motor GStreamer** con `playbin3` — demuxing, decodificación por hardware (VAAPI/NVDEC) y sincronización A/V automática.
- **Integración nativa GTK4** mediante `gtk4paintablesink` — el video se renderiza directamente en un `GtkPicture`.
- **Controles de reproducción:**
  - Play / Pausa
  - Seek (barra de progreso interactiva)
  - Control de volumen con íconos contextuales (mute, bajo, medio, alto)
  - Pantalla completa con toggle y atajo F11
- **Auto-ocultación de controles** en pantalla completa tras 3 segundos de inactividad; reaparecen al mover el mouse.
- **Inhibición del screensaver** vía D-Bus (`org.freedesktop.ScreenSaver`) durante la reproducción. Se libera automáticamente al pausar, detener o cerrar.
- **Filtro de eventos sintéticos** de X11 para evitar falsos positivos en la detección de movimiento del mouse.
- **Atajos de teclado:** `F11` para fullscreen, `Escape` para salir de fullscreen.

### Preferencias

- **Diálogo de Preferencias** con dos páginas:
  - **APIs** — claves de OMDb y OpenSubtitles (campos de contraseña) con enlaces directos para obtenerlas gratis.
  - **General** — idioma preferido de audio, idioma preferido de subtítulos, y fuente de subtítulos.
- **Persistencia nativa** mediante GSettings/dconf con esquema `com.vladzur.videoclub`.
- Las preferencias se guardan instantáneamente al cambiar cualquier valor.

### Internacionalización

- **Soporte i18n** con `gettext` — plantillas de traducción en `po/`.
- **Textos traducibles** en la interfaz, mensajes de la aplicación y metadatos de AppStream.

---

## 🏗️ Arquitectura

Videoclub sigue una **arquitectura de tres capas** con separación estricta de responsabilidades:

```
┌──────────────────────────────────────────────────┐
│          CAPA DE PRESENTACIÓN (UI)               │
│  GTK4 + Libadwaita                               │
│  Templates XML + CSS + GObject bindings          │
│  Widgets: PosterCard, VideoWidget                │
│  Diálogos: Preferences, Edit Movie, About        │
├──────────────────────────────────────────────────┤
│        CAPA DE LÓGICA DE NEGOCIO                 │
│  videoclub-core (biblioteca sin deps GTK)        │
│  MovieObject (GObject), MovieCatalog (ListStore) │
│  Scanner, OmdbClient, TmdbClient, SubtitlesClient│
│  MovieEnricher, MetadataStore, PosterCache       │
│  AppSettings (GSettings wrapper), FileHash       │
├──────────────────────────────────────────────────┤
│          CAPA DE REPRODUCCIÓN                    │
│  GStreamer playbin3 + gtk4paintablesink          │
│  PlaybackPipeline, PlaybackController            │
│  ScreensaverInhibitor (D-Bus)                    │
└──────────────────────────────────────────────────┘
```

### Workspace de Cargo

El proyecto está organizado como un **workspace de Cargo** con dos miembros:

| Miembro | Descripción |
|---------|-------------|
| `videoclub-core/` | Biblioteca de lógica pura. Sin dependencias de GTK ni GStreamer. Testeable sin display. |
| `src/` (raíz) | Binario principal. UI con GTK4 + Adwaita + GStreamer. |

Esta separación permite probar la lógica de negocio (parseo, clientes HTTP, caching) sin necesidad de un servidor gráfico.

### Estructura de Directorios

```
videoclub/
├── Cargo.toml                        # Workspace root + binario
├── meson.build                       # Build system (Meson + Cargo)
├── com.vladzur.videoclub.json        # Flatpak manifest (GNOME 48 SDK)
│
├── videoclub-core/                   # Biblioteca de lógica
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                    # Declaración de módulos
│       ├── movie.rs                  # MovieObject (GObject con propiedades)
│       ├── catalog.rs                # MovieCatalog (wrapper gio::ListStore)
│       ├── settings.rs               # AppSettings (wrapper gio::Settings)
│       ├── scanner.rs                # Escáner walkdir recursivo
│       ├── filename.rs               # Parser título+año con regex
│       ├── hash.rs                   # Hash SHA-256 para OpenSubtitles
│       ├── cache.rs                  # PosterCache (~/.cache/videoclub/posters/)
│       ├── omdb.rs                   # OmdbClient (API OMDb)
│       ├── tmdb.rs                   # TmdbClient (API TMDb)
│       ├── subtitles.rs              # SubtitlesClient (API OpenSubtitles)
│       ├── enricher.rs               # MovieEnricher (orquestador)
│       └── metadata_store.rs         # MetadataStore (JSON persistente)
│
├── src/                              # Binario principal
│   ├── main.rs                       # Punto de entrada
│   ├── application.rs                # VideoclubApplication (AdwApplication)
│   ├── window.rs                     # VideoclubWindow (lógica principal, ~750 líneas)
│   ├── preferences_dialog.rs         # Diálogo de preferencias
│   ├── edit_movie_dialog.rs          # Diálogo de edición de metadatos
│   ├── style.css                     # Estilos CSS globales
│   │
│   ├── player/                       # Subsistema de reproducción
│   │   ├── mod.rs
│   │   ├── pipeline.rs               # PlaybackPipeline (playbin3 + gtk4paintablesink)
│   │   ├── controller.rs             # PlaybackController (API ergonómica)
│   │   ├── events.rs                 # PlayerEvent, PlaybackState
│   │   ├── inhibit.rs                # ScreensaverInhibitor (D-Bus)
│   │   └── bus.rs                    # Stub — monitoreo del bus
│   │
│   └── widgets/                      # Widgets reutilizables
│       ├── mod.rs
│       ├── poster_card.rs            # PosterCard (celda de grilla)
│       └── video_widget.rs           # VideoWidget (reproductor + controles)
│
├── data/                             # Archivos de distribución
│   ├── com.vladzur.videoclub.gschema.xml      # GSettings schema (9 keys)
│   ├── com.vladzur.videoclub.desktop.in       # Desktop entry
│   ├── com.vladzur.videoclub.metainfo.xml.in  # AppStream metadata
│   ├── com.vladzur.videoclub.service.in       # D-Bus service
│   ├── poster_placeholder.svg                 # Placeholder para pósters
│   └── icons/                                 # Íconos scalable + symbolic
│
└── po/                               # Internacionalización (gettext)
```

### Flujo de Datos Principal

```
main()
  ├─ gstreamer::init()
  ├─ gettext (i18n)
  ├─ gio::Resource::load()
  └─ VideoclubApplication::new() → app.run()
       └─ activate()
            ├─ Crear VideoclubWindow
            ├─ window.set_catalog_store(store)
            │    ├─ Filtro: StringFilter sobre propiedad "title"
            │    ├─ Cadena: ListStore → FilterListModel → SingleSelection → GridView
            │    └─ Factory: SignalListItemFactory → PosterCard
            ├─ Conectar click en tarjeta → open_player()
            ├─ Re-escanear directorios guardados (GSettings)
            └─ window.present()
```

### Comunicación entre Hilos

Toda operación de I/O se ejecuta fuera del hilo principal de GTK:

| Operación | Hilo | Comunicación |
|-----------|------|-------------|
| Escaneo de archivos | `std::thread` | `async_channel` → `glib::spawn_future_local` |
| Enriquecimiento OMDb | `std::thread` + `tokio::Runtime` | `async_channel` → `glib::spawn_future_local` |
| Descarga de subtítulos | `std::thread` + `tokio::Runtime` | Interno en `MovieEnricher` |
| Reproducción GStreamer | Hilo interno de GStreamer | `gst::Bus` → `glib::MainContext::channel` |

### Tipos de Datos Centrales

#### `MovieObject` (GObject)

Propiedades expuestas con bindings automáticos a GTK:

| Propiedad | Tipo | Descripción |
|-----------|------|-------------|
| `title` | `String` | Título de la película |
| `video-path` | `String` | Ruta absoluta al archivo de video |
| `poster-path` | `String` | Ruta al póster cacheado localmente |
| `year` | `i32` | Año de estreno (0 = desconocido) |
| `synopsis` | `String` | Sinopsis / descripción |
| `subtitles-ready` | `bool` | Subtítulos descargados |
| `has-metadata` | `bool` | Metadatos enriquecidos desde API |
| `file-hash` | `String` | Hash para OpenSubtitles |
| `imdb-id` | `String` | Identificador IMDb |

#### `MovieCatalog`

Wrapper sobre `gio::ListStore` que actúa como **Single Source of Truth** para toda la UI. Cualquier cambio en el `ListStore` se refleja automáticamente en la grilla gracias al binding de GTK.

#### `MetadataStore`

Persistencia JSON en `~/.local/share/videoclub/library.json`. Almacena un mapa `video_path → StoredMetadata` con todos los metadatos de cada película, incluyendo parámetros de búsqueda personalizados.

#### `AppSettings`

Wrapper tipado sobre `gio::Settings` con 9 claves de configuración: directorios de escaneo, API keys, idiomas preferidos, dimensiones de ventana, y fuente de subtítulos.

### Pipeline de Video

```
playbin3
├── video-sink → gtk4paintablesink  (render nativo en GtkPicture)
└── audio-sink → autoaudiosink      (PulseAudio/PipeWire automático)
```

### Tecnologías Clave

| Componente | Tecnología | Justificación |
|------------|------------|---------------|
| **UI Framework** | `gtk4` + `libadwaita` | Consistencia visual con GNOME moderno, esquinas redondeadas, soporte Flatpak, rendimiento acelerado por GPU (NGL) |
| **Motor de Video** | `gstreamer` (playbin3) | Estándar industrial en Linux. Decodificación por hardware nativa. Integración perfecta con GTK4 |
| **E/S Asíncrona** | `tokio` + `glib::MainContext` | `tokio` para descargas HTTP concurrentes. `glib` para comunicación segura con widgets de UI |
| **HTTP Client** | `reqwest` (rustls-tls) | Cliente async sin dependencias de sistema. Soporte JSON nativo |
| **Serialización** | `serde` + `serde_json` | Parseo de respuestas JSON de APIs con cero copias en memoria |
| **Build System** | Meson + Cargo | Meson orquesta recursos, i18n, y Flatpak. Cargo compila Rust |
| **Empaquetado** | Flatpak (GNOME 48 SDK) | Distribución sandboxed multiplataforma. Runtime actualizado |

### APIs Externas

| API | URL Base | Auth | Uso |
|-----|----------|------|-----|
| OMDb | `http://www.omdbapi.com/` | `?apikey=` | Metadatos, pósters |
| OpenSubtitles | `https://api.opensubtitles.com/api/v1/` | Header `Api-Key` | Búsqueda y descarga de subtítulos |
| TMDb | `https://api.themoviedb.org/3/` | `?api_key=` | Metadatos alternativos |

---

## 🚀 Instalación

### Requisitos

- **GNOME 48** (o superior)
- **Rust 1.82+** (stable)
- **Meson 1.0+**
- **GStreamer 1.24+** con plugins `good`, `bad`, `ugly`
- **GTK4** y **Libadwaita** headers de desarrollo

### Compilación desde Código Fuente

```bash
# Clonar el repositorio
git clone https://github.com/vladzur/videoclub.git
cd videoclub

# Configurar variables de entorno (opcional)
cp .env.example .env
# Edita .env con tus API keys de OMDb y OpenSubtitles

# Compilar con Meson
meson setup _build
meson compile -C _build

# Ejecutar
./_build/src/videoclub
```

### Flatpak (Recomendado)

```bash
# Instalar el SDK de GNOME y la extensión de Rust
flatpak install org.gnome.Sdk//48 org.freedesktop.Sdk.Extension.rust-stable//24.08

# Construir e instalar
flatpak-builder --user --install build-dir com.vladzur.videoclub.json

# Ejecutar
flatpak run com.vladzur.videoclub
```

---

## ⚙️ Configuración

### API Keys

1. **OMDb** — Obtén una clave gratuita en [omdbapi.com/apikey.aspx](https://www.omdbapi.com/apikey.aspx)
2. **OpenSubtitles** — Regístrate en [opensubtitles.com/consumers](https://www.opensubtitles.com/consumers)

Configura las claves desde el diálogo de **Preferencias** (`Ctrl+,`) o mediante variables de entorno en `.env`:

```env
OMDB_API_KEY=tu_api_key
OPENSUBTITLES_API_KEY=tu_api_key
RUST_LOG=info
```

### Directorios de Películas

Al hacer clic en **"Add Folder"**, el directorio seleccionado se escanea recursivamente y se guarda en GSettings. La aplicación recordará estos directorios y los re-escaneará automáticamente al iniciar.

---

## 🤝 Guía de Contribución

¡Las contribuciones son bienvenidas! Aquí te explicamos cómo puedes colaborar.

### Configuración del Entorno de Desarrollo

```bash
# Fork y clon
git clone https://github.com/tu-usuario/videoclub.git
cd videoclub

# Instalar dependencias de desarrollo
# Ubuntu/Debian:
sudo apt install libgtk-4-dev libadwaita-1-dev libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev meson cargo

# Fedora:
sudo dnf install gtk4-devel libadwaita-devel gstreamer1-devel \
  gstreamer1-plugins-base-devel meson cargo

# Configurar build
meson setup _build
meson compile -C _build
```

### Convenciones del Proyecto

| Aspecto | Convención |
|---------|------------|
| **Código** | Inglés (nombres de variables, funciones, clases, módulos, archivos) |
| **Comentarios** | Español (doc comments `///`, inline comments `//`) |
| **Commits** | [Conventional Commits](https://www.conventionalcommits.org/) en inglés |
| **Ramas** | `feat/`, `fix/`, `chore/` desde y hacia `master` |
| **Formato** | Rust estándar (`cargo fmt`) |
| **Linting** | Sin warnings (`cargo clippy -- -D warnings`) |

### Flujo de Trabajo

1. **Crea una rama** desde `master`:
   ```bash
   git checkout -b feat/nombre-de-la-funcionalidad
   ```

2. **Implementa tus cambios** siguiendo las convenciones del proyecto.

3. **Ejecuta las pruebas**:
   ```bash
   # Tests de la biblioteca core (sin necesidad de display)
   cargo test -p videoclub-core

   # Verificar formato y lints
   cargo fmt --check
   cargo clippy -- -D warnings
   ```

4. **Haz commit** con Conventional Commits:
   ```bash
   git commit -m "feat: add series support to the catalog"
   ```

5. **Abre un Pull Request** contra `master`. El PR debe incluir:

   - **Description** — resumen del propósito del cambio.
   - **Changes** — lista técnica de las modificaciones realizadas.
   - **QA** — pasos específicos para probar y verificar que el código funciona correctamente.

### Cómo Agregar Nuevas Funcionalidades

#### Agregar una propiedad a `MovieObject`

1. Editar [`videoclub-core/src/movie.rs`](videoclub-core/src/movie.rs) — agregar campo `#[property]`
2. Actualizar tests en el mismo archivo

#### Agregar un nuevo cliente de API

1. Crear `videoclub-core/src/nueva_api.rs` con el cliente HTTP (`reqwest`)
2. Registrar en [`videoclub-core/src/lib.rs`](videoclub-core/src/lib.rs): `pub mod nueva_api;`
3. Integrar en [`videoclub-core/src/enricher.rs`](videoclub-core/src/enricher.rs) si aplica

#### Agregar un nuevo widget

1. Crear `.rs` + `.ui` en `src/widgets/`
2. Agregar el template `.ui` a [`src/videoclub.gresource.xml`](src/videoclub.gresource.xml)
3. Registrar el módulo en [`src/widgets/mod.rs`](src/widgets/mod.rs)

#### Agregar una nueva preferencia

1. [`data/com.vladzur.videoclub.gschema.xml`](data/com.vladzur.videoclub.gschema.xml) — agregar `<key>`
2. [`videoclub-core/src/settings.rs`](videoclub-core/src/settings.rs) — agregar getter/setter
3. [`src/preferences_dialog.ui`](src/preferences_dialog.ui) — agregar fila en el diálogo
4. [`src/preferences_dialog.rs`](src/preferences_dialog.rs) — conectar señal y guardar

### Reportar Errores

Los bugs se reportan a través de [GitHub Issues](https://github.com/vladzur/videoclub/issues). Incluye:

- Versión de Videoclub
- Distribución y versión de GNOME
- Pasos para reproducir el error
- Logs relevantes (ejecuta con `RUST_LOG=debug`)

### Licencia

Videoclub es software libre bajo la licencia **GNU General Public License v3.0 o posterior** ([GPL-3.0-or-later](COPYING)).

---

## 📝 Créditos

Desarrollado por **Vladimir Zurita** © 2026.

Construido con:
- [GTK4](https://gtk.org/) y [Libadwaita](https://gnome.pages.gitlab.gnome.org/libadwaita/)
- [GStreamer](https://gstreamer.freedesktop.org/)
- [Rust](https://www.rust-lang.org/)
- APIs de [OMDb](https://www.omdbapi.com/), [TMDb](https://www.themoviedb.org/) y [OpenSubtitles](https://opensubtitles.com)
