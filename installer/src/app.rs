//! Application state and the terminal chrome shared by every screen.
//!
//! Every screen lives in `screens/` and is dispatched from here. The chrome is
//! always the same: a one-line header, a text progress bar, one centered panel
//! (fixed 52–60 columns) with its title embedded in the top border, a button
//! row at the bottom of the panel, and an optional commands column toggled
//! globally with `v`.

use crate::detect::{self, Disk};
use crate::disk::{self, NamedPartition};
use crate::install::{self, Msg as InstallMsg, Plan, Step};
use crate::settings::Settings;
use crate::theme::{self, Palette};
use color_eyre::eyre::Result;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

const PANEL_WIDTH: u16 = 58;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    License,
    System,
    Password,
    Disk,
    Summary,
    Installing,
    Done,
}

impl Screen {
    pub const ORDER: [Screen; 8] = [
        Screen::Welcome,
        Screen::License,
        Screen::System,
        Screen::Password,
        Screen::Disk,
        Screen::Summary,
        Screen::Installing,
        Screen::Done,
    ];

    pub fn index(self) -> usize {
        Self::ORDER.iter().position(|&s| s == self).unwrap_or(0)
    }

    pub fn title(self) -> &'static str {
        match self {
            Screen::Welcome => "Bienvenido",
            Screen::License => "Licencia",
            Screen::System => "Tu sistema",
            Screen::Password => "Contraseña",
            Screen::Disk => "Disco",
            Screen::Summary => "Resumen",
            Screen::Installing => "Instalando",
            Screen::Done => "Listo",
        }
    }

    fn previous(self) -> Option<Screen> {
        match self {
            Screen::Welcome => None,
            Screen::License => Some(Screen::Welcome),
            Screen::System => Some(Screen::License),
            Screen::Password => Some(Screen::System),
            Screen::Disk => Some(Screen::Password),
            Screen::Summary => Some(Screen::Disk),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Plain,
    Username,
    TimeZone,
    Email,
}

pub struct FormField {
    pub label: &'static str,
    pub kind: FieldKind,
    pub input: Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quit {
    Normal,
    Installing,
}

pub struct DoneInfo {
    pub host: String,
    pub duration: Duration,
    pub ok: bool,
}

/// Live state of the install step.
pub struct InstallView {
    pub started: Instant,
    pub current: Option<Step>,
    pub done: Vec<Step>,
    pub log: VecDeque<String>,
    pub note: Option<String>,
    pub built: u64,
    pub total: u64,
    pub finished: bool,
    pub error: Option<String>,
    pub log_path: Option<PathBuf>,
}

impl InstallView {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            current: None,
            done: Vec::new(),
            log: VecDeque::new(),
            note: None,
            built: 0,
            total: 0,
            finished: false,
            error: None,
            log_path: None,
        }
    }

    pub fn push_log(&mut self, line: String) {
        self.log.push_back(line);
        while self.log.len() > 200 {
            self.log.pop_front();
        }
    }

    pub fn visible_log(&self, lines: usize) -> Vec<&String> {
        let skip = self.log.len().saturating_sub(lines);
        self.log.iter().skip(skip).collect()
    }
}

pub struct App {
    pub palette: Palette,
    pub screen: Screen,

    pub dry_run: bool,
    pub repo_spec: String,
    pub repo_dir: PathBuf,
    pub source_ok: bool,

    pub settings: Settings,
    pub settings_ok: bool,

    // welcome detections
    pub arch: Option<String>,
    pub virt: Option<String>,
    pub disks_found: bool,
    pub network: bool,

    // license
    pub license_accepted: bool,

    // system form
    pub fields: Vec<FormField>,
    pub field_focus: usize,

    // password
    pub password: Input,
    pub password2: Input,
    pub password_hash: Option<String>,

    // disk
    pub disks: Vec<Disk>,
    pub disk_index: usize,
    pub layout: Vec<NamedPartition>,
    pub disk_confirmed: bool,

    // summary
    pub keep_uutils: bool,

    // button focus within the current screen (0 = back, 1 = primary)
    pub button_focus: usize,

    pub show_commands: bool,

    pub install: InstallView,
    install_rx: Option<Receiver<InstallMsg>>,
    pub done: Option<DoneInfo>,

