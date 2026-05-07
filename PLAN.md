# Plan de Implementación: `cdx-rs` — Migración de Cdx.ps1 a Rust

## Decisiones de Diseño Confirmadas

| Decisión | Elección |
|---|---|
| Ubicación | Proyecto independiente en `~/dev/cdx-rs` |
| Walking (TUI) | `ignore` crate (autónomo, misma librería que fd) |
| Búsqueda global `-g` | Subprocess a `rg` (demasiado complejo reimplementar) |
| Zoxide | Subprocess a `zoxide query` |
| ShowResult (ls post-cd) | En el wrapper PowerShell (4 líneas) |
| Preview (eza/bat/git) | Subprocesos desde Rust (sin overhead de PowerShell) |

---

## 1. Estructura del Proyecto

```
~/dev/cdx-rs/
├── Cargo.toml
├── .gitignore
├── README.md
├── src/
│   ├── main.rs              # Entry point + dispatch
│   ├── cli.rs               # Clap argument parsing
│   ├── config.rs            # Exclude lists, constants
│   ├── tui/
│   │   ├── mod.rs           # Re-exports
│   │   ├── app.rs           # AppState, Mode enum, DirEntryItem
│   │   ├── ui.rs            # ratatui layout + rendering
│   │   └── events.rs        # Event loop, keybindings
│   ├── walker/
│   │   └── mod.rs           # Directory/file walking con `ignore`
│   ├── preview/
│   │   └── mod.rs           # Preview panel (eza, bat, git)
│   ├── search/
│   │   └── mod.rs           # Búsqueda global -g (rg subprocess)
│   └── zoxide/
│       └── mod.rs           # Zoxide query + cache
└── tests/
    └── walker_tests.rs      # Tests de walking + filtros
```

### Cargo.toml

```toml
[package]
name = "cdx-rs"
version = "0.1.0"
edition = "2021"
description = "Interactive directory navigator — Rust rewrite of Cdx.ps1"

[dependencies]
# TUI
ratatui = "0.29"
crossterm = "0.28"

# Fuzzy matching (same algorithm as fzf)
nucleo-matcher = "0.3"

# Directory walking (same library fd/rg use internally)
ignore = "0.4"

# CLI parsing
clap = { version = "4", features = ["derive"] }

# Cross-platform directories
dirs = "5"

# Error handling
anyhow = "1"
thiserror = "1"

[profile.release]
opt-level = 3
lto = true        # Link-time optimization (smaller binary)
strip = true      # Strip symbols
```

---

## 2. Arquitectura: Diagrama de Flujo

```
                         ┌──────────────────────────┐
                         │       cdx-rs.exe         │
                         │                          │
  Args ─────────────────▶│  main.rs (dispatch)      │
                         │                          │
                         │  ┌────────────────────┐  │
  -g <query> ───────────▶│  │ search/mod.rs      │  │──▶ stdout (path)
                         │  │ (rg subprocess)     │  │
                         │  └────────────────────┘  │
                         │                          │
  -h ───────────────────▶│  help text               │──▶ stdout
                         │                          │
  sin args ─────────────▶│  ┌────────────────────┐  │
  <query>  ─────────────▶│  │ tui/ (ratatui)     │  │──▶ stdout (path)
                         │  │                    │  │
                         │  │  events.rs ──┐     │  │
                         │  │  (key loop)  │     │  │
                         │  │       │      │     │  │
                         │  │       ▼      │     │  │
                         │  │  app.rs      │     │  │
                         │  │  (state)     │     │  │
                         │  │   │    │     │     │  │
                         │  │   ▼    ▼     │     │  │
                         │  │ walker  preview  │  │
                         │  │ (ignore) (subproc)│  │
                         │  └────────────────────┘  │
                         └──────────────────────────┘
                                    │
                                    ▼
                         ┌──────────────────────────┐
                         │  PowerShell Wrapper      │
                         │  function cdx {          │
                         │    $t = & cdx-rs @args   │
                         │    cd $t; Show-CdxResult │
                         │  }                       │
                         └──────────────────────────┘
```

---

## 3. Estructuras de Datos Clave

### `AppState` (el corazón de la TUI)

