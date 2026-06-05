# Videoclub — Documento Técnico de Referencia

> Última actualización: 2026-06-05
> GNOME Builder + Flatpak GNOME 48 SDK

---

## 1. Visión General

Videoclub es una aplicación de escritorio GTK4/Libadwaita para **explorar y reproducir películas** almacenadas en el sistema de archivos local. Escanea directorios, descarga metadatos de OMDb, pósters, y subtítulos de OpenSubtitles, y reproduce video usando GStreamer.

**Plataforma objetivo:** GNOME 48+ (Flatpak)
**Lenguaje:** Rust (edition 2021)
**Build system:** Meson (orquestador) + Cargo (compilador)
**Licencia:** GPL-3.0-or-later

---

## 2. Arquitectura de Capas

```
┌─────────────────────────────────────────┐
│  CAPA DE PRESENTACIÓN (UI)              │
│  GTK4 + Libadwaita                      │
│  Templates XML + CSS + GObject bindings │
├─────────────────────────────────────────┤
│  CAPA DE LÓGICA DE NEGOCIO              │
│  Escáner, HTTP clients, caché, parser   │
│  (videoclub-core — sin deps GTK)        │
├─────────────────────────────────────────┤
│  CAPA DE REPRODUCCIÓN                   │
│  GStreamer playbin3 + gtk4paintablesink │
└─────────────────────────────────────────┘
```

Separación en **workspace de Cargo** con dos miembros:
- `videoclub-core/` — biblioteca pura (testeable sin display, solo `glib` + `gio`)
- `src/` — binario principal (GTK4 + Adwaita + GStreamer + UI)

---

## 3. Estructura del Proyecto

```
videoclub/
├── Cargo.toml                     # Workspace root + binario
├── meson.build                    # Meson root
├── com.vladzur.videoclub.json     # Flatpak manifest (GNOME 48)
│
├── videoclub-core/                # ─── BIBLIOTECA DE LÓGICA ───
│   ├── Cargo.toml                 # glib, gio, serde, reqwest, walkdir, sha2...
│   └── src/
│       ├── lib.rs                 # Declaración de módulos
│       ├── movie.rs               # MovieObject (GObject subclass con #[properties])
│       ├── catalog.rs             # MovieCatalog (wrapper gio::ListStore)
│       ├── settings.rs            # AppSettings (wrapper gio::Settings)
│       ├── scanner.rs             # Scanner (walkdir recursivo)
│       ├── filename.rs            # Parser título+año (regex)
│       ├── hash.rs                # SHA-256 para OpenSubtitles
│       ├── cache.rs               # PosterCache (~/.cache/videoclub/posters/)
│       ├── omdb.rs                # OmdbClient (API OMDb)
│       ├── tmdb.rs                # TmdbClient (API TMDb)
│       ├── subtitles.rs           # SubtitlesClient (API OpenSubtitles)
│       ├── enricher.rs            # MovieEnricher (orquestador OMDb+subs+caché)
│       └── metadata_store.rs      # MetadataStore (JSON persistente)
│
├── src/                           # ─── BINARIO PRINCIPAL ───
│   ├── main.rs                    # Punto de entrada
│   ├── application.rs             # VideoclubApplication (AdwApplication subclass)
│   ├── window.rs                   # VideoclubWindow (~750 líneas, lógica principal)
│   ├── config.rs                   # Constantes (generado desde config.rs.in)
│   ├── preferences_dialog.rs      # Diálogo de preferencias (procedural)
│   ├── edit_movie_dialog.rs       # Diálogo de edición de metadatos
│   ├── style.css                  # Estilos CSS globales
│   ├── meson.build                # Compila gresources + invoca cargo
│   ├── videoclub.gresource.xml    # Registro de recursos
│   │
│   ├── player/                    # ─── SUBSISTEMA DE REPRODUCCIÓN ───
│   │   ├── mod.rs
│   │   ├── pipeline.rs            # PlaybackPipeline (playbin3 + gtk4paintablesink)
│   │   ├── controller.rs          # PlaybackController (API ergonómica)
│   │   ├── events.rs              # PlayerEvent, PlaybackState enums
│   │   └── bus.rs                 # Stub — monitoreo del bus pendiente
│   │
│   └── widgets/                   # ─── WIDGETS REUTILIZABLES ───
│       ├── mod.rs
│       ├── poster_card.rs         # PosterCard (celda de grilla)
│       ├── poster_card.ui         # Template XML (aspect frame 2:3)
│       ├── video_widget.rs        # VideoWidget (reproductor + controles)
│       └── video_widget.ui        # Template XML (overlay + header bar)
│
├── data/                          # ─── ARCHIVOS DE DISTRIBUCIÓN ───
│   ├── com.vladzur.videoclub.gschema.xml    # GSettings schema (8 keys)
│   ├── com.vladzur.videoclub.desktop.in     # Desktop entry
│   ├── com.vladzur.videoclub.metainfo.xml.in # AppStream metadata
│   ├── com.vladzur.videoclub.service.in     # D-Bus service
│   ├── poster_placeholder.svg               # SVG placeholder para pósters
│   └── icons/                               # Íconos (scalable + symbolic)
│
└── po/                            # Internacionalización (gettext)
```