    pub status: Option<String>,
    pub quit: Option<Quit>,
    pub should_quit: bool,
    pub tick: u64,
    /// Set by the license screen to open `less` over the TUI.
    pub pending_less: Option<PathBuf>,
}

impl App {
    pub fn new(dry_run: bool, repo_spec: String, repo_dir: PathBuf) -> Self {
        let palette = if theme::console::is_linux_console() {
            Palette::console()
        } else {
            Palette::truecolor()
        };

        let (mut settings, settings_ok) = match Settings::read(&repo_dir) {
            Ok(settings) => (settings, true),
            Err(_) => (Settings::fallback(), false),
        };
        // `system` is detected, never taken from the file.
        if let Some(system) = detect::nix_system() {
            settings.system = system;
        }

        let disks = detect::disks();
        let fields = build_fields(&settings);
        let arch = detect::arch();
        let virt = detect::environment();

        let mut app = Self {
            palette,
            screen: Screen::Welcome,
            dry_run,
            repo_spec,
            repo_dir,
            source_ok: false,
            settings,
            settings_ok,
            arch,
            virt,
            disks_found: !disks.is_empty(),
            network: false,
            license_accepted: false,
            fields,
            field_focus: 0,
            password: Input::default(),
            password2: Input::default(),
            password_hash: None,
            disks,
            disk_index: 0,
            layout: Vec::new(),
            disk_confirmed: false,
            keep_uutils: false,
            button_focus: 1,
            show_commands: false,
            install: InstallView::new(),
            install_rx: None,
            done: None,
            status: None,
            quit: None,
            should_quit: false,
            tick: 0,
            pending_less: None,
        };
        app.load_layout();
        app
    }