```rust
pub struct App {
    // ── Navegación ──
    pub current_dir: PathBuf,          // Directorio virtual actual
    pub items: Vec<DirEntryItem>,      // Items crudos (todos)
    pub filtered_indices: Vec<usize>,  // Índices tras fuzzy filtering

    // ── Toggles (equivalente al state bitmask) ──
    pub mode: Mode,                    // Find (dirs) o Search (archivos)
    pub show_dotfiles: bool,           // Ctrl+A
    pub show_winhidden: bool,          // Ctrl+W

    // ── UI state ──
    pub list_state: ListState,         // Scroll + selección
    pub query: String,                 // Texto del fuzzy input
    pub cursor_pos: usize,             // Posición del cursor en el input
    pub focus: Focus,                  // List | Input

    // ── Popup / Overlay ──
    pub popup_width_pct: u16,          // % del terminal ancho (default 80)
    pub popup_height_pct: u16,         // % del terminal alto (default 80)

    // ── Salida ──
    pub should_quit: bool,
    pub exit_action: ExitAction,       // Qué hacer al salir

    // ── Cachés ──
    pub zoxide_cache: Vec<PathBuf>,
    pub preview_content: String,
    pub preview_dirty: bool,           // ¿Necesita regenerar preview?

    // ── Matcher ──
    pub matcher: Matcher,              // nucleo::Matcher (se reusa)
}

pub enum Mode {
    Find,     // Listar directorios
    Search,   // Listar archivos
    Results,  // Resultados de búsqueda global (-g) — FUTURO
}

pub struct DirEntryItem {
    pub display: String,      // Nombre para mostrar (ej: "★ dev/project")
    pub rel_path: String,     // Path relativo al current_dir
    pub full_path: PathBuf,   // Path absoluto
    pub is_zoxide: bool,      // Prefijo ★
    pub is_dir: bool,
}

pub enum Focus {
    List,
    Input,
}

pub enum ExitAction {
    OutputPath(PathBuf),      // Normal: imprimir path y salir
    SpawnYazi(PathBuf),       // Ctrl+O: spawn yazi, luego imprimir path
    JustExit,                 // Ctrl+C sin navegar
}
```

---

## 4. Especificación de Módulos

### 4.1 `main.rs` — Entry Point

**Responsabilidad:** Parsear args, dispatchear al modo correcto.

**Flujo:**
```
1. crossterm::terminal::enable_raw_mode()
2. Cli::parse()
3. Match:
   - cli.grep && cli.query no vacío  → run_global_search(query) → exit
   - cli.grep && query vacío          → error: "-g requires a query"
   - cli.query es ["~"|"..."]         → imprimir HOME → exit
   - cli.query es ruta válida         → imprimir ruta → exit
   - cli.query no vacío               → zoxide query → si ok imprimir → exit
                                        → fallback: run_tui(Some(query))
   - sin args                         → run_tui(None)
4. crossterm::terminal::disable_raw_mode()
5. Imprimir exit_path a stdout si existe
```

**Pseudo-código:**
```rust
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let query = cli.query.join(" ");

    if cli.grep {
        return search::global_search(&query);
    }

    if query == "~" || query == "..." {
        println!("{}", dirs::home_dir().unwrap().display());
        return Ok(());
    }

    if !query.is_empty() {
        let path = PathBuf::from(&query);
        if path.is_dir() {
            println!("{}", path.display());
            return Ok(());
        }
        if let Some(z_path) = zoxide::query(&query) {
            println!("{}", z_path.display());
            return Ok(());
        }
    }

    // TUI mode
    let initial_query = if query.is_empty() { None } else { Some(query) };
    let exit_path = tui::run(initial_query)?;
    if let Some(path) = exit_path {
        println!("{}", path.display());
    }
    Ok(())
}
```

### 4.2 `cli.rs` — Argument Parsing

```rust
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "cdx",
    about = "Interactive directory navigator",
    disable_help_flag = true   // Liberamos -h para "help" manual
)]
pub struct Cli {
    /// Global content search using ripgrep
    #[arg(short = 'g', long)]
    pub grep: bool,

    /// Show help
    #[arg(short = 'h', long = "help", action = clap::ArgAction::Help)]
    pub help: (),

    /// Search query or directory path
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub query: Vec<String>,
}
```

### 4.3 `config.rs` — Constantes y Exclusión

```rust
/// Directorios excluidos (equivalente a $script:ExcludeDirs)
pub const EXCLUDE_DIRS: &[&str] = &[
    "node_modules", ".git", ".cache", "cache", "licenses",
    "vendor", "target", "build", "dist", "Modules", "modules",
    "lib", "platform",
];

/// Directorios del sistema Windows (equivalente a $script:ExcludeWinDirs)
pub const EXCLUDE_WIN_DIRS: &[&str] = &[
    "AppData", "ProgramData",
];

/// Patrones de ruta completa (equivalente a $script:ExcludePathGlobs)
pub const EXCLUDE_PATH_GLOBS: &[&str] = &[
    "**/go/pkg/mod",
];

/// Directorios prioritarios para búsqueda global -g
pub const PRIORITY_ROOTS: &[&str] = &["dev", ".config"];

pub const MAX_PRIORITY_DEPTH: usize = 6;
pub const MAX_SECONDARY_DEPTH: usize = 5;
```