---

## 4. Dependencias Clave

### 4.1 Crate `videoclub` (binario)

| Crate | Versión | Propósito |
|-------|---------|-----------|
| `gtk4` | 0.9 | UI framework (feature: `gnome_47`) |
| `libadwaita` | 0.7 | Widgets modernos GNOME (feature: `v1_6`) |
| `gstreamer` | 0.23 | Motor de video |
| `gstreamer-video` | 0.23 | Tipos específicos de video |
| `gstreamer-play` | 0.23 | API de alto nivel (no usada directamente) |
| `gstreamer-audio` | 0.23 | Tipos de audio |
| `tokio` | 1 | Runtime asíncrono (rt-multi-thread, sync) |
| `async-channel` | 2 | Canal multi-productor para hilos |
| `log` + `env_logger` | 0.4 / 0.11 | Logging |
| `gettext-rs` | 0.7 | i18n |
| `dotenvy` | 0.15 | Variables de entorno (.env) |

### 4.2 Crate `videoclub-core` (biblioteca)

| Crate | Versión | Propósito |
|-------|---------|-----------|
| `glib` | 0.20 | Sistema de objetos GObject |
| `gio` | 0.20 | I/O, ListStore, Settings |
| `serde` / `serde_json` | 1 | Serialización JSON |
| `reqwest` | 0.12 | HTTP client (rustls-tls) |
| `walkdir` | 2 | Escaneo recursivo de archivos |
| `regex` | 1 | Parseo de nombres de archivo |
| `sha2` | 0.10 | Hash para OpenSubtitles |
| `directories` | 6 | Directorios XDG (~/.cache, ~/.local/share) |
| `thiserror` | 2 | Derivación de errores |
| `bytes` | 1 | Manejo eficiente de buffers |

---

## 5. Flujo de Datos Principal

### 5.1 Inicio de la Aplicación

```
main()
  ├─ gstreamer::init()
  ├─ gettext (i18n)
  ├─ gio::Resource::load() → registrar recursos
  └─ VideoclubApplication::new() → app.run()
       └─ ApplicationImpl::startup()
            ├─ Cargar CSS desde recurso: /com/vladzur/videoclub/style.css
            └─ ApplicationImpl::activate()
                 ├─ Crear VideoclubWindow
                 ├─ window.set_catalog_store(store, is_test_data=false)
                 │    ├─ Filtrar por título con gtk::StringFilter
                 │    ├─ Envolver en gtk::FilterListModel
                 │    ├─ Envolver en gtk::SingleSelection
                 │    └─ grid.set_model(selection)
                 ├─ Configurar SignalListItemFactory → PosterCard
                 ├─ Conectar grid.connect_activate() → open_player()
                 ├─ Re-escanear directorios guardados (GSettings)
                 └─ window.present()
```

### 5.2 Escaneo de Directorios

