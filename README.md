# FTP Downloader

[![Español](https://img.shields.io/badge/lang-Español-blue)](README.es.md)

A Rust program that downloads all files from a remote FTP directory, featuring a Norton Commander-style TUI.

## Features

- Bulk file download from an FTP server
- Norton Commander-style TUI (blue background, double borders, colors)
- Headless console mode for scripts and automation
- Visual progress bar with percentage and counters
- Scrollable log showing the status of each file
- Preserves original file modification dates
- Skips already downloaded files to avoid duplicates
- Configurable timeout for connection and transfers
- Detailed error report at the end listing all failed files
- Exit code: `0` on success, `1` if any errors occurred
- Configuration via TOML file

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.56 or higher

## Installation

```bash
git clone https://github.com/jimmyalcala/ftp_downloader.git
cd ftp_downloader
```

## Configuration

Edit the `config.toml` file with your FTP server details:

```toml
host = "ftp.example.com"
port = 21
username = "user"
password = "password"
remote_directory = "/remote/path"
local_directory = "./downloads"
# Timeout in seconds (default 15)
timeout = 15
# Show TUI interface (default true)
gui = true
```

| Field              | Description                                    | Default     |
|--------------------|------------------------------------------------|-------------|
| `host`             | FTP server address                             | (required)  |
| `port`             | Connection port                                | (required)  |
| `username`         | FTP username                                   | (required)  |
| `password`         | FTP password                                   | (required)  |
| `remote_directory` | Remote directory path to download from         | (required)  |
| `local_directory`  | Local directory path to save files             | (required)  |
| `timeout`          | Timeout in seconds for connection and transfers| `15`        |
| `gui`              | Show TUI interface (`true`/`false`)            | `true`      |

## Build and Run

### Development mode

```bash
cargo run
```

### Release build (optimized)

```bash
cargo build --release
```

The binary is generated at `target/release/ftp_downloader.exe` (Windows) or `target/release/ftp_downloader` (Linux/Mac).

### Command line options

```bash
# Use default config file (config.toml)
ftp_downloader

# Use a different config file
ftp_downloader my_config.toml

# Disable GUI from command line
ftp_downloader --nogui
ftp_downloader -q

# Combine options
ftp_downloader -q my_config.toml
```

| Flag       | Description                          |
|------------|--------------------------------------|
| `--nogui`  | Run in console mode without TUI      |
| `-q`       | Same as `--nogui` (quiet mode)       |

## Operation Modes

### GUI mode (default)

Norton Commander-style interface with blue background, double borders, scrollable log and progress bar. Waits for ENTER before exiting.

```
╔══════════════════════════════════════════════════╗
║                 FTP Downloader                   ║
╠══════════════════════════════════════════════════╣
║ Connecting to ftp.example.com:21...              ║
║ Authenticated as user                            ║
║ Found 150 files.                                 ║
║ OK    file001.pdf (34521 bytes)                  ║
║ SKIP  file002.pdf                                ║
║ ERR   file003.pdf: timeout                       ║
║ OK    file004.pdf (12045 bytes)                  ║
╠══════════════════════════════════════════════════╣
║  67% [████████████████████░░░░░░░░░░]            ║
║ 100/150 | Desc:85 Omit:10 Err:5 | file100       ║
```

### Console mode (`--nogui` or `gui = false`)

Plain text output, ideal for scripts, cron jobs or integration with other programs. Exits automatically without waiting for input.

```
 Connecting to ftp.example.com:21...
 Authenticated as user
 Found 150 files.
 OK    file001.pdf (34521 bytes)
 SKIP  file002.pdf
 ERR   file003.pdf: timeout
 Summary: 85 downloaded, 10 skipped, 5 errors.

 Files with errors:
  - file003.pdf: timeout
```

## File States

- **OK** (green) - File downloaded successfully
- **SKIP** (yellow) - File skipped because it already exists locally
- **ERR** (red) - Download error; continues with the next file and reports at the end

## Exit Codes

| Code | Meaning                                    |
|------|--------------------------------------------|
| `0`  | All files processed without errors         |
| `1`  | At least one file had an error             |