### 4.4 `walker/mod.rs` — Directory Walking

**Responsabilidad:** Listar directorios y archivos usando `ignore::WalkBuilder`, aplicando filtros de exclusión y toggles.

**API:**
```rust
/// Lista subdirectorios inmediatos de `root`.
/// Si es una raíz de unidad (C:\), usa WalkBuilder sin `--type d` (porque
/// en Windows los drive roots funcionan distinto).
pub fn list_dirs(
    root: &Path,
    show_dotfiles: bool,
    show_winhidden: bool,
) -> Vec<DirEntryItem>;

/// Lista archivos inmediatos de `root` (equivale a rg --files).
pub fn list_files(
    root: &Path,
    show_dotfiles: bool,
    show_winhidden: bool,
) -> Vec<DirEntryItem>;

/// Determina si un nombre de archivo/directorio debe excluirse
fn is_excluded(name: &str, is_winhidden: bool, show_winhidden: bool) -> bool;
```

**Implementación de `list_dirs`:**

```rust
pub fn list_dirs(root: &Path, show_dotfiles: bool, show_winhidden: bool) -> Vec<DirEntryItem> {
    let is_root = root.parent().is_none(); // Es C:\ o D:\ ?

    let mut builder = WalkBuilder::new(root);
    builder.max_depth(Some(1));              // Solo inmediatos
    builder.hidden(!show_dotfiles);          // Si show_dotfiles=true, incluir hidden
    builder.require_git(false);              // No requerir .git
    builder.filter_entry(move |entry| {
        // Filtro de exclusión por nombre
        let name = entry.file_name().to_string_lossy();
        if EXCLUDE_DIRS.contains(&name.as_ref()) { return false; }
        if !show_winhidden && EXCLUDE_WIN_DIRS.contains(&name.as_ref()) { return false; }
        if !show_dotfiles && name.starts_with('.') { return false; }
        if entry.file_type().map_or(false, |ft| ft.is_dir()) { return true; }
        false // Solo directorios
    });

    builder.build()
        .filter_map(|r| r.ok())
        .filter(|e| e.path() != root)
        .map(|e| DirEntryItem {
            display: e.file_name().to_string_lossy().to_string(),
            rel_path: e.path().strip_prefix(root).unwrap().to_string_lossy().to_string(),
            full_path: e.path().to_path_buf(),
            is_zoxide: false,
            is_dir: true,
        })
        .collect()
}
```

**Merge con Zoxide** (en `tui/app.rs` o `walker/mod.rs`):
- Zoxide cache se carga una vez en `App::new()` con `zoxide::get_list()`
- En cada cambio de directorio, se extraen los paths relativos del cache
- Los items de zoxide se marcan con ★ y se ponen primero
- Los items de walker se filtran para no duplicar zoxide

### 4.5 `zoxide/mod.rs` — Integración Zoxide

```rust
/// Consulta zoxide para un query (modo jump).
pub fn query(query_str: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("zoxide")
        .args(["query", query_str])
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() { return Some(PathBuf::from(path)); }
    }
    None
}

/// Obtiene lista completa de directorios frecuentados.
pub fn get_list() -> Vec<PathBuf> {
    std::process::Command::new("zoxide")
        .args(["query", "--list"])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}
```

### 4.6 `tui/events.rs` — Event Loop

**Responsabilidad:** Bucle principal de eventos crossterm. Lee teclas, actualiza `App`, dispara redibujado.

**Keybindings:**

| Tecla | Acción | Código |
|---|---|---|
| `Enter` | En Find: cd al dir seleccionado. En Search: abrir archivo con bat. En ambos: regenerar items. | `app.handle_enter()` |
| `Esc` | Subir al padre (`cd ..`). Si ya está en raíz → `should_quit = true`. | `app.handle_esc()` |
| `Ctrl+C` | Salir (quedarse en dir actual). | `app.should_quit = true` |
| `Ctrl+G` | Toggle Find ↔ Search. | `app.mode = !app.mode` |
| `Ctrl+A` | Toggle dotfiles. | `app.show_dotfiles = !app.show_dotfiles` |
| `Ctrl+W` | Toggle WinHidden. | `app.show_winhidden = !app.show_winhidden` |
| `Ctrl+H` | Ir a HOME. | `app.current_dir = dirs::home_dir()` |
| `Ctrl+O` | Yazi: salir con `ExitAction::SpawnYazi`. | `app.exit_action = ...` |
| `↑` / `↓` | Navegar lista. | `app.list_state.select_next/previous()` |
| `Tab` | Cambiar foco List ↔ Input. | `app.focus = !app.focus` |
| `Backspace` | Borrar último char del query. | `app.query.pop()` |
| `Esc` (en input) | Limpiar query. | `app.query.clear()` |
| Cualquier char | Añadir al query (si focus=Input). | `app.query.push(c)` |