```
window.scan_directory(dir)
  └─ std::thread::spawn()
       └─ Scanner::scan_directory(dir) → Vec<String>
            └─ async_channel::Sender → async_channel::Receiver (hilo principal)
                 └─ Para cada archivo:
                      ├─ MovieObject::from_video_path(path)
                      ├─ Restaurar metadatos desde MetadataStore
                      ├─ catalog.add_movie(movie)
                      └─ update_content_stack()
```

### 5.3 Enriquecimiento de Metadatos

```
window.enrich_all_movies()
  └─ std::thread::spawn()
       └─ tokio::runtime::Runtime::new()
            └─ Para cada MovieObject sin has_metadata:
                 └─ MovieEnricher::enrich_metadata(movie)
                      ├─ parse_movie_filename(video_path) → (title, year)
                      ├─ OmdbClient::search_movie(title, year)
                      ├─ OmdbClient::download_poster(url) → PosterCache
                      ├─ movie.set_title/set_year/set_synopsis...
                      ├─ movie.set_poster_path(cached_path)
                      ├─ movie.set_has_metadata(true)
                      ├─ MetadataStore::upsert(StoredMetadata)
                      └─ async_channel → hilo principal → reload_poster()
```

### 5.4 Reproducción de Video

```
window.open_player(movie)
  └─ Cerrar ventana de video previa (si existe)
  └─ Crear adw::Window + VideoWidget
       └─ PlaybackController::new(video_widget.picture)
            └─ PlaybackPipeline::new()
                 ├─ playbin3 + gtk4paintablesink
                 └─ autoaudiosink
       └─ controller.load(movie.video_path())
       └─ controller.play()
       └─ Timer cada 500ms → actualizar barra de progreso
       └─ Conectar:
            ├─ Play/Pause → controller.toggle_play_pause()
            ├─ Seek → controller.seek_seconds()
            ├─ Volumen → controller.set_volume()
            ├─ Fullscreen → window.fullscreen() / unfullscreen()
            └─ Auto-hide controles en fullscreen (3s timeout)
```

---

## 6. Tipos de Datos Centrales

### 6.1 `MovieObject` (GObject)

Archivo: `videoclub-core/src/movie.rs`

Propiedades GObject expuestas (bindings automáticos con GTK):

| Propiedad | Tipo Rust | Tipo GObject | Descripción |
|-----------|-----------|--------------|-------------|
| `title` | `String` | `gchararray` | Título de la película |
| `video-path` | `String` | `gchararray` | Ruta absoluta al archivo |
| `poster-path` | `String` | `gchararray` | Ruta al póster cacheado |
| `year` | `i32` | `gint` | Año (0 = desconocido) |
| `synopsis` | `String` | `gchararray` | Sinopsis |
| `subtitles-ready` | `bool` | `gboolean` | Subtítulos descargados |
| `has-metadata` | `bool` | `gboolean` | Metadatos enriquecidos |
| `file-hash` | `String` | `gchararray` | Hash OpenSubtitles |
| `imdb-id` | `String` | `gchararray` | IMDb ID |

Constructor clave: `MovieObject::from_video_path(path)` — extrae título del nombre del archivo.

### 6.2 `MovieCatalog`

Archivo: `videoclub-core/src/catalog.rs`

Wrapper sobre `gio::ListStore` que sirve como **Single Source of Truth** para la UI. Métodos:
- `store()` → `&gio::ListStore` — para bindear a `GtkGridView`
- `add_movie()`, `remove_movie()`, `clear()`
- `find_by_path()` → `Option<MovieObject>`
- `len()`, `is_empty()`

### 6.3 `StoredMetadata` (persistencia JSON)

Archivo: `videoclub-core/src/metadata_store.rs`

```rust
struct StoredMetadata {
    search_title: String,   // Título usado para buscar
    search_year: Option<i32>,
    title: String,
    year: Option<i32>,
    synopsis: String,
    poster_path: String,
    imdb_id: String,
    has_metadata: bool,
}
```

Persiste en `~/.local/share/videoclub/library.json`. Clave: `video_path` (hash del path como key del mapa JSON).

### 6.4 `AppSettings` (GSettings)

Archivo: `videoclub-core/src/settings.rs`

Wrapper tipado sobre `gio::Settings`. Keys del esquema:

| Key | Tipo | Default | Uso |
|-----|------|---------|-----|
| `scan-directories` | `as` | `[]` | Directorios a escanear |
| `omdb-api-key` | `s` | `""` | API key de OMDb |
| `opensubtitles-api-key` | `s` | `""` | API key de OpenSubtitles |
| `preferred-audio-language` | `s` | `"es"` | Idioma de audio preferido |
| `preferred-subtitle-language` | `s` | `"es"` | Idioma de subtítulos preferido |
| `window-width` | `i` | `1200` | Ancho de ventana |
| `window-height` | `i` | `800` | Alto de ventana |
| `window-maximized` | `b` | `false` | ¿Maximizada? |

---

## 7. Subsistema de Reproducción (GStreamer)

### 7.1 `PlaybackPipeline`

Archivo: `src/player/pipeline.rs`

Pipeline GStreamer:
```
playbin3
├── video-sink → gtk4paintablesink (integración nativa GTK4)
└── audio-sink → autoaudiosink
```

Métodos principales:
- `new()` — crea el pipeline con `playbin3`
- `load_file(path)` — carga URI `file://`
- `play()`, `pause()`, `stop()`, `toggle_play_pause()`
- `seek(seconds)` — seek con `SeekFlags::FLUSH | KEY_UNIT`
- `set_volume(f64)`, `volume()`
- `position_seconds()`, `duration_seconds()`
- `subtitle_track_count()`, `set_subtitle_track()`
- `audio_track_count()`, `set_audio_track()`

### 7.2 `PlaybackController`

Archivo: `src/player/controller.rs`

Wrapper ergonómico sobre `Rc<PlaybackPipeline>`:
- `new(picture: gtk::Picture)` — crea pipeline + asocia al widget
- `load()`, `play()`, `pause()`, `stop()`
- `seek_seconds()`, `set_volume()`
- `duration_seconds()`, `position_seconds()`

### 7.3 `PlayerEvent` / `PlaybackState`

Archivo: `src/player/events.rs`

Enums para el bus de eventos (pendiente de implementar):
```rust
enum PlayerEvent {
    StateChanged(PlaybackState),
    PositionUpdated(u64),      // nanosegundos
    DurationChanged(u64),
    Buffering(i32),            // porcentaje
    Error(String),
    EndOfStream,
    SubtitleTrack(u32, String),
    AudioTrack(u32, String),
}

enum PlaybackState { Stopped, Playing, Paused }
```

---

## 8. Widgets de UI

### 8.1 `PosterCard`

Archivos: `src/widgets/poster_card.rs` + `src/poster_card.ui`

Widget GObject que representa una celda en la grilla. Template:
- `GtkBox` vertical con clase CSS `poster-card`
- `GtkAspectFrame` (ratio 0.6667 = 2:3, clase `poster-frame`)
  - `GtkPicture` (content-fit=cover, clase `poster`)
- `GtkLabel` título (bold, ellipsized)
- `GtkLabel` año (dim-label)
- `GtkImage` badge de subtítulos (application-x-subrip-symbolic)

Método `bind(movie: &MovieObject)`:
1. `bind_property("title", label, "label")` — binding directo
2. `reload_poster()` — carga el póster desde archivo o placeholder SVG
3. `bind_property("year", label, "label")` con `transform_to` (0 → None)
4. `bind_property("subtitles-ready", badge, "visible")` — badge condicional
5. Conecta `notify::poster-path` → `reload_poster()`

Método `unbind()` — limpia bindings al reciclar la celda.

### 8.2 `VideoWidget`

Archivos: `src/widgets/video_widget.rs` + `src/video_widget.ui`

Widget GObject para el reproductor de video. Template:
- `AdwToolbarView` raíz
  - `AdwHeaderBar` (flat, clase `video-header`) con botón fullscreen
  - Contenido: `GtkOverlay`
    - `GtkPicture` (content-fit=contain)
    - `GtkRevealer` overlay (crossfade, bottom) con controles:
      - `GtkScale` de progreso (clase `video-progress`)
      - Botón play/pause
      - `GtkLabel` de tiempo
      - `GtkScaleButton` de volumen

