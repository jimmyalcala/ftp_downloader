use crossterm::{
    cursor, execute, queue,
    style::{self, Attribute, Color, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use filetime::FileTime;
use serde::Deserialize;
use std::fs;
use std::io::{self, Seek, Write};
use std::path::Path;
use suppaftp::FtpStream;

#[derive(Deserialize)]
struct Config {
    host: String,
    port: u16,
    username: String,
    password: String,
    remote_directory: String,
    local_directory: String,
}

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

struct Ui {
    width: u16,
    height: u16,
    lines: Vec<LogLine>,
}

impl Ui {
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
        Ok(ui)
    }

    /// Cuántas líneas de log caben entre el header (3 filas) y la barra (3 filas)
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

        // Fondo azul completo
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

        // Borde superior
        let top = format!("╔{}╗", "═".repeat(w.saturating_sub(2)));
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            SetBackgroundColor(BG_MAIN),
            SetForegroundColor(FG_TITLE),
            SetAttribute(Attribute::Bold),
            style::Print(&top),
        )?;

        // Título centrado
        let inner = w.saturating_sub(2);
        let tlen = title.chars().count().min(inner);
        let pad_l = (inner - tlen) / 2;
        let pad_r = inner - tlen - pad_l;
        let mid = format!("║{}{}{}║", " ".repeat(pad_l), &title[..title.len().min(inner)], " ".repeat(pad_r));
        queue!(
            stdout,
            cursor::MoveTo(0, 1),
            style::Print(&mid),
        )?;

        // Separador
        let sep = format!("╠{}╣", "═".repeat(inner));
        queue!(
            stdout,
            cursor::MoveTo(0, 2),
            style::Print(&sep),
            SetAttribute(Attribute::Reset),
        )?;

        stdout.flush()
    }

    fn log(&mut self, text: &str, color: Color) -> io::Result<()> {
        self.lines.push(LogLine {
            text: text.to_string(),
            color,
        });
        self.redraw_log()
    }

    fn redraw_log(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        let cap = self.log_capacity();
        let w = self.width as usize;
        let inner = w.saturating_sub(2);

        // Determinar qué líneas mostrar (las últimas que caben)
        let start = if self.lines.len() > cap {
            self.lines.len() - cap
        } else {
            0
        };

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

        // Limpiar filas vacías restantes
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

    fn draw_progress(
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

        // ── Separador ──
        let sep = format!("╠{}╣", "═".repeat(inner));
        queue!(
            stdout,
            cursor::MoveTo(0, bar_y),
            SetBackgroundColor(BG_MAIN),
            SetForegroundColor(FG_TITLE),
            SetAttribute(Attribute::Bold),
            style::Print(&sep),
        )?;

        // ── Barra de progreso ──
        // Formato: "║ 100% [████████░░░░] ║"
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

        // ── Línea de estado ──
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

    fn cleanup(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(
            stdout,
            SetBackgroundColor(Color::Reset),
            SetForegroundColor(Color::Reset),
            SetAttribute(Attribute::Reset),
            cursor::Show,
            cursor::MoveTo(0, self.height),
        )
    }
}

fn main() {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_string());

    let config_text = fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("No se pudo leer {config_path}: {e}"));

    let config: Config =
        toml::from_str(&config_text).unwrap_or_else(|e| panic!("Error en configuración: {e}"));

    fs::create_dir_all(&config.local_directory).unwrap_or_else(|e| {
        panic!("No se pudo crear directorio {}: {e}", config.local_directory)
    });

    let mut ui = Ui::init().expect("Error inicializando terminal");
    ui.draw_header(" FTP Downloader ").ok();

    let address = format!("{}:{}", config.host, config.port);
    ui.log(&format!(" Conectando a {address}..."), FG_TEXT).ok();
    ui.draw_progress(0, 0, 0, 0, 0, "Conectando...").ok();

    let mut ftp = FtpStream::connect(&address)
        .unwrap_or_else(|e| panic!("No se pudo conectar a {address}: {e}"));

    ftp.login(&config.username, &config.password)
        .unwrap_or_else(|e| panic!("Error de autenticación: {e}"));

    ui.log(&format!(" Autenticado como {}", config.username), FG_OK).ok();

    ftp.cwd(&config.remote_directory)
        .unwrap_or_else(|e| panic!("No se pudo acceder a {}: {e}", config.remote_directory));

    ui.log(&format!(" Directorio: {}", config.remote_directory), FG_TEXT).ok();

    ftp.transfer_type(suppaftp::types::FileType::Binary)
        .unwrap_or_else(|e| panic!("Error modo binario: {e}"));

    ui.log(" Listando archivos...", FG_TEXT).ok();

    let listing = ftp
        .nlst(None)
        .unwrap_or_else(|e| panic!("Error al listar archivos: {e}"));

    let archivos: Vec<&str> = listing
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect();

    let total = archivos.len() as u32;
    ui.log(&format!(" Encontrados {total} archivos."), FG_TITLE).ok();
    ui.draw_progress(0, total, 0, 0, 0, "Iniciando...").ok();

    let mut descargados = 0u32;
    let mut omitidos = 0u32;
    let mut errores = 0u32;
    let mut procesados = 0u32;

    for filename in &archivos {
        procesados += 1;
        let local_path = Path::new(&config.local_directory).join(filename);

        ui.draw_progress(procesados, total, descargados, omitidos, errores, filename).ok();

        if local_path.exists() {
            ui.log(&format!(" SKIP  {filename}"), FG_SKIP).ok();
            omitidos += 1;
            ui.draw_progress(procesados, total, descargados, omitidos, errores, filename).ok();
            continue;
        }

        match ftp.retr_as_buffer(filename) {
            Ok(mut cursor) => {
                let size = cursor.seek(io::SeekFrom::End(0)).unwrap() as usize;
                cursor.seek(io::SeekFrom::Start(0)).unwrap();
                let data = cursor.into_inner();
                let mut file = fs::File::create(&local_path).unwrap_or_else(|e| {
                    panic!("No se pudo crear archivo {}: {e}", local_path.display())
                });
                file.write_all(&data).unwrap_or_else(|e| {
                    panic!("Error al escribir {}: {e}", local_path.display())
                });
                if let Ok(remote_time) = ftp.mdtm(filename) {
                    let timestamp = remote_time.and_utc().timestamp();
                    let ft = FileTime::from_unix_time(timestamp, 0);
                    filetime::set_file_mtime(&local_path, ft).ok();
                }
                ui.log(&format!(" OK    {filename} ({size} bytes)"), FG_OK).ok();
                descargados += 1;
            }
            Err(e) => {
                ui.log(&format!(" ERR   {filename}: {e}"), FG_ERR).ok();
                errores += 1;
            }
        }

        ui.draw_progress(procesados, total, descargados, omitidos, errores, filename).ok();
    }

    let _ = ftp.quit();

    ui.draw_progress(total, total, descargados, omitidos, errores, "Completado!").ok();
    ui.log("", FG_TEXT).ok();
    ui.log(
        &format!(" Resumen: {descargados} descargados, {omitidos} omitidos, {errores} errores."),
        FG_TITLE,
    ).ok();

    // Esperar Enter para salir
    ui.log("", FG_TEXT).ok();
    ui.log(" Presione ENTER para salir...", FG_BAR).ok();
    ui.draw_progress(total, total, descargados, omitidos, errores, "Completado!").ok();

    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);

    ui.cleanup().ok();
}
