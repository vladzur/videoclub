# videoclub

# Proyecto "videoclub"

Videoclub es una aplicación para escritorio Gnome que va a reproducir películas almacenadas en el sistema de archivos local o disco externo. Necesita poder mostrar el catálogo de peliculas como una grilla con las carátulas de las películas, debe ser capaz de descargar metadatos y subtitulos.

## 1. Arquitectura de Alto Nivel (Capas del Sistema)

Para mantener el proyecto mantenible y escalable, dividiremos la aplicación en tres capas principales utilizando un modelo guiado por eventos:

```
+-------------------------------------------------------+
|            CAPA DE PRESENTACIÓN (UI)                  |
|     GTK4 + Libadwaita (AdwGridView, AdwWindow)        |
+-------------------------------------------------------+
                           |  (Mensajes/Canales glib)
                           v
+-------------------------------------------------------+
|             CAPA DE LÓGICA DE NEGOCIO                 |
|   - Motor de Escaneo (Walkdir)   - Gestor de Caché   |
|   - Cliente HTTP (Reqwest)       - Base de Datos      |
+-------------------------------------------------------+
                           |
                           v
+-------------------------------------------------------+
|            CAPA DE REPRODUCCIÓN (CORE)                |
|          GStreamer Pipelines / GstPlay                |
+-------------------------------------------------------+
```

## 2. Plan de Implementación por Fases

Hemos estructurado el desarrollo en **4 fases iterativas** (Sprints), priorizando la mitigación de los mayores riesgos técnicos (integración de GObject y GStreamer) en las primeras etapas.

### Fase 1: Base del Proyecto e Infraestructura GObject

*Objetivo: Configurar el sistema de construcción y el puente entre Rust y el sistema de objetos de GNOME.*

1. **Estructura del Repositorio:** Configurar el sistema de construcción **Meson** junto con `Cargo.toml`. Aislar la lógica pura en una **biblioteca** interna de Rust (`videoclub-core`) y la UI en el binario principal.
  
2. **Modelado de Datos (GObject Subclassing):** * Implementar `MovieObject` utilizando las macros de `glib::wrapper!`. Este objeto representará una película en memoria y debe exponer propiedades dinámicas (`title`, `poster_path`, `video_url`, `subtitles_ready`) mediante `glib::Property`.
  
  - Crear el `gio::ListStore` global que servirá como la *Single Source of Truth* (Fuente única de verdad) para la UI.

### Fase 2: El Motor de Vídeo (GStreamer Backend)

*Objetivo: Lograr la reproducción estable de archivos locales sin bloquear la interfaz.*

1. **Integración de GStreamer:** Inicializar el crate `gstreamer` (`gst`). Usar el widget nativo `GtkVideo` o, para un control más fino de los overlays de subtítulos, implementar un `GtkPicture` conectado a un app-sink de GStreamer.
  
2. **Pipeline de Reproducción:** * Configurar un pipeline estándar: `playbin` (o el moderno `playbin3`) que resuelve automáticamente el demuxing, decodificación por hardware (VAAPI/NVDEC) y sincronización de audio/video.
  
  - **Arquitectura de Eventos:** Conectar el bus de GStreamer (`gst::Bus`) al bucle de eventos de GTK utilizando `glib::MainContext::channel`, permitiendo que comandos como Pausa, Play o Seek se ejecuten de forma segura desde los hilos de reproducción hacia la UI.

### Fase 3: Escáner Asíncrono y Consumo de APIs

*Objetivo: Poblar el catálogo local y enriquecerlo con datos externos de forma reactiva.*

1. **Motor de Escaneo de Disco:**
  
  - Utilizar el crate `walkdir` para leer de forma recursiva los directorios configurados por el usuario.
    
  - Filtrar por extensiones válidas (`.mp4`, `.mkv`, `.avi`).
    
  - *Regla de Arquitectura:* El escaneo **nunca** ocurre en el hilo de la UI. Se ejecuta en un hilo nativo de Rust (`std::thread` o `tokio`) y envía las rutas encontradas a la UI a través de un canal asíncrono.
    
