use crossterm::{
    cursor, execute, queue,
    style::{self, Attribute, Color, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use filetime::FileTime;
use serde::Deserialize;
use std::fs;
use std::io::{self, Seek, Write};
use std::net::ToSocketAddrs;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;
use suppaftp::FtpStream;

#[derive(Deserialize)]
struct Config {
    host: String,
    port: u16,
    username: String,
    password: String,
    remote_directory: String,
    local_directory: String,
    #[serde(default = "default_timeout")]
    timeout: u64,
    #[serde(default = "default_gui")]
    gui: bool,
}

fn default_timeout() -> u64 {
    15
}

fn default_gui() -> bool {
    true
}

// ─── Output trait: abstrae GUI vs consola ───

trait Output {
    fn log(&mut self, text: &str, level: LogLevel);
    fn progress(&mut self, procesados: u32, total: u32, descargados: u32, omitidos: u32, errores: u32, file: &str);
    fn wait_exit(&mut self);
    fn cleanup(&mut self);
}

#[derive(Clone, Copy)]
enum LogLevel {
    Info,
    Ok,
    Skip,
    Error,
    Title,
}

// ─── Modo consola (sin GUI) ───

struct ConsoleOutput;

impl Output for ConsoleOutput {
    fn log(&mut self, text: &str, level: LogLevel) {
        match level {
            LogLevel::Error => eprintln!("{text}"),
            _ => println!("{text}"),
        }
    }

    fn progress(&mut self, _procesados: u32, _total: u32, _descargados: u32, _omitidos: u32, _errores: u32, _file: &str) {
        // No mostrar barra en modo consola
    }

    fn wait_exit(&mut self) {
        // No esperar en modo consola
    }

    fn cleanup(&mut self) {}
}

// ─── Modo GUI (Norton Commander) ───

const BG_MAIN: Color = Color::DarkBlue;
const FG_TITLE: Color = Color::Yellow;
const FG_TEXT: Color = Color::Cyan;
const FG_OK: Color = Color::Green;
const FG_SKIP: Color = Color::DarkYellow;
const FG_ERR: Color = Color::Red;
const BG_BAR: Color = Color::DarkCyan;
const FG_BAR: Color = Color::White;
const FG_BAR_FILL: Color = Color::Yellow;

struct LogLine {
    text: String,
    color: Color,
}

struct GuiOutput {
    width: u16,
    height: u16,
    lines: Vec<LogLine>,
}

impl GuiOutput {
    fn init() -> io::Result<Self> {
        let (width, height) = terminal::size()?;
        let mut stdout = io::stdout();
        execute!(stdout, terminal::Clear(ClearType::All), cursor::Hide)?;
        stdout.flush()?;

        let ui = Self {
            width,
            height,
            lines: Vec::new(),
        };
        ui.redraw_full()?;
        ui.draw_header(" FTP Downloader ")?;
        Ok(ui)
    }

    fn log_capacity(&self) -> usize {
        if self.height > 6 {
            (self.height - 6) as usize
        } else {
            1
        }
    }

    fn redraw_full(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        let w = self.width as usize;
        for row in 0..self.height {
            queue!(
                stdout,
                cursor::MoveTo(0, row),
                SetBackgroundColor(BG_MAIN),
                SetForegroundColor(FG_TEXT),
                style::Print(" ".repeat(w)),
            )?;
        }
        stdout.flush()
    }

    fn draw_header(&self, title: &str) -> io::Result<()> {
        let mut stdout = io::stdout();
        let w = self.width as usize;
        let inner = w.saturating_sub(2);

        let top = format!("╔{}╗", "═".repeat(inner));
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            SetBackgroundColor(BG_MAIN),
            SetForegroundColor(FG_TITLE),
            SetAttribute(Attribute::Bold),
            style::Print(&top),
        )?;

        let tlen = title.chars().count().min(inner);
        let pad_l = (inner - tlen) / 2;
        let pad_r = inner - tlen - pad_l;
        let mid = format!(
            "║{}{}{}║",
            " ".repeat(pad_l),
            &title[..title.len().min(inner)],
            " ".repeat(pad_r)
        );
        queue!(stdout, cursor::MoveTo(0, 1), style::Print(&mid))?;

        let sep = format!("╠{}╣", "═".repeat(inner));
        queue!(
            stdout,
            cursor::MoveTo(0, 2),
            style::Print(&sep),
            SetAttribute(Attribute::Reset),
        )?;

        stdout.flush()
    }

    fn redraw_log(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        let cap = self.log_capacity();
        let w = self.width as usize;
        let inner = w.saturating_sub(2);

        let start = self.lines.len().saturating_sub(cap);
        let visible = &self.lines[start..];

        for (i, line) in (0..cap).zip(visible.iter()) {
            let row = 3 + i as u16;
            let display: String = line.text.chars().take(inner).collect();
            let pad = inner.saturating_sub(display.chars().count());
            queue!(
                stdout,
                cursor::MoveTo(0, row),
                SetBackgroundColor(BG_MAIN),
                SetForegroundColor(FG_TITLE),
                style::Print("║"),
                SetForegroundColor(line.color),
                style::Print(&display),
                style::Print(" ".repeat(pad)),
                SetForegroundColor(FG_TITLE),
                style::Print("║"),
            )?;
        }

        for i in visible.len()..cap {
            let row = 3 + i as u16;
            queue!(
                stdout,
                cursor::MoveTo(0, row),
                SetBackgroundColor(BG_MAIN),
                SetForegroundColor(FG_TITLE),
                style::Print("║"),
                style::Print(" ".repeat(inner)),
                style::Print("║"),
            )?;
        }

        stdout.flush()
    }

    fn draw_progress_bar(
        &self,
        procesados: u32,
        total: u32,
        descargados: u32,
        omitidos: u32,
        errores: u32,
        current_file: &str,
    ) -> io::Result<()> {
        let mut stdout = io::stdout();
        let w = self.width as usize;
        let inner = w.saturating_sub(2);
        let bar_y = self.height - 3;

        let pct = if total > 0 {
            (procesados as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };

        let sep = format!("╠{}╣", "═".repeat(inner));
        queue!(
            stdout,
            cursor::MoveTo(0, bar_y),
            SetBackgroundColor(BG_MAIN),
            SetForegroundColor(FG_TITLE),
            SetAttribute(Attribute::Bold),
            style::Print(&sep),
        )?;

        let prefix = format!(" {:>3}% [", pct);
        let suffix = "] ";
        let bar_width = inner.saturating_sub(prefix.len() + suffix.len());
        let filled = (bar_width as f64 * procesados as f64 / total.max(1) as f64) as usize;
        let empty = bar_width.saturating_sub(filled);

        queue!(
            stdout,
            cursor::MoveTo(0, bar_y + 1),
            SetBackgroundColor(BG_BAR),
            SetForegroundColor(FG_TITLE),
            style::Print("║"),
            SetForegroundColor(FG_BAR),
            style::Print(&prefix),
            SetForegroundColor(FG_BAR_FILL),
            style::Print("█".repeat(filled)),
            SetForegroundColor(Color::DarkGrey),
            style::Print("░".repeat(empty)),
            SetForegroundColor(FG_BAR),
            style::Print(&suffix),
            SetForegroundColor(FG_TITLE),
            style::Print("║"),
            SetAttribute(Attribute::Reset),
        )?;

        let file_display: String = current_file.chars().take(30).collect();
        let status = format!(
            " {}/{} | Desc:{} Omit:{} Err:{} | {}",
            procesados, total, descargados, omitidos, errores, file_display
        );
        let status_trimmed: String = status.chars().take(inner).collect();
        let status_pad = inner.saturating_sub(status_trimmed.chars().count());

        queue!(
            stdout,
            cursor::MoveTo(0, bar_y + 2),
            SetBackgroundColor(BG_BAR),
            SetForegroundColor(FG_TITLE),
            style::Print("║"),
            SetForegroundColor(FG_BAR),
            style::Print(&status_trimmed),
            style::Print(" ".repeat(status_pad)),
            SetForegroundColor(FG_TITLE),
            style::Print("║"),
            SetAttribute(Attribute::Reset),
        )?;

        stdout.flush()
    }
}

impl Output for GuiOutput {
    fn log(&mut self, text: &str, level: LogLevel) {
        let color = match level {
            LogLevel::Info => FG_TEXT,
            LogLevel::Ok => FG_OK,
            LogLevel::Skip => FG_SKIP,
            LogLevel::Error => FG_ERR,
            LogLevel::Title => FG_TITLE,
        };
        self.lines.push(LogLine {
            text: text.to_string(),
            color,
        });
        self.redraw_log().ok();
    }

    fn progress(&mut self, procesados: u32, total: u32, descargados: u32, omitidos: u32, errores: u32, file: &str) {
        self.draw_progress_bar(procesados, total, descargados, omitidos, errores, file).ok();
    }

    fn wait_exit(&mut self) {
        self.log("", LogLevel::Info);
        self.log(" Presione ENTER para salir...", LogLevel::Info);
        let mut buf = String::new();
        let _ = io::stdin().read_line(&mut buf);
    }

    fn cleanup(&mut self) {
        let mut stdout = io::stdout();
        execute!(
            stdout,
            SetBackgroundColor(Color::Reset),
            SetForegroundColor(Color::Reset),
            SetAttribute(Attribute::Reset),
            cursor::Show,
            cursor::MoveTo(0, self.height),
        )
        .ok();
    }
}

// ─── Main ───

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    let nogui_flag = args.iter().any(|a| a == "--nogui" || a == "-q");

    let config_path = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| "config.toml".to_string());

    let config_text = match fs::read_to_string(&config_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("No se pudo leer {config_path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let config: Config = match toml::from_str(&config_text) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error en configuración: {e}");
            return ExitCode::FAILURE;
        }
    };

    let use_gui = config.gui && !nogui_flag;

    let mut out: Box<dyn Output> = if use_gui {
        match GuiOutput::init() {
            Ok(g) => Box::new(g),
            Err(_) => Box::new(ConsoleOutput),
        }
    } else {
        Box::new(ConsoleOutput)
    };

    if let Err(e) = fs::create_dir_all(&config.local_directory) {
        out.log(&format!(" Error creando directorio {}: {e}", config.local_directory), LogLevel::Error);
        out.cleanup();
        return ExitCode::FAILURE;
    }

    let address = format!("{}:{}", config.host, config.port);
    out.log(&format!(" Conectando a {address}..."), LogLevel::Info);
    out.log(&format!(" Timeout: {} segundos", config.timeout), LogLevel::Info);
    out.progress(0, 0, 0, 0, 0, "Conectando...");

    let timeout = Duration::from_secs(config.timeout);

    let sock_addr = match address.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(a) => a,
            None => {
                out.log(&format!(" No se encontró dirección para {address}"), LogLevel::Error);
                out.wait_exit();
                out.cleanup();
                return ExitCode::FAILURE;
            }
        },
        Err(e) => {
            out.log(&format!(" No se pudo resolver {address}: {e}"), LogLevel::Error);
            out.wait_exit();
            out.cleanup();
            return ExitCode::FAILURE;
        }
    };

    let mut ftp = match FtpStream::connect_timeout(sock_addr, timeout) {
        Ok(f) => f,
        Err(e) => {
            out.log(&format!(" No se pudo conectar a {address}: {e}"), LogLevel::Error);
            out.wait_exit();
            out.cleanup();
            return ExitCode::FAILURE;
        }
    };

    ftp.get_ref().set_read_timeout(Some(timeout)).ok();
    ftp.get_ref().set_write_timeout(Some(timeout)).ok();

    if let Err(e) = ftp.login(&config.username, &config.password) {
        out.log(&format!(" Error de autenticación: {e}"), LogLevel::Error);
        out.wait_exit();
        out.cleanup();
        return ExitCode::FAILURE;
    }

    out.log(&format!(" Autenticado como {}", config.username), LogLevel::Ok);

    if let Err(e) = ftp.cwd(&config.remote_directory) {
        out.log(&format!(" No se pudo acceder a {}: {e}", config.remote_directory), LogLevel::Error);
        out.wait_exit();
        out.cleanup();
        return ExitCode::FAILURE;
    }

    out.log(&format!(" Directorio: {}", config.remote_directory), LogLevel::Info);

    ftp.transfer_type(suppaftp::types::FileType::Binary).ok();

    out.log(" Listando archivos...", LogLevel::Info);

    let listing = match ftp.nlst(None) {
        Ok(l) => l,
        Err(e) => {
            out.log(&format!(" Error al listar archivos: {e}"), LogLevel::Error);
            out.wait_exit();
            out.cleanup();
            return ExitCode::FAILURE;
        }
    };

    let archivos: Vec<&str> = listing
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect();

    let total = archivos.len() as u32;
    out.log(&format!(" Encontrados {total} archivos."), LogLevel::Title);
    out.progress(0, total, 0, 0, 0, "Iniciando...");

    let mut descargados = 0u32;
    let mut omitidos = 0u32;
    let mut errores = 0u32;
    let mut procesados = 0u32;
    let mut archivos_con_error: Vec<(String, String)> = Vec::new();

    for filename in &archivos {
        procesados += 1;
        let local_path = Path::new(&config.local_directory).join(filename);

        out.progress(procesados, total, descargados, omitidos, errores, filename);

        if local_path.exists() {
            out.log(&format!(" SKIP  {filename}"), LogLevel::Skip);
            omitidos += 1;
            out.progress(procesados, total, descargados, omitidos, errores, filename);
            continue;
        }

        match ftp.retr_as_buffer(filename) {
            Ok(mut cursor) => {
                let size = cursor.seek(io::SeekFrom::End(0)).unwrap() as usize;
                cursor.seek(io::SeekFrom::Start(0)).unwrap();
                let data = cursor.into_inner();
                let mut file = match fs::File::create(&local_path) {
                    Ok(f) => f,
                    Err(e) => {
                        let msg = format!("{e}");
                        out.log(&format!(" ERR   {filename}: {msg}"), LogLevel::Error);
                        archivos_con_error.push((filename.to_string(), msg));
                        errores += 1;
                        continue;
                    }
                };
                if let Err(e) = file.write_all(&data) {
                    let msg = format!("{e}");
                    out.log(&format!(" ERR   {filename}: {msg}"), LogLevel::Error);
                    archivos_con_error.push((filename.to_string(), msg));
                    errores += 1;
                    continue;
                }
                if let Ok(remote_time) = ftp.mdtm(filename) {
                    let timestamp = remote_time.and_utc().timestamp();
                    let ft = FileTime::from_unix_time(timestamp, 0);
                    filetime::set_file_mtime(&local_path, ft).ok();
                }
                out.log(&format!(" OK    {filename} ({size} bytes)"), LogLevel::Ok);
                descargados += 1;
            }
            Err(e) => {
                let msg = format!("{e}");
                out.log(&format!(" ERR   {filename}: {msg}"), LogLevel::Error);
                archivos_con_error.push((filename.to_string(), msg));
                errores += 1;
            }
        }

        out.progress(procesados, total, descargados, omitidos, errores, filename);
    }

    let _ = ftp.quit();

    out.progress(total, total, descargados, omitidos, errores, "Completado!");
    out.log("", LogLevel::Info);
    out.log(
        &format!(" Resumen: {descargados} descargados, {omitidos} omitidos, {errores} errores."),
        LogLevel::Title,
    );

    if !archivos_con_error.is_empty() {
        out.log("", LogLevel::Info);
        out.log(" Archivos con error:", LogLevel::Error);
        for (archivo, motivo) in &archivos_con_error {
            out.log(&format!("  - {archivo}: {motivo}"), LogLevel::Error);
        }
    }

    out.wait_exit();
    out.cleanup();

    if errores > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