**Estructura del event loop:**
```rust
pub fn run(initial_query: Option<String>) -> anyhow::Result<Option<PathBuf>> {
    let mut terminal = ratatui::init();  // Setup terminal + raw mode + panic hook
    let mut app = App::new(initial_query)?;

    // Carga inicial de items
    app.refresh_items();

    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                app.handle_key(key);
            }
        }
    }

    ratatui::restore();  // Restaura terminal

    // Si es Ctrl+O, spawn yazi
    if let ExitAction::SpawnYazi(path) = &app.exit_action {
        std::process::Command::new("yazi")
            .arg(path)
            .status()?;
        return Ok(Some(app.current_dir));
    }

    if let ExitAction::OutputPath(path) = &app.exit_action {
        return Ok(Some(path.clone()));
    }

    // JustExit: devolver current_dir
    Ok(Some(app.current_dir))
}
```

### 4.7 `tui/app.rs` — Lógica de Estado

**Métodos principales:**
```rust
impl App {
    pub fn new(initial_query: Option<String>) -> anyhow::Result<Self> { ... }

    /// Regenera items (directorios o archivos) desde el current_dir.
    pub fn refresh_items(&mut self) { ... }

    /// Aplica fuzzy filtering con nucleo sobre self.items → self.filtered_indices.
    pub fn apply_query(&mut self) { ... }

    /// Handle de Enter según modo actual.
    pub fn handle_enter(&mut self) { ... }

    /// Handle de Esc (subir al padre).
    pub fn handle_esc(&mut self) { ... }

    /// Handle de tecla genérica.
    pub fn handle_key(&mut self, key: KeyEvent) { ... }

    /// Merge de zoxide con items de walker.
    fn merge_zoxide(&mut self, walker_items: Vec<DirEntryItem>) -> Vec<DirEntryItem> { ... }

    /// Determina si la TUI debe renderizarse como popup overlay.
    /// Cae a full-screen si el terminal es menor a 80×24.
    fn should_use_popup(&self, term: Rect) -> bool {
        term.width >= 80 && term.height >= 24
    }
}
```

**`apply_query` — Fuzzy filtering con nucleo:**
```rust
pub fn apply_query(&mut self) {
    if self.query.is_empty() {
        self.filtered_indices = (0..self.items.len()).collect();
        return;
    }

    let pattern = Pattern::parse(
        &self.query,
        CaseMatching::Ignore,
        Normalization::Smart,
    );

    // Para path matching, activar match_paths
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());

    let mut scored: Vec<(usize, u16)> = self.items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            pattern.score(item.display.as_ref(), &mut matcher)
                .map(|s| (i, s))
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    self.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();

    // Resetear selección si los items cambiaron
    if !self.filtered_indices.is_empty() {
        self.list_state.select(Some(0));
    }
}
```

### 4.8 `tui/ui.rs` — Renderizado Popup Overlay con Ratatui

#### Estrategia

La TUI se renderiza como un **popup centrado** que ocupa ~80% del terminal, con el fondo de la terminal atenuado detrás. Si el terminal es muy pequeño (< 80×24), cae a **full-screen** sin atenuar.

**Representación visual:**

```
┌──────────────────────────────────────────────────────────┐
│  (fondo atenuado — se ve el prompt/historial detrás)      │
│                                                           │
│      ┌────────── cdx ───────────────────────────┐         │
│      │ Enter (cd) | Esc (..) | Ctrl+G toggle    │         │
│      ├────────────────┬─────────────────────────┤         │
│      │ ★ proyectos    │ === CONTENTS ===        │         │
│      │   src/         │ src/  Cargo.toml        │         │
│      │   docs/        │                          │         │
│      │ > target/      │ === GIT STATUS ===      │         │
│      │   .git/        │ Clean                    │         │
│      ├────────────────┴─────────────────────────┤         │
│      │ ~/dev/proyecto  | Find  | dotfiles: ✗   │         │
│      ├──────────────────────────────────────────┤         │
│      │ > proj                                    │         │
│      └──────────────────────────────────────────┘         │
│                                                           │
│  (fondo atenuado)                                         │
└──────────────────────────────────────────────────────────┘
```

#### Función principal `render`