    /// Moves the disk selection and reloads the disko layout.
    pub fn cycle_disk(&mut self, delta: isize) {
        if self.disks.is_empty() {
            return;
        }
        let len = self.disks.len() as isize;
        self.disk_index = ((self.disk_index as isize + delta).rem_euclid(len)) as usize;
        self.load_layout();
        self.disk_confirmed = false;
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            if ratatui::crossterm::event::poll(Duration::from_millis(100))? {
                let event = ratatui::crossterm::event::read()?;
                self.handle_event(event);
            }

            if let Some(path) = self.pending_less.take() {
                self.suspend_for_less(terminal, &path)?;
            }

            self.tick = self.tick.wrapping_add(1);
            self.poll_install();

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    /// Leaves the alternate screen, runs `less <path>`, then restores the TUI.
    fn suspend_for_less(
        &self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
        path: &std::path::Path,
    ) -> Result<()> {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        let _ = std::process::Command::new("less").arg(path).status();
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        enable_raw_mode()?;
        terminal.clear()?;
        Ok(())
    }

    // -- data ----------------------------------------------------------------

    pub fn detected_system(&self) -> String {
        self.settings.system.clone()
    }

    fn load_layout(&mut self) {
        if let Some(disk) = self.disks.get(self.disk_index) {
            let device = disk.path();
            match disk::layout(&self.repo_dir, &device) {
                Ok(layout) => self.layout = layout,
                Err(_) => self.layout = disk::fallback_layout(),
            }
        } else {
            self.layout = disk::fallback_layout();
        }
    }

    pub fn apply_settings_from_fields(&mut self) {
        let values: Vec<(&'static str, String)> = self
            .fields
            .iter()
            .map(|f| (f.label, f.input.value().to_string()))
            .collect();
        for (label, value) in values {
            match label {
                "hostname" => self.settings.host_name = value,
                "zona horaria" => self.settings.time_zone = value,
                "idioma" => self.settings.default_locale = value,
                "usuario" => self.settings.username = value,
                "nombre" => self.settings.userdescription = value,
                "git nombre" => self.settings.git_name = value,
                "git email" => self.settings.git_email = value,
                _ => {}
            }
        }
    }

    /// Whether the field with the given label currently holds a valid value.
    pub fn field_valid(&self, label: &str, kind: FieldKind) -> bool {
        match kind {
            FieldKind::Username => self.username_valid(),
            FieldKind::TimeZone => self.validate_timezone(),
            FieldKind::Email => {
                let value = self
                    .fields
                    .iter()
                    .find(|f| f.label == label)
                    .map(|f| f.input.value())
                    .unwrap_or("");
                value.contains('@')
            }
            FieldKind::Plain => true,
        }
    }

    pub fn validate_timezone(&self) -> bool {
        let tz = self.settings.time_zone.trim();
        if tz.is_empty() {
            return false;
        }
        let zoneinfo = std::path::Path::new("/etc/zoneinfo").join(tz);
        let zoneinfo_alt = std::path::Path::new("/usr/share/zoneinfo").join(tz);
        if zoneinfo.exists() || zoneinfo_alt.exists() {
            return true;
        }
        // Fall back to timedatectl when zoneinfo layout is unusual.
        if let Some(list) = detect::run_trim("timedatectl", &["list-timezones"]) {
            if list.lines().any(|l| l.trim() == tz) {
                return true;
            }
        }
        false
    }

    pub fn username_valid(&self) -> bool {
        let name = self.settings.username.as_str();
        let mut chars = name.chars();
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() || c == '_' => {}
            _ => return false,
        }
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    }

    pub fn passwords_match(&self) -> bool {
        !self.password.value().is_empty() && self.password.value() == self.password2.value()
    }

    pub fn selected_disk(&self) -> Option<&Disk> {
        self.disks.get(self.disk_index)
    }

    // -- install -------------------------------------------------------------

    pub fn start_install(&mut self) {
        let Some(disk) = self.selected_disk().cloned() else {
            self.status = Some("no hay disco seleccionado".into());
            return;
        };
        if self.password_hash.is_none() {
            let password = self.password.value().to_string();
            if self.dry_run {
                self.password_hash = Some("$y$j9T$SIMULATED".into());
            } else {
                match install::hash_password(&password) {
                    Ok(hash) => self.password_hash = Some(hash),
                    Err(err) => {
                        self.status = Some(format!("mkpasswd: {err}"));
                        return;
                    }
                }
            }
        }

        let (tx, rx) = mpsc::channel();
        self.install_rx = Some(rx);
        self.install = InstallView::new();

        let root = if self.dry_run {
            let dir = std::env::temp_dir().join("nixdots-mnt");
            let _ = std::fs::create_dir_all(&dir);
            dir
        } else {
            PathBuf::from("/mnt")
        };

        let plan = Plan {
            source: self.repo_dir.clone(),
            root,
            settings: self.settings.clone(),
            password_hash: self.password_hash.clone().unwrap_or_default(),
            device: disk.path(),
            keep_uutils: self.keep_uutils,
            dry_run: self.dry_run,
        };
        install::spawn(plan, tx);
    }

    fn poll_install(&mut self) {
        let Some(rx) = self.install_rx.take() else {
            return;
        };
        let mut finished: Option<(bool, Option<String>)> = None;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                InstallMsg::Step(step) => {
                    if let Some(prev) = self.install.current.replace(step) {
                        if !self.install.done.contains(&prev) {
                            self.install.done.push(prev);
                        }
                    }
                }
                InstallMsg::Note(note) => self.install.note = Some(note),
                InstallMsg::Log(line) => self.install.push_log(line),
                InstallMsg::Progress { built, total } => {
                    self.install.built = built;
                    self.install.total = total;
                }
                InstallMsg::Finished { ok, error, log_path } => {
                    if let Some(step) = self.install.current.take() {
                        self.install.done.push(step);
                    }
                    self.install.log_path = Some(log_path);
                    finished = Some((ok, error));
                }
            }
        }
        if let Some((ok, error)) = finished {
            self.install.finished = true;
            self.install.error = error;
            if ok {
                self.done = Some(DoneInfo {
                    host: self.settings.host_name.clone(),
                    duration: self.install.started.elapsed(),
                    ok: true,
                });
                self.screen = Screen::Done;
                self.button_focus = 1;
            }
        } else {
            self.install_rx = Some(rx);
        }
    }

    // -- events --------------------------------------------------------------

    pub fn handle_event(&mut self, event: Event) {
        if self.quit.is_some() {
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                    self.handle_quit(key);
                }
            }
            return;
        }

        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
                return;
            }
            // Text fields consume their own keys first.
            if self.screen_has_text_focus() && self.route_text(key) {
                return;
            }
            self.route(key);
        }
    }

    fn screen_has_text_focus(&self) -> bool {
        matches!(self.screen, Screen::System | Screen::Password)
    }

    fn route_text(&mut self, key: KeyEvent) -> bool {
        match self.screen {
            Screen::System => {
                // Up/down are navigation, everything else edits the field.
                if matches!(key.code, KeyCode::Up | KeyCode::Down | KeyCode::Tab | KeyCode::Enter | KeyCode::Esc) {
                    return false;
                }
                let field = &mut self.fields[self.field_focus];
                field.input.handle_event(&Event::Key(key));
                self.on_system_edited();
                true
            }
            Screen::Password => {
                if matches!(key.code, KeyCode::Up | KeyCode::Down | KeyCode::Tab | KeyCode::Enter | KeyCode::Esc) {
                    return false;
                }
                if self.field_focus == 0 {
                    self.password.handle_event(&Event::Key(key));
                } else {
                    self.password2.handle_event(&Event::Key(key));
                }
                true
            }
            _ => false,
        }
    }

    fn on_system_edited(&mut self) {
        self.apply_settings_from_fields();
        self.load_layout();
    }

    fn route(&mut self, key: KeyEvent) {
        // Global keys first.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = Some(if self.screen == Screen::Installing {
                Quit::Installing
            } else {
                Quit::Normal
            });
            return;
        }

        match key.code {
            KeyCode::Char('v') => {
                self.show_commands = !self.show_commands;
                return;
            }
            KeyCode::Char('q') if self.screen != Screen::Installing => {
                self.quit = Some(Quit::Normal);
                return;
            }
            KeyCode::Esc => {
                if self.screen == Screen::Installing {
                    self.quit = Some(Quit::Installing);
                } else {
                    self.go_back();
                }
                return;
            }
            _ => {}
        }

        self.route_screen(key);
    }

    fn route_screen(&mut self, key: KeyEvent) {
        match self.screen {
            Screen::Welcome => crate::screens::welcome::key(self, key),
            Screen::License => crate::screens::license::key(self, key),
            Screen::System => crate::screens::system::key(self, key),
            Screen::Password => crate::screens::password::key(self, key),
            Screen::Disk => crate::screens::disk::key(self, key),
            Screen::Summary => crate::screens::summary::key(self, key),
            Screen::Installing => crate::screens::installing::key(self, key),
            Screen::Done => crate::screens::done::key(self, key),
        }
    }

    fn handle_quit(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('s') | KeyCode::Enter => {
                self.should_quit = true;
            }
            KeyCode::Char('n') | KeyCode::Esc => self.quit = None,
            _ => {}
        }
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.screen.previous() {
            self.screen = prev;
            self.button_focus = 1;
            self.status = None;
        }
    }

    pub fn advance(&mut self) {
        let next = match self.screen {
            Screen::Welcome => Some(Screen::License),
            Screen::License => Some(Screen::System),
            Screen::System => Some(Screen::Password),
            Screen::Password => Some(Screen::Disk),
            Screen::Disk => Some(Screen::Summary),
            Screen::Summary => {
                self.screen = Screen::Installing;
                self.button_focus = 1;
                self.show_commands = true;
                self.start_install();
                return;
            }
            _ => None,
        };
        if let Some(next) = next {
            self.screen = next;
            self.button_focus = 1;
            self.status = None;
        }
    }

    pub fn primary_enabled(&self) -> bool {
        match self.screen {
            Screen::License => self.license_accepted,
            Screen::System => self.username_valid() && self.validate_timezone(),
            Screen::Password => self.passwords_match(),
            Screen::Disk => self.disk_confirmed && self.selected_disk().is_some(),
            _ => true,
        }
    }

    // -- rendering -----------------------------------------------------------

    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        f.render_widget(
            Block::default().style(Style::default().bg(self.palette.reset)),
            area,
        );

        let content_h = self.content_height();
        let commands_lines = self.commands_lines();
        let commands_h = if self.show_commands {
            (commands_lines.len() as u16 + 2).clamp(2, area.height / 2)
        } else {
            0
        };
        let panel_h = content_h + 4;
        let width = PANEL_WIDTH.min(area.width);
        let gap: u16 = 1;
        let mut total = 1 + 1 + gap + panel_h + gap + 1;
        if commands_h > 0 {
            total += gap + commands_h;
        }

        let top = area.y + area.height.saturating_sub(total) / 2;
        let x = area.x + area.width.saturating_sub(width) / 2;

        let header = Rect::new(x, top, width, 1);
        let progress = Rect::new(x, top + 2, width, 1);
        let panel = Rect::new(x, top + 3 + gap, width, panel_h);
        let shortcuts = Rect::new(x, panel.bottom() + gap, width, 1);
        let commands = Rect::new(
            x,
            shortcuts.bottom() + gap,
            width,
            commands_h,
        );

        self.draw_header(f, header);
        self.draw_progress(f, progress);
        self.draw_panel(f, panel);
        self.draw_shortcuts(f, shortcuts);
        if self.show_commands {
            self.draw_commands(f, commands, &commands_lines, commands_h);
        }

        if self.quit.is_some() {
            self.draw_quit(f, area);
        }
    }

    fn content_height(&self) -> u16 {
        match self.screen {
            Screen::Welcome => crate::screens::welcome::height(self),
            Screen::License => crate::screens::license::height(self),
            Screen::System => crate::screens::system::height(self),
            Screen::Password => crate::screens::password::height(self),
            Screen::Disk => crate::screens::disk::height(self),
            Screen::Summary => crate::screens::summary::height(self),
            Screen::Installing => crate::screens::installing::height(self),
            Screen::Done => crate::screens::done::height(self),
        }
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let mut spans = vec![
            Span::styled(
                "❄ nixdots",
                Style::default()
                    .fg(self.palette.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" instalador", Style::default().fg(self.palette.muted)),
        ];
        if self.dry_run {
            spans.push(Span::styled(
                "  · simulación",
                Style::default().fg(self.palette.warning),
            ));
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)).alignment(ratatui::layout::Alignment::Center),
            area,
        );
    }

    fn draw_progress(&self, f: &mut Frame, area: Rect) {
        let total = Screen::ORDER.len();
        let current = self.screen.index() + 1;
        let label = format!("  {current}/{total}");
        let bar_width = (area.width as usize).saturating_sub(label.len() + 2).max(1);
        let filled = (bar_width * current) / total;
        let line = Line::from(vec![
            Span::styled(
                "━".repeat(filled),
                Style::default().fg(self.palette.primary),
            ),
            Span::styled(
                "─".repeat(bar_width - filled),
                Style::default().fg(self.palette.muted),
            ),
            Span::styled(label, Style::default().fg(self.palette.muted)),
        ])
        .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(Paragraph::new(line), area);
    }

    fn draw_panel(&self, f: &mut Frame, area: Rect) {
        let border = match self.screen {
            Screen::Done => self.palette.success,
            Screen::Installing if self.install.error.is_some() => self.palette.error,
            _ => self.palette.primary,
        };
        let title = format!(" {} ", self.screen.title());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border))
            .title(Span::styled(title, Style::default().fg(border)));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let content_h = self.content_height();
        let content = Rect::new(
            inner.x + 2,
            inner.y,
            inner.width.saturating_sub(4),
            content_h.min(inner.height.saturating_sub(2)),
        );

        match self.screen {
            Screen::Welcome => crate::screens::welcome::draw(f, self, content),
            Screen::License => crate::screens::license::draw(f, self, content),
            Screen::System => crate::screens::system::draw(f, self, content),
            Screen::Password => crate::screens::password::draw(f, self, content),
            Screen::Disk => crate::screens::disk::draw(f, self, content),
            Screen::Summary => crate::screens::summary::draw(f, self, content),
            Screen::Installing => crate::screens::installing::draw(f, self, content),
            Screen::Done => crate::screens::done::draw(f, self, content),
        }

        self.draw_buttons(f, inner);
    }

    fn buttons(&self) -> (Option<&'static str>, Option<&'static str>) {
        // (back, primary)
        match self.screen {
            Screen::Welcome => (Some("Salir"), Some("Continuar")),
            Screen::License => (Some("Atrás"), Some("Continuar")),
            Screen::System => (Some("Atrás"), Some("Continuar")),
            Screen::Password => (Some("Atrás"), Some("Continuar")),
            Screen::Disk => (Some("Atrás"), Some("Continuar")),
            Screen::Summary => (Some("Atrás"), Some("Instalar")),
            Screen::Installing => (None, None),
            Screen::Done => (Some("Salir"), Some("Reiniciar")),
        }
    }

    fn draw_buttons(&self, f: &mut Frame, area: Rect) {
        let (back, primary) = self.buttons();
        let row = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
        let mut spans: Vec<Span> = Vec::new();
        if let Some(back) = back {
            let back_focused = self.button_focus == 0;
            let color = if back_focused {
                self.palette.highlight
            } else {
                self.palette.muted
            };
            spans.push(Span::styled(
                format!("‹ {back}"),
                Style::default()
                    .fg(color)
                    .add_modifier(if back_focused {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ));
            spans.push(Span::raw("    "));
        }
        if let Some(primary) = primary {
            let enabled = self.primary_enabled();
            let style = if enabled {
                Style::default()
                    .fg(self.palette.on_primary)
                    .bg(self.palette.primary)
            } else {
                Style::default().fg(self.palette.muted)
            };
            spans.push(Span::styled(format!(" {primary} › "), style));
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)).alignment(ratatui::layout::Alignment::Center),
            row,
        );
    }

    fn draw_shortcuts(&self, f: &mut Frame, area: Rect) {
        let toggle = format!(
            "v comandos: {}",
            if self.show_commands { "on" } else { "off" }
        );

        let (line, color) = if let Some(status) = &self.status {
            (status.clone(), self.palette.warning)
        } else {
            match self.screen {
                Screen::Summary => (
                    format!("no se puede deshacer · {toggle}"),
                    self.palette.warning,
                ),
                Screen::Done => (
                    format!("retira la ISO antes de reiniciar · {toggle}"),
                    self.palette.muted,
                ),
                _ => {
                    let mut parts: Vec<&str> = match self.screen {
                        Screen::Welcome => vec!["enter continuar", "esc salir"],
                        Screen::License => vec!["espacio aceptar", "l ver licencia"],
                        Screen::System | Screen::Password => vec!["↑↓ campo"],
                        Screen::Disk => vec!["↑↓ disco", "espacio borrar"],
                        Screen::Installing => vec!["ctrl+c cancelar"],
                        _ => vec![],
                    };
                    parts.push("tab botones");
                    (format!("{} · {toggle}", parts.join(" · ")), self.palette.muted)
                }
            }
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(line, Style::default().fg(color))))
                .alignment(ratatui::layout::Alignment::Center),
            area,
        );
    }

    fn draw_commands(&self, f: &mut Frame, area: Rect, lines: &[String], height: u16) {
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(self.palette.border));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let visible: Vec<Line> = if self.screen == Screen::Installing {
            self.install
                .visible_log(height as usize - 1)
                .into_iter()
                .map(|l| self.log_line(l))
                .collect()
        } else {
            lines
                .iter()
                .map(|l| {
                    Line::from(Span::styled(
                        format!("$ {l}"),
                        Style::default().fg(self.palette.muted),
                    ))
                })
                .collect()
        };
        f.render_widget(
            Paragraph::new(visible).wrap(Wrap { trim: true }),
            inner,
        );
    }

    fn log_line(&self, line: &str) -> Line<'static> {
        let color = if line.starts_with('$') {
            self.palette.primary
        } else if line.starts_with("error") || line.contains("error:") {
            self.palette.error
        } else {
            self.palette.text
        };
        Line::from(Span::styled(line.to_string(), Style::default().fg(color)))
    }

    fn draw_quit(&self, f: &mut Frame, area: Rect) {
        let message = match self.quit {
            Some(Quit::Installing) => "¿Cancelar la instalación?",
            _ => "¿Salir del instalador?",
        };
        let width = 42.min(area.width);
        let height = 5;
        let x = area.x + area.width.saturating_sub(width) / 2;
        let y = area.y + area.height.saturating_sub(height) / 2;
        let rect = Rect::new(x, y, width, height);
        f.render_widget(Clear, rect);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.palette.warning));
        let inner = block.inner(rect);
        f.render_widget(block, rect);
        let text = vec![
            Line::from(Span::styled(
                message,
                Style::default().fg(self.palette.text),
            ))
            .alignment(ratatui::layout::Alignment::Center),
            Line::from(""),
            Line::from(Span::styled(
                "s / enter · n / esc",
                Style::default().fg(self.palette.muted),
            ))
            .alignment(ratatui::layout::Alignment::Center),
        ];
        f.render_widget(Paragraph::new(text), inner);
    }

    // -- shared small widgets ------------------------------------------------

    pub fn checkbox(&self, checked: bool, label: &str, focused: bool) -> Line<'static> {
        let mark = if checked { "[x]" } else { "[ ]" };
        let color = if focused {
            self.palette.highlight
        } else {
            self.palette.text
        };
        Line::from(vec![
            Span::styled(
                format!("{mark} "),
                Style::default().fg(if checked {
                    self.palette.success
                } else {
                    self.palette.muted
                }),
            ),
            Span::styled(label.to_string(), Style::default().fg(color)),
        ])
    }

    pub fn radio(&self, selected: bool, label: &str) -> Line<'static> {
        let mark = if selected { "(•)" } else { "( )" };
        Line::from(Span::styled(
            format!("{mark} {label}"),
            Style::default().fg(if selected {
                self.palette.highlight
            } else {
                self.palette.muted
            }),
        ))
    }

    /// Renders an editable field with a block cursor, horizontal scrolling and
    /// an optional mask for passwords.
    pub fn field_line(
        &self,
        label: &str,
        value: &str,
        cursor: usize,
        focused: bool,
        masked: bool,
        width: u16,
    ) -> Line<'static> {
        let marker = if focused { "› " } else { "  " };
        let label_style = if focused {
            Style::default()
                .fg(self.palette.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.palette.muted)
        };

        let chars: Vec<char> = if masked {
            vec!['•'; value.chars().count()]
        } else {
            value.chars().collect()
        };
        let cursor = cursor.min(chars.len());
        let available = width.saturating_sub(label.len() as u16 + 4).max(1) as usize;
        let start = if chars.len() > available {
            cursor.saturating_sub(available.saturating_sub(1))
        } else {
            0
        };
        let end = (start + available).min(chars.len());
        let window: Vec<char> = chars[start..end].to_vec();
        let cursor_in_window = cursor.saturating_sub(start);

        let before: String = window[..cursor_in_window.min(window.len())].iter().collect();
        let at: char = window.get(cursor_in_window).copied().unwrap_or(' ');
        let after: String = window
            .get(cursor_in_window + 1..)
            .unwrap_or(&[])
            .iter()
            .collect();

        let mut spans = vec![
            Span::styled(
                marker,
                Style::default().fg(if focused {
                    self.palette.primary
                } else {
                    self.palette.muted
                }),
            ),
            Span::styled(format!("{label}: "), label_style),
            Span::styled(before, Style::default().fg(self.palette.text)),
        ];
        if focused {
            spans.push(Span::styled(
                at.to_string(),
                Style::default()
                    .fg(self.palette.on_primary)
                    .bg(self.palette.primary),
            ));
        } else {
            spans.push(Span::styled(
                at.to_string(),
                Style::default().fg(self.palette.text),
            ));
        }
        spans.push(Span::styled(after, Style::default().fg(self.palette.text)));
        Line::from(spans)
    }

    pub fn command_list(&self) -> Vec<String> {
        match self.screen {
            Screen::Welcome => crate::screens::welcome::commands(self),
            Screen::License => crate::screens::license::commands(self),
            Screen::System => crate::screens::system::commands(self),
            Screen::Password => crate::screens::password::commands(self),
            Screen::Disk => crate::screens::disk::commands(self),
            Screen::Summary => crate::screens::summary::commands(self),
            Screen::Installing => crate::screens::installing::commands(self),
            Screen::Done => crate::screens::done::commands(self),
        }
    }

    fn commands_lines(&self) -> Vec<String> {
        self.command_list()
    }
}

fn build_fields(settings: &Settings) -> Vec<FormField> {
    vec![
        FormField {
            label: "hostname",
            kind: FieldKind::Plain,
            input: Input::new(settings.host_name.clone()),
        },
        FormField {
            label: "zona horaria",
            kind: FieldKind::TimeZone,
            input: Input::new(settings.time_zone.clone()),
        },
        FormField {
            label: "idioma",
            kind: FieldKind::Plain,
            input: Input::new(settings.default_locale.clone()),
        },
        FormField {
            label: "usuario",
            kind: FieldKind::Username,
            input: Input::new(settings.username.clone()),
        },
        FormField {
            label: "nombre",
            kind: FieldKind::Plain,
            input: Input::new(settings.userdescription.clone()),
        },
        FormField {
            label: "git nombre",
            kind: FieldKind::Plain,
            input: Input::new(settings.git_name.clone()),
        },
        FormField {
            label: "git email",
            kind: FieldKind::Email,
            input: Input::new(settings.git_email.clone()),
        },
    ]
}