Lógica del widget:
- Timer cada 500ms actualiza barra de progreso + etiqueta de tiempo
- Movimiento del mouse → muestra/oculta controles (3s auto-hide en fullscreen)
- Tecla Escape → sale de fullscreen
- Filtro de eventos de movimiento sintéticos (X11)

### 8.3 Templates Clave

#### `window.ui` (template principal)
- `AdwHeaderBar`: botón search, botón add folder, botón refresh, botón preferences
- `GtkSearchBar` con `GtkSearchEntry`
- `GtkStack` con dos páginas:
  - `"empty"` → `AdwStatusPage` (placeholder + botón "Add Movie Folder")
  - `"catalog"` → `GtkScrolledWindow` + `GtkGridView` (2-6 columnas, single-click activate)

---

## 9. Clientes de API

### 9.1 OMDb (`videoclub-core/src/omdb.rs`)

API: `http://www.omdbapi.com/`
- `search_movie(title, year)` → `OmdbResult`
- `download_poster(url)` → bytes del póster
- `OmdbResult`: Title, Year, Plot, Poster, imdbRating, imdbID, Genre, Runtime

### 9.2 TMDb (`videoclub-core/src/tmdb.rs`)

API: `https://api.themoviedb.org/3/`
- `search_movie(query, year)` → `Vec<TmdbSearchResult>`
- `get_movie_details(tmdb_id)` → `TmdbMovieDetail`
- `download_poster(poster_path)` → bytes del póster
- Obtiene configuración de imágenes en `new()` (tamaño `w342` preferido)

### 9.3 OpenSubtitles (`videoclub-core/src/subtitles.rs`)

API REST v1: `https://api.opensubtitles.com/api/v1/`
- `search_by_hash(hash, language)` → búsqueda exacta por hash SHA-256
- `search_by_name(query, language, year)` → búsqueda textual
- `download_subtitle(download_url)` → contenido `.srt`

---

## 10. Persistencia

### 10.1 MetadataStore

Archivo: `videoclub-core/src/metadata_store.rs`

JSON en `~/.local/share/videoclub/library.json`.
Estructura: mapa `video_path → StoredMetadata`.

Operaciones:
- `load()` — cargar desde disco al iniciar
- `save()` — persistir a disco
- `get(path)` — recuperar metadatos guardados
- `upsert(path, metadata)` — insertar o actualizar
- `clear_metadata(path)` — eliminar entrada

### 10.2 PosterCache

Archivo: `videoclub-core/src/cache.rs`

Pósters en `~/.cache/videoclub/posters/{cache_key}.jpg`:
- `get_cached(cache_key)` → `Option<PathBuf>`
- `cache_poster(cache_key, data)` → `PathBuf`
- `prune(max_age)` → limpia entradas antiguas (auto al Drop, 90 días)

### 10.3 GSettings

Esquema: `com.vladzur.videoclub` (path: `/com/vladzur/videoclub/`)

Persistencia nativa de GNOME vía dconf. Acceso tipado mediante `AppSettings`.

---

## 11. Diálogos

### 11.1 Preferencias (`src/preferences_dialog.rs` + `src/preferences_dialog.ui`)

Template `AdwPreferencesDialog` con 2 páginas:
- **APIs**: `AdwPasswordEntryRow` para OMDb y OpenSubtitles API keys
- **General**: `AdwComboRow` para idioma de subtítulos y audio

El código Rust carga valores actuales desde `AppSettings` y guarda cambios inmediatamente vía señales `changed`.

### 11.2 Editar Metadatos (`src/edit_movie_dialog.rs`)

Diálogo procedural `adw::Dialog`:
- **Search Parameters**: título, año, botón "Fetch from OMDb"
- **Stored Metadata**: título, año, sinopsis, IMDb ID (editables)
- Botón "Fetch" → limpia metadatos actuales → `window.enrich_single_movie()`
- Botón "Save" → escribe cambios a `MovieObject` + `MetadataStore`
- Notify signals conectadas para actualizar campos al recibir metadatos asíncronos

---

## 12. Sistema de Build

### 12.1 Meson (`meson.build` + `src/meson.build`)