```rust
pub fn render(frame: &mut Frame, app: &mut App) {
    let term = frame.area();

    // ── Calcular área del popup ──
    let (outer, main) = if app.should_use_popup(term) {
        let w = std::cmp::min(term.width * app.popup_width_pct / 100, 120);
        let h = std::cmp::min(term.height * app.popup_height_pct / 100, 40);
        let x = (term.width - w) / 2;
        let y = (term.height - h) / 2;
        let popup = Rect::new(x, y, w, h);

        // Fondo atenuado sobre toda la pantalla
        frame.render_widget(
            Block::default()
                .style(Style::default().bg(Color::DarkGray)),
            term,
        );

        // Limpiar el área del popup
        frame.render_widget(Clear, popup);

        // Borde del popup con título flotante
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" cdx ")
            .title_alignment(Alignment::Center);

        frame.render_widget(block.clone(), popup);
        let inner = block.inner(popup);

        layout_inner(inner)
    } else {
        // Full-screen fallback (terminal < 80×24)
        layout_inner(term)
    };

    // ── Renderizar widgets internos ──
    render_header(frame, outer[0], app);
    render_list(frame, main[0], app);
    render_preview(frame, main[1], app);
    render_status(frame, outer[2], app);
    render_input(frame, outer[3], app);
}

/// Divide el área interna en 4 filas: header, contenido, status, input.
/// Contenido se subdivide en lista (60%) y preview (40%).
fn layout_inner(area: Rect) -> ([Rect; 4], [Rect; 2]) {
    let rows = Layout::vertical([
        Constraint::Length(1),   // Header: "Enter (cd) | Esc (..) | Ctrl+G ..."
        Constraint::Fill(1),     // Main: Lista + Preview
        Constraint::Length(1),   // Status: "~/dev | Find | dotfiles: ✗"
        Constraint::Length(1),   // Query: "> proj"
    ])
    .split(area);

    let cols = Layout::horizontal([
        Constraint::Percentage(60),  // Lista
        Constraint::Percentage(40),  // Preview
    ])
    .split(rows[1]);

    (rows, cols)
}
```

#### `render_list` — Panel izquierdo con lista navegable

```rust
fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let items: Vec<ListItem> = app.filtered_indices
        .iter()
        .filter_map(|&i| app.items.get(i))
        .map(|item| {
            let style = if item.is_zoxide {
                Style::default().fg(Color::Yellow)  // ★ items amarillo
            } else {
                Style::default()
            };
            ListItem::new(item.display.as_str()).style(style)
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().reversed())
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}
```

#### `render_header` — Línea superior de atajos

```rust
fn render_header(frame: &mut Frame, area: Rect, app: &mut App) {
    let label = match app.mode {
        Mode::Find => "Enter (cd) | Esc (..) | Ctrl+G (Search) | Ctrl+O (yazi)",
        Mode::Search => "Enter (open) | Esc (..) | Ctrl+G (Find) | Ctrl+H (home)",
    };
    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(Color::Cyan)),
        area,
    );
}
```

#### `render_status` — Barra de estado

```rust
fn render_status(frame: &mut Frame, area: Rect, app: &mut App) {
    let path = display_path(&app.current_dir);
    let mode = match app.mode {
        Mode::Find => "Find",
        Mode::Search => "Search",
        Mode::Results => "Results",
    };
    let dot = if app.show_dotfiles { "✓" } else { "✗" };
    let win = if app.show_winhidden { "✓" } else { "✗" };

    let text = format!(" {} | {} | dotfiles: {} | WinHidden: {}", path, mode, dot, win);
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}
```

#### `render_input` — Campo de búsqueda fuzzy

```rust
fn render_input(frame: &mut Frame, area: Rect, app: &mut App) {
    let prefix = if app.focus == Focus::Input { "> " } else { "> " };
    let text = format!("{}{}", prefix, app.query);
    let style = match app.focus {
        Focus::Input => Style::default().fg(Color::Green),
        Focus::List => Style::default().fg(Color::White),
    };
    frame.render_widget(Paragraph::new(text).style(style), area);
}
```

#### `render_preview` — Panel derecho (contenido dinámico)

```rust
fn render_preview(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);

    // Si no hay selección, mostrar placeholder
    if app.preview_content.is_empty() {
        frame.render_widget(
            Paragraph::new(" (select an item) ")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            inner,
        );
        frame.render_widget(block, area);
        return;
    }

    let lines: Vec<Line> = app.preview_content
        .lines()
        .map(|line| {
            // Resaltar encabezados === SECTION ===
            if line.starts_with("===") {
                Line::from(Span::styled(line, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            } else {
                Line::from(Span::raw(line))
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .scroll((app.preview_scroll as u16, 0));

    frame.render_widget(block, area);
    frame.render_widget(paragraph, inner);
}
```

