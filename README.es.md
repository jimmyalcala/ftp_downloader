# FTP Downloader

Programa en Rust que descarga todos los archivos de un directorio FTP remoto, con una interfaz visual estilo Norton Commander.

## Características

- Descarga masiva de archivos desde un servidor FTP
- Interfaz TUI con colores estilo Norton Commander (fondo azul, bordes dobles)
- Modo consola sin GUI para uso en scripts y automatización
- Barra de progreso visual con porcentaje y contadores
- Log con scroll automático mostrando el estado de cada archivo
- Preserva la fecha de modificación original de los archivos
- Omite archivos ya descargados para evitar duplicados
- Timeout configurable para conexión y transferencias
- Reporte detallado de archivos con error al finalizar
- Exit code: `0` si todo OK, `1` si hubo errores
- Configuración mediante archivo TOML

## Instalación rápida

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/jimmyalcala/ftp_downloader/master/install.sh | bash
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/jimmyalcala/ftp_downloader/master/install.ps1 | iex
```

### Descarga manual

Binarios pre-compilados disponibles en la página de [Releases](https://github.com/jimmyalcala/ftp_downloader/releases) para:
- Windows (x86_64)
- macOS (x86_64, Apple Silicon)
- Linux (x86_64)

## Compilar desde código fuente

### Requisitos

- [Rust](https://www.rust-lang.org/tools/install) 1.56 o superior

```bash
git clone https://github.com/jimmyalcala/ftp_downloader.git
cd ftp_downloader
cargo build --release
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
# Timeout en segundos (por defecto 15)
timeout = 15
# Mostrar interfaz gráfica TUI (por defecto true)
gui = true
```

| Campo              | Descripción                                       | Por defecto |
|--------------------|---------------------------------------------------|-------------|
| `host`             | Dirección del servidor FTP                        | (requerido) |
| `port`             | Puerto de conexión                                | (requerido) |
| `username`         | Usuario FTP                                       | (requerido) |
| `password`         | Contraseña FTP                                    | (requerido) |
| `remote_directory` | Ruta del directorio remoto a descargar            | (requerido) |
| `local_directory`  | Ruta local donde se guardarán los archivos        | (requerido) |
| `timeout`          | Timeout en segundos para conexión y transferencias| `15`        |
| `gui`              | Mostrar interfaz TUI (`true`/`false`)             | `true`      |

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

### Opciones de línea de comandos

```bash
# Usar archivo de configuración por defecto (config.toml)
ftp_downloader

# Usar un archivo de configuración diferente
ftp_downloader mi_config.toml

# Desactivar GUI desde línea de comandos
ftp_downloader --nogui
ftp_downloader -q

# Combinar opciones
ftp_downloader -q mi_config.toml
```

| Flag       | Descripción                           |
|------------|---------------------------------------|
| `--nogui`  | Ejecutar en modo consola sin interfaz |
| `-q`       | Igual que `--nogui` (modo silencioso) |

## Modos de operación

### Modo GUI (por defecto)

Interfaz estilo Norton Commander con fondo azul, bordes dobles, scroll y barra de progreso. Espera ENTER al finalizar.

```
╔══════════════════════════════════════════════════╗
║                 FTP Downloader                   ║
╠══════════════════════════════════════════════════╣
║ Conectando a ftp.ejemplo.com:21...               ║
║ Autenticado como usuario                         ║
║ Encontrados 150 archivos.                        ║
║ OK    archivo001.pdf (34521 bytes)               ║
║ SKIP  archivo002.pdf                             ║
║ ERR   archivo003.pdf: timeout                    ║
║ OK    archivo004.pdf (12045 bytes)               ║
╠══════════════════════════════════════════════════╣
║  67% [████████████████████░░░░░░░░░░]            ║
║ 100/150 | Desc:85 Omit:10 Err:5 | archivo100    ║
```

### Modo consola (`--nogui` o `gui = false`)

Salida de texto plano, ideal para scripts, cron jobs o integración con otros programas. Termina automáticamente sin esperar input.

```
 Conectando a ftp.ejemplo.com:21...
 Autenticado como usuario
 Encontrados 150 archivos.
 OK    archivo001.pdf (34521 bytes)
 SKIP  archivo002.pdf
 ERR   archivo003.pdf: timeout
 Resumen: 85 descargados, 10 omitidos, 5 errores.

 Archivos con error:
  - archivo003.pdf: timeout
```

## Estados de archivo

- **OK** (verde) - Archivo descargado correctamente
- **SKIP** (amarillo) - Archivo omitido porque ya existe localmente
- **ERR** (rojo) - Error al descargar; continúa con el siguiente y reporta al final

## Exit codes

| Código | Significado                              |
|--------|------------------------------------------|
| `0`    | Todos los archivos se procesaron sin error |
| `1`    | Hubo al menos un error en algún archivo  |