```
meson.build (root)
├── project('videoclub', 'rust', version: '0.1.0')
├── subdir('data')    → desktop, metainfo, gschema, icons, D-Bus
├── subdir('src')     → gresource + cargo build
└── subdir('po')      → i18n
```

`src/meson.build`:
1. `gnome.compile_resources()` — compila `videoclub.gresource.xml`
2. `configure_file()` — genera `config.rs` desde `config.rs.in`
3. `custom_target('cargo-build')` — ejecuta `cargo build` con `CARGO_HOME` aislado

### 12.2 Flatpak (`com.vladzur.videoclub.json`)

- **Runtime:** `org.gnome.Platform` 48
- **SDK:** `org.gnome.Sdk` + `org.freedesktop.Sdk.Extension.rust-stable`
- **Finish args:** network, IPC, fallback X11, DRI, Wayland, PulseAudio
- **Build options:** `RUST_BACKTRACE=1`, `RUST_LOG=videoclub=debug`
- **Buildsystem:** Meson

---

## 13. Patrones de Código

### 13.1 Subclassing GObject (CompositeTemplate)

```rust
mod imp {
    #[derive(gtk::CompositeTemplate, Default)]
    #[template(resource = "/com/vladzur/videoclub/mi_widget.ui")]
    pub struct MiWidget {
        #[template_child]
        pub mi_elemento: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MiWidget {
        const NAME: &'static str = "VideoclubMiWidget";
        type Type = super::MiWidget;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for MiWidget {
        fn constructed(&self) {
            self.parent_constructed();
            // Inicialización post-template
        }
    }
    impl WidgetImpl for MiWidget {}
}

glib::wrapper! {
    pub struct MiWidget(ObjectSubclass<imp::MiWidget>)
        @extends gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}
```

### 13.2 Señales con `glib::clone!` (sintaxis nueva)

```rust
obj.connect_signal(glib::clone!(
    #[weak(rename_to = win)] self,
    #[weak] movie,
    move |source, param| {
        // self → win (débil), movie → débil
        // Si el objeto fue destruido, el closure retorna temprano
    }
));
```

### 13.3 Propiedades GObject con `#[properties]`

```rust
#[derive(glib::Properties)]
#[properties(wrapper_type = super::MovieObject)]
pub struct MovieObject {
    #[property(get, set, name = "title")]
    title: RefCell<String>,
    // ...
}
```
Genera automáticamente `title()`, `set_title()`, `connect_title_notify()`, etc.
Los setters toman `impl Borrow<str>` para `String`, `i32` para `i32`.

### 13.4 Comunicación entre hilos

**Escaneo → Hilo principal:**
```rust
let (tx, rx) = async_channel::unbounded::<String>();
std::thread::spawn(move || {
    let paths = Scanner::scan_directory(dir);
    for p in paths { tx.send_blocking(p).ok(); }
});
// En el hilo principal:
rx.attach(glib::clone!(... move |path| { ... }));
```

**Enriquecimiento → Hilo principal:**
```rust
// Thread separado con runtime tokio
std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        enricher.enrich_metadata(&movie).await;
        tx.send_blocking(movie).ok();
    });
});
```

### 13.5 Bindings GObject → Widget

```rust
// Binding directo (título)
movie.bind_property("title", &label, "label")
    .flags(glib::BindingFlags::SYNC_CREATE)
    .build();

// Binding con transformación (año: i32 → String)
movie.bind_property("year", &label, "label")
    .flags(glib::BindingFlags::SYNC_CREATE)
    .transform_to(|_, year: i32| -> Option<String> {
        if year > 0 { Some(year.to_string()) } else { None }
    })
    .build();

// Binding condicional (badge de subtítulos)
movie.bind_property("subtitles-ready", &badge, "visible")
    .flags(glib::BindingFlags::SYNC_CREATE)
    .build();
```

---

## 14. Comunicación entre Componentes