### 4.9 `preview/mod.rs` — Panel de Preview

**Responsabilidad:** Generar contenido para el panel derecho. Se llama cuando cambia la selección.

```rust
pub fn generate(app: &App, selected_item: &DirEntryItem) -> String {
    let full_path = app.current_dir.join(&selected_item.rel_path);

    if selected_item.is_dir {
        preview_directory(&full_path)
    } else {
        preview_file(&full_path)
    }
}

fn preview_directory(path: &Path) -> String {
    let mut output = String::new();

    // 1. Contenido con eza (o fallback manual)
    output.push_str("=== CONTENTS ===\n");
    if let Ok(out) = std::process::Command::new("eza")
        .args(["--icons", "--group-directories-first", "--color=always"])
        .arg(path)
        .output()
    {
        output.push_str(&String::from_utf8_lossy(&out.stdout));
    }

    // 2. Git status
    output.push_str("\n=== GIT STATUS ===\n");
    if let Ok(out) = std::process::Command::new("git")
        .args(["-C"])
        .arg(path)
        .args(["status", "--short"])
        .output()
    {
        let status = String::from_utf8_lossy(&out.stdout);
        if status.trim().is_empty() {
            output.push_str("Clean\n");
        } else {
            output.push_str(&status);
        }
    }

    output
}

fn preview_file(path: &Path) -> String {
    // bat --color=always --line-range :50 <path>
    if let Ok(out) = std::process::Command::new("bat")
        .args(["--color=always", "--line-range", ":50"])
        .arg(path)
        .output()
    {
        String::from_utf8_lossy(&out.stdout).to_string()
    } else {
        // Fallback: leer primeras 50 líneas
        std::fs::read_to_string(path)
            .map(|s| s.lines().take(50).collect::<Vec<_>>().join("\n"))
            .unwrap_or_default()
    }
}
```

**Debouncing:** Para no spammear subprocesos en cada cambio de selección, usar un contador:
```rust
// En el event loop:
if app.preview_dirty {
    if let Some(idx) = app.list_state.selected() {
        if let Some(item) = app.filtered_indices.get(idx).and_then(|&i| app.items.get(i)) {
            app.preview_content = preview::generate(&app, item);
        }
    }
    app.preview_dirty = false;
}
```

### 4.10 `search/mod.rs` — Búsqueda Global `-g`

**Responsabilidad:** Implementar la búsqueda en 3 fases (contenido → nombre archivo → nombre directorio), usando `rg` como subproceso.

```rust
pub fn global_search(query: &str) -> anyhow::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("HOME not set"))?;
    let priority_roots: Vec<PathBuf> = PRIORITY_ROOTS
        .iter()
        .map(|r| home.join(r))
        .filter(|p| p.exists())
        .collect();

    // Fase 1: Coincidencias de contenido
    let content_matches = search_content(query, &priority_roots, &home)?;

    // Fase 2: Coincidencias de nombre de archivo
    let filename_matches = search_filenames(query, &priority_roots, &home)?;

    // Fase 3: Coincidencias de nombre de directorio
    let dirname_matches = search_dirnames(query, &priority_roots, &home)?;

    // Combinar + deduplicar
    let all: Vec<String> = [content_matches, filename_matches, dirname_matches]
        .concat()
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if all.is_empty() {
        eprintln!("[i] No matches found for '{}'", query);
        return Ok(());
    }

    // Presentar con fzf
    present_results(&all, query)
}

fn search_content(query: &str, priority: &[PathBuf], home: &Path) -> anyhow::Result<Vec<String>> {
    let mut results = Vec::new();
    for root in priority {
        let output = std::process::Command::new("rg")
            .args(["--files-with-matches", "--smart-case", "--hidden"])
            .args(build_exclude_args())
            .args(["--max-depth", &MAX_PRIORITY_DEPTH.to_string()])
            .arg(query)
            .arg(root)
            .output()?;
        if output.status.success() {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                results.push(line.to_string());
            }
        }
    }
    // Secondary: HOME con profundidad 5
    let output = std::process::Command::new("rg")
        .args(["--files-with-matches", "--smart-case", "--hidden"])
        .args(build_exclude_args())
        .args(["--max-depth", &MAX_SECONDARY_DEPTH.to_string()])
        .arg(query)
        .arg(home)
        .output()?;
    // ...
    Ok(results)
}

fn build_exclude_args() -> Vec<String> {
    let mut args = Vec::new();
    for d in EXCLUDE_DIRS {
        args.push("--glob".into());
        args.push(format!("!{}", d));
    }
    for d in EXCLUDE_WIN_DIRS {
        args.push("--glob".into());
        args.push(format!("!{}", d));
    }
    for p in EXCLUDE_PATH_GLOBS {
        args.push("--glob".into());
        args.push(format!("!{}", p));
    }
    args
}
```