2. **Capa de Red (Metadatos y Subtítulos):**
  
  - **Metadatos:** Implementar un cliente HTTP ligero con `reqwest` (modo asíncrono) para consultar la API de **TheMovieDB (TMDb)**. Al recibir la respuesta, parsear con `serde_json`.
    
  - **Descarga de Subtítulos:** Conectar con la API de **OpenSubtitles**. Calcular el *hash* del archivo de vídeo local (requerido por OpenSubtitles para un emparejamiento exacto) y descargar el archivo `.srt` en la misma carpeta del vídeo o en un directorio oculto de la app.
    
  - **Gestión de Caché:** Descargar los pósters de las películas directamente a `~/.cache/videoclub/posters/` para evitar peticiones de red redundantes en el próximo inicio.
    

### Fase 4: La Interfaz de Usuario (Libadwaita)

*Objetivo: Crear una experiencia visual moderna, fluida y adaptativa.*

1. **La Rejilla Principal (`AdwGridView`):**
  
  - Diseñar el archivo de interfaz (XML o Blueprint) utilizando un `GtkGridView` dentro de un `GtkScrolledWindow`.
    
  - Configurar el `GtkListItemFactory` para inflar el diseño de cada carátula (Imagen + Título).
    
  - Vincular (*bind*) las propiedades de nuestro `MovieObject` directamente a los widgets de la celda.
    
2. **Vista de Detalle y Reproductor:**
  
  - Implementar un `AdwDialog` o una vista lateral que se active al hacer click en una carátula, mostrando la sinopsis, año y el botón para gestionar/descargar subtítulos.
    
  - Diseñar los controles flotantes sobre el vídeo (barra de progreso, volumen, selector de pista de subtítulos) utilizando estilos CSS de GNOME.
    

## 3. Decisiones Tecnológicas Justificadas (Tech Stack)

| **Componente** | **Tecnología** | **Justificación Arquitectónica** |
| --- | --- | --- |
| **UI Framework** | `gtk4` + `libadwaita` crates | Consistencia visual con el escritorio GNOME moderno, esquinas redondeadas, soporte nativo de Flatpak y rendimiento acelerado por GPU (NGL). |
| **Motor de Video** | `gstreamer` + `gstreamer-play` | Es el estándar industrial en Linux. Soporta decodificación por hardware de manera nativa y se integra perfectamente con las estructuras de GTK4. |
| **E/S Asíncrona** | `tokio` / `glib::MainContext` | `tokio` gestionará las descargas concurrentes de subtítulos y metadatos, mientras que el contexto de `glib` asegura la comunicación segura con los widgets de la UI. |
| **Serialización** | `serde` + `serde_json` | Es el estándar de facto en Rust para parsear las respuestas JSON de las APIs de cine con cero copias en memoria. |

## 4. Riesgos Técnicos y Mitigaciones

1. **Riesgo: Bloqueo de la interfaz al cargar imágenes de carátulas grandes.**
  
  - *Mitigación:* Nunca cargar archivos directamente con `gdk::Texture::from_file` en el hilo principal. Las imágenes deben ser decodificadas asíncronamente en segundo plano, o utilizar `GtkPicture` que maneja parte de la carga de manera eficiente, asegurándose de redimensionar las imágenes en caché a un tamaño máximo (ej. 300x450 píxeles).
2. **Riesgo: Ciclos de referencia en Rust debido a los Callbacks de GTK.**
  
  - *Mitigación:* Al conectar señales de botones o eventos (ej. `connect_clicked`), utilizar sistemáticamente la macro `glib::clone!` para pasar referencias débiles (`@weak`) de los objetos y ventanas, evitando fugas de memoria (*memory leaks*).