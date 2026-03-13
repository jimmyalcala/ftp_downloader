# tonyprg - FTP Downloader

Programa en Rust que descarga todos los archivos de un directorio FTP remoto, con una interfaz visual estilo Norton Commander.

## Características

- Descarga masiva de archivos desde un servidor FTP
- Interfaz TUI con colores estilo Norton Commander (fondo azul, bordes dobles)
- Barra de progreso visual con porcentaje y contadores
- Log con scroll automático mostrando el estado de cada archivo
- Preserva la fecha de modificación original de los archivos
- Omite archivos ya descargados para evitar duplicados
- Configuración mediante archivo TOML

## Requisitos

- [Rust](https://www.rust-lang.org/tools/install) 1.56 o superior

## Instalación

```bash
git clone https://github.com/jimmyalcala/tonyprg.git
cd tonyprg
```

## Configuración

Editar el archivo `config.toml` con los datos del servidor FTP:

```toml
host = "ftp.ejemplo.com"
port = 21
username = "usuario"
password = "contraseña"
remote_directory = "/ruta/remota"
local_directory = "./descargas"
```

| Campo              | Descripción                                      |
|--------------------|--------------------------------------------------|
| `host`             | Dirección del servidor FTP                       |
| `port`             | Puerto de conexión (normalmente 21)              |
| `username`         | Usuario FTP                                      |
| `password`         | Contraseña FTP                                   |
| `remote_directory` | Ruta del directorio remoto a descargar           |
| `local_directory`  | Ruta local donde se guardarán los archivos       |

## Compilar y ejecutar

### Modo desarrollo

```bash
cargo run
```

### Compilar release (optimizado)

```bash
cargo build --release
```

El ejecutable se genera en `target/release/ftp_downloader.exe` (Windows) o `target/release/ftp_downloader` (Linux/Mac).

### Usar un archivo de configuración diferente

```bash
cargo run -- mi_config.toml
# o con el ejecutable compilado:
./target/release/ftp_downloader mi_config.toml
```

## Interfaz

```
╔══════════════════════════════════════════════════╗
║                 FTP Downloader                   ║
╠══════════════════════════════════════════════════╣
║ Conectando a ftp.ejemplo.com:21...               ║
║ Autenticado como usuario                         ║
║ Directorio: /ruta/remota                         ║
║ Encontrados 150 archivos.                        ║
║ OK    archivo001.pdf (34521 bytes)               ║
║ SKIP  archivo002.pdf                             ║
║ OK    archivo003.pdf (12045 bytes)               ║
╠══════════════════════════════════════════════════╣
║  67% [████████████████████░░░░░░░░░░]            ║
║ 100/150 | Desc:85 Omit:10 Err:5 | archivo100    ║
```

- **OK** (verde) - Archivo descargado correctamente
- **SKIP** (amarillo) - Archivo omitido porque ya existe localmente
- **ERR** (rojo) - Error al descargar (puede ser un directorio)