---

## 5. Shell Wrapper (PowerShell)

### 5.1 Función cdx + Show-CdxResult

Reemplaza las líneas 85-90 de `Microsoft.PowerShell_profile.ps1`:

```powershell
# cdx — CD Interactivo Unificado (Rust binary)
$cdxBin = "$HOME\.local\bin\cdx-rs.exe"
function cdx {
    if (-not (Test-Path $cdxBin)) {
        Write-Host "[!] cdx-rs.exe not found at $cdxBin" -ForegroundColor Red
        Write-Host "    Build: cd ~/dev/cdx-rs; cargo build --release" -ForegroundColor DarkGray
        return
    }
    $target = & $cdxBin @args 2>$null
    if ($LASTEXITCODE -ne 0) { return }
    if ($target -and (Test-Path $target)) {
        Set-Location $target
        Show-CdxResult
    }
}
function Show-CdxResult {
    $path = (Get-Location).Path
    $display = if ($path.StartsWith($env:USERPROFILE)) {
        "~" + $path.Substring($env:USERPROFILE.Length).Replace('\', '/')
    } else { $path.Replace('\', '/') }
    Write-Host "`n$display" -ForegroundColor Cyan
    if (Get-Command eza -ErrorAction SilentlyContinue) {
        eza --icons --group-directories-first
    } else { Get-ChildItem -Force | Format-Table }
    if (Get-Command git -ErrorAction SilentlyContinue) {
        $gitRoot = git rev-parse --show-toplevel 2>$null
        if ($gitRoot) {
            Write-Host "  Consider using: yazi, broot, nvim, lazygit, code ." -ForegroundColor DarkGray
        }
    }
}
```

### 5.2 PSReadLine Shortcut (Alt+C)

Añadir en el perfil tras la definición de `cdx`:

```powershell
# Alt+C → lanzar cdx TUI (como Ctrl+T de fzf pero para directorios)
# Si hay texto en el buffer antes del cursor, se pasa como query pre-llenado
Set-PSReadLineKeyHandler -Key Alt+C -ScriptBlock {
    $line = $null; $cursor = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
    $query = $line.Substring(0, $cursor).Trim()
    [Microsoft.PowerShell.PSConsoleReadLine]::BeginningOfLine()
    [Microsoft.PowerShell.PSConsoleReadLine]::KillLine()
    if ($query) { cdx $query } else { cdx }
    [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt()
}
```

**Por qué Alt+C:**
- No tiene binding en PSReadLine.
- No es señal de terminal (Ctrl+Q/XON podría tragarse la tecla en algunos terminales).
- Es semántico: fzf en bash usa Alt+C para `cd` — mismo concepto aquí.
- Libre de conflictos con los bindings existentes del perfil (Ctrl+T, Ctrl+G, Ctrl+R).

---

## 6. Fases de Implementación

| Fase | Módulos | Entregable | Est. horas |
|---|---|---|---|
| **P1: Skeleton** | `main.rs`, `cli.rs`, `config.rs`, `Cargo.toml` | Compila, acepta args, imprime path | 2h |
| **P2: Walker** | `walker/mod.rs`, `zoxide/mod.rs` | Lista dirs/archivos con filtros. Tests. | 4h |
| **P3: TUI Core** | `tui/app.rs`, `tui/events.rs`, `tui/ui.rs` | TUI funcional: lista, navegación flechas, Enter/Esc | 6h |
| **P4: TUI Interactivo** | Fuzzy input, toggles, header, status bar | Ctrl+G/A/W/H, filtrado fuzzy con nucleo | 4h |
| **P5: Preview** | `preview/mod.rs` | Panel derecho con eza/bat/git status | 3h |
| **P6: Search -g** | `search/mod.rs` | Búsqueda global 3 fases con rg | 3h |
| **P7: Polish** | Error handling, shell wrapper, chezmoi, docs | Binario listo para producción | 3h |
| **Total** | | | **~25h** |

### Orden de dependencias:
```
P1 ──► P2 ──► P3 ──► P4 ──► P5
                        │
                        └──► P6 (independiente de P4/P5)
                                  │
                                  └──► P7 (depende de todo)
```

---

## 7. Estrategia de Testing

```rust
// tests/walker_tests.rs

#[test]
fn test_list_dirs_basic() {
    let dirs = list_dirs(Path::new("./test_fixtures"), false, false);
    assert!(!dirs.is_empty());
    // Verificar que node_modules/.git están excluidos
    assert!(!dirs.iter().any(|d| d.display == "node_modules"));
}

#[test]
fn test_list_dirs_dotfiles_toggle() {
    let without = list_dirs(Path::new("./test_fixtures"), false, false);
    let with = list_dirs(Path::new("./test_fixtures"), true, false);
    assert!(with.len() >= without.len());
}

#[test]
fn test_exclude_win_dirs() {
    let without = list_dirs(Path::new("./test_fixtures"), true, false);
    let with = list_dirs(Path::new("./test_fixtures"), true, true);
    assert!(with.len() > without.len());
}

#[test]
fn test_zoxide_merge_dedup() {
    // Test que items de walker no duplican zoxide
}

#[test]
fn test_fuzzy_filtering() {
    // Test que nucleo filtra correctamente
}
```

---

## 8. Distribución & Chezmoi

```powershell
# Build release
cd ~/dev/cdx-rs
cargo build --release

# Copiar al PATH
cp target/release/cdx-rs.exe ~/.local/bin/

# Chezmoi: añadir el binario
chezmoi add ~/.local/bin/cdx-rs.exe
```

**Workflow de desarrollo:**
```powershell
# Iterar rápido:
cargo build --release && cp target/release/cdx-rs.exe ~/.local/bin/

# El perfil ya referencia ~/.local/bin/cdx-rs.exe
# Así que tras cada build, cdx usa el nuevo binario inmediatamente
```

---

## 9. Checklist de Feature Parity

| Feature | Cdx.ps1 | cdx-rs | Estado |
|---|---|---|---|
| TUI Browse (cdx sin args) | ✅ | ✅ P3 | |
| Jump directo (cdx \<ruta\>) | ✅ | ✅ P1 | |
| Jump zoxide (cdx \<nombre\>) | ✅ | ✅ P2 | |
| Búsqueda global (cdx -g) | ✅ | ✅ P6 | |
| Ayuda (cdx -h) | ✅ | ✅ P1 | |
| Shortcuts ~ / ... | ✅ | ✅ P1 | |
| Toggle Find↔Search (Ctrl+G) | ✅ | ✅ P4 | |
| Toggle dotfiles (Ctrl+A) | ✅ | ✅ P4 | |
| Toggle WinHidden (Ctrl+W) | ✅ | ✅ P4 | |
| Ir a HOME (Ctrl+H) | ✅ | ✅ P4 | |
| Abrir yazi (Ctrl+O) | ✅ | ✅ P4 | |
| Preview dir (eza + git) | ✅ | ✅ P5 | |
| Preview archivo (bat) | ✅ | ✅ P5 | |
| Zoxide merge (★) | ✅ | ✅ P2 | |
| ShowResult (ls post-cd) | ✅ | ✅ wrapper PS | |
| Exclude dirs | ✅ | ✅ P2 | |
| Exclude Win dirs | ✅ | ✅ P2 | |
| Exclude path globs | ✅ | ✅ P2 | |
| Search mode: abrir bat inline | ✅ | ✅ P4 | |
| Esc = cd .. | ✅ | ✅ P3 | |
| Ctrl+C = exit | ✅ | ✅ P3 | |
| Shortcut Alt+C (PSReadLine) | ➕ nuevo | ✅ P7 | |

---

## 10. Riesgos y Mitigaciones

| Riesgo | Mitigación |
|---|---|
| `ignore` crate en root drives (C:\) puede ser lento | `WalkBuilder::max_depth(1)` — solo lista inmediatos, igual que el código actual |
| Subprocess a eza/bat/git puede fallar si no están instalados | Fallbacks integrados: `ignore` para dir listing, `fs::read_to_string` para archivos |
| nucleo fuzzy matching puede diferir del comportamiento de fzf | Mismo algoritmo (Smith-Waterman). Testear con queries complejos. Si difiere, `Pattern::parse` permite ajustar comportamiento. |
| La preview regenerada en cada selección puede ser lenta | Debouncing: solo regenerar si la selección no ha cambiado en 50ms |
| Ctrl+O (yazi) necesita manejar el terminal correctamente | `ratatui::restore()` limpia el terminal antes de spawn yazi. Al volver, el wrapper PS maneja el cd. |
| Popup overlay en terminal < 80×24 se ve muy pequeño | `should_use_popup()` verifica el tamaño: si < 80×24, cae a full-screen sin atenuado. Mínimo fijo de 60 columnas si el usuario fuerza popup. |
