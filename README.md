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

<img width="1575" height="326" alt="image" src="https://github.com/user-attachments/assets/a1310292-087f-4d1c-9e73-b2d2f3ca4f51" />


### Ranking de canciones más escuchadas

Muestra todas las canciones con al menos N reproducciones:

```bash
cargo run --release -- --top 150
```

<img width="1085" height="316" alt="image" src="https://github.com/user-attachments/assets/f9220c56-0b62-4148-8c9b-33de6e0c881a" />


### Estadísticas generales

Tabla por año con streams totales, canciones únicas y tiempo de escucha:

```bash
cargo run --release -- --stats
```

<img width="762" height="507" alt="image" src="https://github.com/user-attachments/assets/060bdaca-010c-45e1-930e-ed2ee450d764" />


## Dependencias

- **serde** + **serde_json** - deserialización de JSON
- **rayon** - paralelismo (procesa los archivos concurrentemente)
