# Spotify Streaming History Parser

Parser de archivos JSON del historial extendido de Spotify, escrito en Rust con procesamiento en paralelo usando [rayon](https://github.com/rayon-rs/rayon).

## Setup

1. Pedí tu data en [Spotify Privacy Settings](https://www.spotify.com/account/privacy/) (Extended Streaming History)
2. Copiá la carpeta `Spotify Extended Streaming History/` (con los archivos `Streaming_History_Audio_*.json`) a la raíz del proyecto
3. Compilá y ejecutá:

```bash
cargo build --release
```

## Uso

### Buscar una canción

```bash
cargo run --release -- "Ride"
```

![Búsqueda de canción](screenshots/search.png)

### Ranking de canciones más escuchadas

Muestra todas las canciones con al menos N reproducciones:

```bash
cargo run --release -- --top 150
```

![Ranking top](screenshots/top.png)

### Estadísticas generales

Tabla por año con streams totales, canciones únicas y tiempo de escucha:

```bash
cargo run --release -- --stats
```

![Estadísticas por año](screenshots/stats.png)

## Dependencias

- **serde** + **serde_json** - deserialización de JSON
- **rayon** - paralelismo (procesa los archivos concurrentemente)