```
VideoclubApplication
  ├── catalog: MovieCatalog (ListStore)
  ├── settings: AppSettings (GSettings)
  └── Crea VideoclubWindow
        ├── Recibe catalog.store() → bindea a GtkGridView
        ├── Recibe settings → preferencias, API keys
        ├── Escanea directorios → puebla el catálogo
        ├── Enriquece metadatos (OMDb/subs) → actualiza MovieObject
        ├── Persiste en MetadataStore (JSON)
        └── Abre VideoWidget + PlaybackController al hacer clic
              └── PlaybackPipeline (GStreamer playbin3)
```

---

## 15. APIs Externas y Configuración

### 15.1 Variables de Entorno (`.env`)

El proyecto usa `dotenvy` para cargar variables desde `.env`:
```
OMDB_API_KEY=tu_api_key
OPENSUBTITLES_API_KEY=tu_api_key
```

También configurables desde el diálogo de Preferencias (se guardan en GSettings/dconf).

### 15.2 APIs Consumidas

| API | URL Base | Auth | Rate Limit |
|-----|----------|------|------------|
| OMDb | `http://www.omdbapi.com/` | `?apikey=` | 1000/día (gratis) |
| OpenSubtitles | `https://api.opensubtitles.com/api/v1/` | Header `Api-Key` | Variable por plan |
| TMDb | `https://api.themoviedb.org/3/` | `?api_key=` | ~40/10s |

---

## 16. Extensión: Cómo Agregar Features

### Agregar una nueva propiedad a MovieObject

1. `videoclub-core/src/movie.rs`: agregar campo `#[property]` al struct `imp::MovieObject`
2. Actualizar tests

### Agregar un nuevo cliente de API

1. `videoclub-core/src/nueva_api.rs`: implementar cliente HTTP con `reqwest`
2. `videoclub-core/src/lib.rs`: agregar `pub mod nueva_api;`
3. `videoclub-core/src/enricher.rs`: integrar en `enrich_metadata()`

### Agregar un nuevo widget

1. Crear `.rs` + `.ui` en `src/widgets/` o `src/`
2. Agregar `.ui` a `src/videoclub.gresource.xml`
3. Registrar módulo en `src/widgets/mod.rs` o `src/main.rs`

### Agregar una nueva preferencia

1. `data/com.vladzur.videoclub.gschema.xml`: agregar `<key>`
2. `videoclub-core/src/settings.rs`: agregar getter/setter
3. `src/preferences_dialog.ui`: agregar fila en el diálogo
4. `src/preferences_dialog.rs`: conectar señal y guardar

### Agregar un nuevo GAction

1. `src/application.rs`: agregar `gio::ActionEntry` en `setup_gactions()`
2. Opcional: `set_accels_for_action()` para atajo de teclado
3. Opcional: agregar item en `primary_menu` de `window.ui`

---

## 17. Convenciones del Proyecto

| Aspecto | Convención |
|---------|------------|
| **Código** | Inglés (nombres, variables, funciones) |
| **Comentarios** | Español (doc comments, inline) |
| **Commits** | Conventional Commits en inglés (`feat:`, `fix:`, `chore:`) |
| **Branches** | `feat/`, `fix/`, `chore/` desde/hacia `master` |
| **Cargo.toml** | Workspace con `videoclub-core` + binario en `src/` |
| **Tests unitarios** | En `videoclub-core` (sin display) |
| **Templates UI** | XML con `CompositeTemplate` + `#[template_child]` |
| **GSettings** | Schema `com.vladzur.videoclub`, wrapper tipado `AppSettings` |

---

## 18. Archivos Clave por Orden de Importancia

1. `src/window.rs` — ~750 líneas, la mayor parte de la lógica de UI
2. `videoclub-core/src/movie.rs` — tipo de datos central (MovieObject)
3. `videoclub-core/src/enricher.rs` — orquestación de APIs externas
4. `src/player/pipeline.rs` — pipeline GStreamer
5. `src/widgets/video_widget.rs` — lógica del reproductor
6. `videoclub-core/src/metadata_store.rs` — persistencia de metadatos
7. `src/application.rs` — entry point de la aplicación GTK
8. `videoclub-core/src/catalog.rs` — Single Source of Truth (ListStore)
9. `videoclub-core/src/scanner.rs` — escaneo de archivos
10. `src/widgets/poster_card.rs` — widget de celda de grilla
