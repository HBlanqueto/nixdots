//! Application state and the terminal chrome shared by every screen.
//!
//! Chrome is always: one header line, a 36-cell text progress bar (3 cells per
//! step, `n/12`), one centered rounded panel with its title embedded in the top
//! border, a button row, and an optional `v` commands column.

use crate::detect::{self, Disk};
use crate::disk::{self, NamedPartition};
use crate::install::{self, Msg as InstallMsg, Plan, Step};
use crate::license_view::LicenseViewer;
use crate::net;
use crate::settings::Settings;
use crate::theme::{self, Palette};
use crate::widgets::{self, Btn};
use color_eyre::eyre::Result;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};
use tui_input::Input;

const PANEL_WIDTH: u16 = 58;
const STEPS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    Network,
    License,
    Host,
    Timezone,
    Locale,
    User,
    Password,
    Git,
    Disk,
    Summary,
    Installing,
    Done,
}

impl Screen {
    pub const ORDER: [Screen; 13] = [
        Screen::Welcome,
        Screen::Network,
        Screen::License,
        Screen::Host,
        Screen::Timezone,
        Screen::Locale,
        Screen::User,
        Screen::Password,
        Screen::Git,
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
            Screen::Network => "Red",
            Screen::License => "Licencia",
            Screen::Host => "Equipo",
            Screen::Timezone => "Zona horaria",
            Screen::Locale => "Idioma",
            Screen::User => "Usuario",
            Screen::Password => "Contraseña",
            Screen::Git => "Git",
            Screen::Disk => "Disco",
            Screen::Summary => "Resumen",
            Screen::Installing => "Instalando",
            Screen::Done => "Listo",
        }
    }

    fn next(self) -> Option<Screen> {
        match self {
            Screen::Welcome => Some(Screen::Network),
            Screen::Network => Some(Screen::License),
            Screen::License => Some(Screen::Host),
            Screen::Host => Some(Screen::Timezone),
            Screen::Timezone => Some(Screen::Locale),
            Screen::Locale => Some(Screen::User),
            Screen::User => Some(Screen::Password),
            Screen::Password => Some(Screen::Git),
            Screen::Git => Some(Screen::Disk),
            Screen::Disk => Some(Screen::Summary),
            Screen::Summary => Some(Screen::Installing),
            _ => None,
        }
    }

    fn previous(self) -> Option<Screen> {
        match self {
            Screen::Welcome | Screen::Installing | Screen::Done => None,
            Screen::Network => Some(Screen::Welcome),
            Screen::License => Some(Screen::Network),
            Screen::Host => Some(Screen::License),
            Screen::Timezone => Some(Screen::Host),
            Screen::Locale => Some(Screen::Timezone),
            Screen::User => Some(Screen::Locale),
            Screen::Password => Some(Screen::User),
            Screen::Git => Some(Screen::Password),
            Screen::Disk => Some(Screen::Git),
            Screen::Summary => Some(Screen::Disk),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Back,
    Next,
    Quit,
    Reboot,
    ViewLicense,
    Connect,
    Rescan,
    NetMenu,
    NetRetry,
    OpenDiskUtility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quit {
    Normal,
    Installing,
}

// -- list picker (timezone / locale) -----------------------------------------

pub struct PickItem {
    pub code: String,
    pub name: String,
    pub extra: String,
}

pub struct Picker {
    pub items: Vec<PickItem>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub query: Input,
    pub searching: bool,
}

impl Picker {
    pub fn new(items: Vec<PickItem>) -> Self {
        let filtered = (0..items.len()).collect();
        Self {
            items,
            filtered,
            selected: 0,
            query: Input::default(),
            searching: false,
        }
    }

    pub fn refilter(&mut self) {
        let needle = self.query.value().to_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                needle.is_empty()
                    || item.code.to_lowercase().contains(&needle)
                    || item.name.to_lowercase().contains(&needle)
                    || item.extra.to_lowercase().contains(&needle)
            })
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered.len() {
            self.selected = 0;
        }
    }

    pub fn up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = (self.selected + self.filtered.len() - 1) % self.filtered.len();
    }

    pub fn down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.filtered.len();
    }

    pub fn current(&self) -> Option<&PickItem> {
        let &idx = self.filtered.get(self.selected)?;
        self.items.get(idx)
    }

    pub fn select_code(&mut self, code: &str) {
        if let Some(pos) = self.filtered.iter().position(|&i| self.items[i].code == code) {
            self.selected = pos;
        }
    }
}

// -- per-screen state ---------------------------------------------------------

/// The Red screen is its own state machine.
pub enum RedFase {
    Lista,
    Contrasena {
        ssid: String,
        input: Input,
        visible: bool,
    },
    Conectando {
        ssid: String,
    },
    Error {
        ssid: String,
        msg: String,
    },
    Conectado {
        ssid: String,
    },
}

impl RedFase {
    pub fn ssid(&self) -> Option<&str> {
        match self {
            RedFase::Lista => None,
            RedFase::Contrasena { ssid, .. }
            | RedFase::Conectando { ssid }
            | RedFase::Error { ssid, .. }
            | RedFase::Conectado { ssid } => Some(ssid),
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, RedFase::Conectado { .. })
    }
}

pub struct NetState {
    pub backend: std::sync::Arc<dyn net::WifiBackend>,
    pub phase: RedFase,
    pub ethernet: bool,
    pub online: bool,
    pub wifi: Vec<net::Wifi>,
    pub selected: usize,
    /// When the current connection attempt started (for the 30 s timeout).
    pub connecting_since: Option<Instant>,
}

pub struct HostState {
    pub input: Input,
}

pub struct UserState {
    pub username: Input,
    pub name: Input,
    pub focus: usize,
}

pub struct PasswordState {
    pub p1: Input,
    pub p2: Input,
    pub focus: usize,
}

pub struct GitState {
    pub enabled: bool,
    pub name: Input,
    pub email: Input,
    pub focus: usize,
}

pub struct DiskState {
    pub disks: Vec<Disk>,
    pub index: usize,
    pub layout: Vec<NamedPartition>,
    pub confirmed: bool,
}

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
        while self.log.len() > 400 {
            self.log.pop_front();
        }
    }

    pub fn visible_log(&self, lines: usize) -> Vec<&String> {
        let skip = self.log.len().saturating_sub(lines);
        self.log.iter().skip(skip).collect()
    }
}

pub struct DoneInfo {
    pub host: String,
    pub duration: Duration,
    pub ok: bool,
}

pub struct App {
    pub palette: Palette,
    pub screen: Screen,

    pub dry_run: bool,
    pub repo_spec: String,
    pub repo_dir: PathBuf,
    pub source_ok: bool,
    pub network: bool,

    pub settings: Settings,
    pub settings_ok: bool,

    // welcome
    pub arch: Option<String>,
    pub virt: Option<String>,

    // network
    pub net: NetState,

    // license
    pub license_accepted: bool,
    pub license: Option<LicenseViewer>,

    // host
    pub host: HostState,

    // pickers
    pub timezones: Option<Picker>,
    pub loc: Option<Picker>,

    // user / password / git
    pub user: UserState,
    pub password: PasswordState,
    pub git: GitState,

    // disk / summary
    pub disks: DiskState,
    pub keep_uutils: bool,

    // installing / done
    pub install: InstallView,
    install_rx: Option<Receiver<InstallMsg>>,
    net_rx: Option<Receiver<(String, std::result::Result<bool, String>)>>,
    pub done: Option<DoneInfo>,

    /// Password hash, kept in memory only (never logged or written to temp).
    pub password_hash: Option<String>,

    pub button_focus: usize,
    pub show_commands: bool,
    pub status: Option<String>,
    pub quit: Option<Quit>,
    pub should_quit: bool,
    pub tick: u64,
    pub want_disk_utility: bool,
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
        let arch = detect::arch();
        if let Some(system) = detect::nix_system() {
            settings.system = system;
        }
        // Hostname defaults per machine, as specified.
        settings.host_name = detect::default_hostname(arch.as_deref().unwrap_or_default());

        let disks = detect::disks();
        let mut app = Self {
            palette,
            screen: Screen::Welcome,
            dry_run,
            repo_spec,
            repo_dir,
            source_ok: false,
            network: false,
            settings_ok,
            host: HostState {
                input: Input::new(settings.host_name.clone()),
            },
            user: UserState {
                username: Input::new(settings.username.clone()),
                name: Input::new(settings.userdescription.clone()),
                focus: 0,
            },
            password: PasswordState {
                p1: Input::default(),
                p2: Input::default(),
                focus: 0,
            },
            git: GitState {
                enabled: false,
                name: Input::new(settings.git_name.clone().unwrap_or_default()),
                email: Input::new(settings.git_email.clone().unwrap_or_default()),
                focus: 0,
            },
            disks: DiskState {
                disks,
                index: 0,
                layout: Vec::new(),
                confirmed: false,
            },
            net: NetState {
                backend: net::backend(dry_run),
                phase: RedFase::Lista,
                ethernet: net::ethernet_connected(),
                online: false,
                wifi: Vec::new(),
                selected: 0,
                connecting_since: None,
            },
            net_rx: None,
            license_accepted: false,
            license: None,
            timezones: None,
            loc: None,
            keep_uutils: false,
            install: InstallView::new(),
            install_rx: None,
            done: None,
            password_hash: None,
            button_focus: 1,
            show_commands: false,
            status: None,
            quit: None,
            should_quit: false,
            tick: 0,
            want_disk_utility: false,
            arch,
            virt: detect::environment(),
            settings,
        };
        app.load_layout();
        app
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
            if self.want_disk_utility {
                self.want_disk_utility = false;
                self.run_disk_utility(terminal)?;
            }
            self.tick = self.tick.wrapping_add(1);
            self.poll_install();
            self.poll_network();

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    /// Leaves the TUI, runs the standalone disk utility, then comes back and
    /// refreshes the disk list.
    fn run_disk_utility(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        let result = crate::diskui::run(self.dry_run);
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        enable_raw_mode()?;
        terminal.clear()?;
        if let Err(err) = result {
            self.status = Some(format!("utilidad de disco: {err}"));
        }
        self.disks.disks = detect::disks();
        self.load_layout();
        Ok(())
    }

    // -- data -----------------------------------------------------------------

    pub fn detected_system(&self) -> String {
        self.settings.system.clone()
    }

    fn load_layout(&mut self) {
        if let Some(disk) = self.disks.disks.get(self.disks.index) {
            let device = disk.path();
            self.disks.layout = disk::layout(&self.repo_dir, &device)
                .unwrap_or_else(|_| disk::fallback_layout());
        } else {
            self.disks.layout = disk::fallback_layout();
        }
    }

    pub fn cycle_disk(&mut self, delta: isize) {
        if self.disks.disks.is_empty() {
            return;
        }
        let len = self.disks.disks.len() as isize;
        self.disks.index = ((self.disks.index as isize + delta).rem_euclid(len)) as usize;
        self.load_layout();
        self.disks.confirmed = false;
    }

    fn ensure_timezones(&mut self) {
        if self.timezones.is_some() {
            return;
        }
        let items = crate::screens::timezone::load_items();
        let mut picker = Picker::new(items);
        picker.select_code(&self.settings.time_zone);
        self.timezones = Some(picker);
    }

    fn ensure_locales(&mut self) {
        if self.loc.is_some() {
            return;
        }
        let items = crate::screens::locale::load_items();
        let mut picker = Picker::new(items);
        picker.select_code(&self.settings.default_locale);
        self.loc = Some(picker);
    }

    pub fn hostname_valid(&self) -> bool {
        let name = self.host.input.value();
        !name.is_empty()
            && name.len() <= 63
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    }

    pub fn username_valid(&self) -> bool {
        let name = self.user.username.value();
        let mut chars = name.chars();
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() || c == '_' => {}
            _ => return false,
        }
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    }

    pub fn passwords_match(&self) -> bool {
        !self.password.p1.value().is_empty()
            && self.password.p1.value() == self.password.p2.value()
    }

    fn apply_current(&mut self) {
        match self.screen {
            Screen::Host => self.settings.host_name = self.host.input.value().to_string(),
            Screen::User => {
                self.settings.username = self.user.username.value().to_string();
                self.settings.userdescription = self.user.name.value().to_string();
            }
            Screen::Git => {
                if self.git.enabled {
                    self.settings.git_name = Some(self.git.name.value().to_string());
                    self.settings.git_email = Some(self.git.email.value().to_string());
                } else {
                    self.settings.git_name = None;
                    self.settings.git_email = None;
                }
            }
            Screen::Timezone => {
                if let Some(p) = &self.timezones {
                    if let Some(item) = p.current() {
                        self.settings.time_zone = item.code.clone();
                    }
                }
            }
            Screen::Locale => {
                if let Some(p) = &self.loc {
                    if let Some(item) = p.current() {
                        self.settings.default_locale = item.code.clone();
                    }
                }
            }
            _ => {}
        }
    }

    pub fn selected_disk(&self) -> Option<&Disk> {
        self.disks.disks.get(self.disks.index)
    }

    // -- install --------------------------------------------------------------

    pub fn start_install(&mut self) {
        let Some(disk) = self.selected_disk().cloned() else {
            self.status = Some("no hay disco seleccionado".into());
            return;
        };
        let hash = if let Some(hash) = self.password_hash.clone() {
            hash
        } else {
            let password = self.password.p1.value().to_string();
            let hash = if self.dry_run {
                "$y$j9T$SIMULATED".to_string()
            } else {
                match install::hash_password(&password) {
                    Ok(hash) => hash,
                    Err(err) => {
                        self.status = Some(format!("mkpasswd: {err}"));
                        return;
                    }
                }
            };
            self.password_hash = Some(hash.clone());
            hash
        };

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
            password_hash: hash,
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

    // -- events ---------------------------------------------------------------

    pub fn handle_event(&mut self, event: Event) {
        let Event::Key(key) = event else { return };
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return;
        }

        // Quit confirmation modal owns the keyboard.
        if self.quit.is_some() {
            self.handle_quit(key);
            return;
        }

        // Full-screen license viewer.
        if self.license.is_some() {
            self.license_key(key);
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = Some(if self.screen == Screen::Installing {
                Quit::Installing
            } else {
                Quit::Normal
            });
            return;
        }

        // Screen-local handling (text fields, list nav, search) first.
        if self.route_screen_key(key) {
            return;
        }

        match key.code {
            KeyCode::Char('v') => {
                self.show_commands = !self.show_commands;
            }
            KeyCode::Char('q') if self.screen != Screen::Installing => {
                self.quit = Some(Quit::Normal);
            }
            KeyCode::Tab | KeyCode::BackTab => self.move_button(1),
            KeyCode::Left | KeyCode::Right if self.screen_uses_button_arrows() => self.move_button_key(key.code),
            KeyCode::Enter => self.activate_focused(),
            KeyCode::Esc => {
                if self.screen == Screen::Installing {
                    self.quit = Some(Quit::Installing);
                } else {
                    self.go_back();
                }
            }
            _ => {}
        }
    }

    fn route_screen_key(&mut self, key: KeyEvent) -> bool {
        match self.screen {
            Screen::Welcome => crate::screens::welcome::key(self, key),
            Screen::Network => crate::screens::network::key(self, key),
            Screen::License => crate::screens::license::key(self, key),
            Screen::Host => crate::screens::host::key(self, key),
            Screen::Timezone => crate::screens::timezone::key(self, key),
            Screen::Locale => crate::screens::locale::key(self, key),
            Screen::User => crate::screens::user::key(self, key),
            Screen::Password => crate::screens::password::key(self, key),
            Screen::Git => crate::screens::git::key(self, key),
            Screen::Disk => crate::screens::disk::key(self, key),
            Screen::Summary => crate::screens::summary::key(self, key),
            Screen::Installing => crate::screens::installing::key(self, key),
            Screen::Done => crate::screens::done::key(self, key),
        }
    }

    fn screen_uses_button_arrows(&self) -> bool {
        matches!(
            self.screen,
            Screen::Welcome | Screen::License | Screen::Installing | Screen::Done
        )
    }

    fn move_button(&mut self, delta: isize) {
        let count = self.buttons().len();
        if count == 0 {
            return;
        }
        self.button_focus = ((self.button_focus as isize + delta).rem_euclid(count as isize)) as usize;
    }

    fn move_button_key(&mut self, code: KeyCode) {
        self.move_button(if code == KeyCode::Left { -1 } else { 1 });
    }

    pub fn buttons(&self) -> Vec<(Btn, Action)> {
        match self.screen {
            Screen::Welcome => crate::screens::welcome::buttons(self),
            Screen::Network => crate::screens::network::buttons(self),
            Screen::License => crate::screens::license::buttons(self),
            Screen::Host => crate::screens::host::buttons(self),
            Screen::Timezone => crate::screens::timezone::buttons(self),
            Screen::Locale => crate::screens::locale::buttons(self),
            Screen::User => crate::screens::user::buttons(self),
            Screen::Password => crate::screens::password::buttons(self),
            Screen::Git => crate::screens::git::buttons(self),
            Screen::Disk => crate::screens::disk::buttons(self),
            Screen::Summary => crate::screens::summary::buttons(self),
            Screen::Installing => Vec::new(),
            Screen::Done => crate::screens::done::buttons(self),
        }
    }

    fn activate_focused(&mut self) {
        let buttons = self.buttons();
        if let Some((_, action)) = buttons.get(self.button_focus) {
            self.activate(*action);
        }
    }

    fn activate(&mut self, action: Action) {
        match action {
            Action::Back => self.go_back(),
            Action::Next => self.advance(),
            Action::Quit => self.quit = Some(Quit::Normal),
            Action::Reboot => crate::screens::done::reboot(self),
            Action::ViewLicense => self.open_license(),
            Action::Connect => crate::screens::network::connect(self),
            Action::Rescan => crate::screens::network::rescan(self),
            Action::NetMenu => crate::screens::network::to_menu(self),
            Action::NetRetry => crate::screens::network::retry(self),
            Action::OpenDiskUtility => self.want_disk_utility = true,
            Action::None => {}
        }
    }

    pub fn open_license(&mut self) {
        let text = std::fs::read_to_string(self.repo_dir.join("LICENSE")).unwrap_or_else(|_| {
            "GNU GENERAL PUBLIC LICENSE\nVersion 2, June 1991\n".to_string()
        });
        self.license = Some(LicenseViewer::new(&text, 76));
    }

    pub fn license_key(&mut self, key: KeyEvent) {
        let Some(viewer) = self.license.as_mut() else {
            return;
        };
        let page = 12usize;
        if viewer.searching {
            match key.code {
                KeyCode::Enter => viewer.searching = false,
                KeyCode::Esc => {
                    viewer.searching = false;
                    viewer.query.clear();
                }
                KeyCode::Backspace => {
                    viewer.query.pop();
                }
                KeyCode::Char(c) => viewer.query.push(c),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Up => viewer.scroll_by(-1, page),
            KeyCode::Down => viewer.scroll_by(1, page),
            KeyCode::PageUp => viewer.scroll_by(-(page as isize), page),
            KeyCode::PageDown | KeyCode::Char(' ') => viewer.scroll_by(page as isize, page),
            KeyCode::Home | KeyCode::Char('g') => viewer.home(),
            KeyCode::End | KeyCode::Char('G') => viewer.end(page),
            KeyCode::Char('/') => {
                viewer.searching = true;
                viewer.query.clear();
            }
            KeyCode::Char('n') => viewer.next_match(),
            KeyCode::Char('N') => viewer.prev_match(),
            KeyCode::Char('q') | KeyCode::Esc => self.license = None,
            _ => {}
        }
        if let Some(viewer) = self.license.as_mut() {
            if viewer.searching {
                return;
            }
            let query = viewer.query.clone();
            if !query.is_empty() {
                viewer.set_query(query);
            }
        }
    }

    fn handle_quit(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('s') | KeyCode::Enter => self.should_quit = true,
            KeyCode::Char('n') | KeyCode::Esc => self.quit = None,
            _ => {}
        }
    }

    /// Points the button focus at the primary button of the current screen.
    pub fn focus_primary(&mut self) {
        let buttons = self.buttons();
        self.button_focus = buttons
            .iter()
            .position(|(b, _)| b.kind == crate::widgets::Kind::Primary)
            .unwrap_or_else(|| buttons.len().saturating_sub(1));
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.screen.previous() {
            self.screen = prev;
            self.focus_primary();
            self.status = None;
        }
    }

    pub fn advance(&mut self) {
        self.apply_current();
        if !self.primary_enabled() {
            return;
        }
        match self.screen.next() {
            Some(Screen::Installing) => {
                self.screen = Screen::Installing;
                self.show_commands = true;
                self.start_install();
            }
            Some(next) => {
                self.screen = next;
                self.focus_primary();
                self.status = None;
                match next {
                    Screen::Timezone => self.ensure_timezones(),
                    Screen::Locale => self.ensure_locales(),
                    Screen::Network => self.refresh_network(),
                    _ => {}
                }
            }
            None => {}
        }
    }

    pub fn refresh_network(&mut self) {
        self.net.ethernet = net::ethernet_connected();
        self.net.online = self.net.backend.status();
        if self.net.ethernet && self.net.online {
            self.net.phase = RedFase::Conectado {
                ssid: "ethernet".to_string(),
            };
            return;
        }
        self.net.wifi = self.net.backend.scan();
        if self.net.wifi.is_empty() && self.dry_run {
            self.net.wifi = net::sample_networks();
        }
        self.net.selected = 0;
        self.net.phase = RedFase::Lista;
    }

    /// Starts a connection attempt on a worker thread (non-blocking), with the
    /// password never touching the log.
    pub fn start_net_connect(&mut self, ssid: String, pass: String) {
        let backend = self.net.backend.clone();
        let (tx, rx) = mpsc::channel();
        self.net_rx = Some(rx);
        self.net.connecting_since = Some(Instant::now());
        self.net.phase = RedFase::Conectando { ssid: ssid.clone() };
        std::thread::spawn(move || {
            let result = backend
                .connect(&ssid, &pass)
                .map(|()| backend.status())
                .map_err(|err| err.to_string());
            let _ = tx.send((ssid, result));
        });
    }

    pub(crate) fn poll_network(&mut self) {
        let Some(rx) = self.net_rx.take() else {
            return;
        };
        let mut reply = None;
        while let Ok(msg) = rx.try_recv() {
            reply = Some(msg);
        }
        if let Some((ssid, result)) = reply {
            self.net.connecting_since = None;
            self.net.phase = match result {
                Ok(true) => {
                    self.net.online = true;
                    RedFase::Conectado { ssid }
                }
                Ok(false) => RedFase::Error {
                    ssid,
                    msg: "sin salida a cache.nixos.org".into(),
                },
                Err(msg) => RedFase::Error { ssid, msg },
            };
            self.focus_primary();
        } else if let Some(since) = self.net.connecting_since {
            if since.elapsed() > Duration::from_secs(30) {
                let ssid = self.net.phase.ssid().unwrap_or_default().to_string();
                self.net.connecting_since = None;
                self.net.phase = RedFase::Error {
                    ssid,
                    msg: "sin respuesta".into(),
                };
            } else {
                self.net_rx = Some(rx);
            }
        } else {
            self.net_rx = Some(rx);
        }
    }

    pub fn primary_enabled(&self) -> bool {
        match self.screen {
            Screen::Network => self.net.phase.is_connected(),
            Screen::License => self.license_accepted,
            Screen::Host => self.hostname_valid(),
            Screen::User => self.username_valid(),
            Screen::Password => self.passwords_match(),
            Screen::Disk => self.disks.confirmed && self.selected_disk().is_some(),
            _ => true,
        }
    }

    // -- rendering ------------------------------------------------------------

    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        f.render_widget(
            Block::default().style(Style::default().bg(self.palette.reset)),
            area,
        );

        if self.license.is_some() {
            let viewer_area = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), area.height.saturating_sub(3));
            let scroll = self.license.as_ref().map(|v| v.scroll).unwrap_or(0);
            let _ = scroll;
            if let Some(viewer) = self.license.as_mut() {
                let inner_w = viewer_area.width.saturating_sub(2) as usize;
                viewer.set_width(inner_w);
                let visible = viewer_area.height.saturating_sub(2) as usize;
                viewer.scroll = viewer.scroll.min(viewer.max_scroll(visible));
                viewer.draw(f, viewer_area, &self.palette);
            }
            return;
        }

        let content_h = self.content_height();
        let commands_lines = self.command_list();
        let commands_h = if self.show_commands {
            (commands_lines.len() as u16 + 2).clamp(2, area.height / 2)
        } else {
            0
        };
        let panel_h = content_h + 4;
        let width = PANEL_WIDTH.min(area.width);
        let gap: u16 = 1;
        let mut total = 1 + 2 + gap + panel_h + gap + 1;
        if commands_h > 0 {
            total += gap + commands_h;
        }
        let top = area.y + area.height.saturating_sub(total) / 2;
        let x = area.x + area.width.saturating_sub(width) / 2;

        let header = Rect::new(x, top, width, 1);
        let progress = Rect::new(x, top + 1, width, 2);
        let panel = Rect::new(x, top + 3 + gap, width, panel_h);
        let shortcuts = Rect::new(x, panel.bottom() + gap, width, 1);
        let commands = Rect::new(x, shortcuts.bottom() + gap, width, commands_h);

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
            Screen::Network => crate::screens::network::height(self),
            Screen::License => crate::screens::license::height(self),
            Screen::Host => crate::screens::host::height(self),
            Screen::Timezone => crate::screens::timezone::height(self),
            Screen::Locale => crate::screens::locale::height(self),
            Screen::User => crate::screens::user::height(self),
            Screen::Password => crate::screens::password::height(self),
            Screen::Git => crate::screens::git::height(self),
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
                Style::default().fg(self.palette.primary).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" instalador", Style::default().fg(self.palette.muted)),
        ];
        if self.dry_run {
            spans.push(Span::styled(
                "  · simulación",
                Style::default().fg(self.palette.warning),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(spans)).alignment(Alignment::Center), area);
    }

    fn draw_progress(&self, f: &mut Frame, area: Rect) {
        // 36 cells, 3 per step.
        let cells: usize = 36;
        let filled = if self.screen == Screen::Done {
            cells
        } else {
            ((self.screen.index() + 1) * 3).min(cells)
        };
        let label = format!("  {}/{}", (self.screen.index() + 1).min(STEPS), STEPS);
        let bar_width = area.width as usize;
        let filled = filled.min(bar_width);
        let line = Line::from(vec![
            Span::styled("━".repeat(filled), Style::default().fg(self.palette.primary)),
            Span::styled(
                "─".repeat(bar_width.saturating_sub(filled)),
                Style::default().fg(self.palette.muted),
            ),
        ])
        .alignment(Alignment::Center);
        let _ = label;
        f.render_widget(Paragraph::new(line), area);
        let label_area = Rect::new(area.x, area.y + 1, area.width, 1);
        f.render_widget(
            Paragraph::new(Span::styled(label, Style::default().fg(self.palette.muted)))
                .alignment(Alignment::Center),
            label_area,
        );
    }

    fn draw_panel(&self, f: &mut Frame, area: Rect) {
        let border = match self.screen {
            Screen::Done => self.palette.success,
            Screen::Installing if self.install.error.is_some() => self.palette.error,
            _ => self.palette.primary,
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border))
            .title(Span::styled(
                format!(" {} ", self.screen.title()),
                Style::default().fg(border),
            ));
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
            Screen::Network => crate::screens::network::draw(f, self, content),
            Screen::License => crate::screens::license::draw(f, self, content),
            Screen::Host => crate::screens::host::draw(f, self, content),
            Screen::Timezone => crate::screens::timezone::draw(f, self, content),
            Screen::Locale => crate::screens::locale::draw(f, self, content),
            Screen::User => crate::screens::user::draw(f, self, content),
            Screen::Password => crate::screens::password::draw(f, self, content),
            Screen::Git => crate::screens::git::draw(f, self, content),
            Screen::Disk => crate::screens::disk::draw(f, self, content),
            Screen::Summary => crate::screens::summary::draw(f, self, content),
            Screen::Installing => crate::screens::installing::draw(f, self, content),
            Screen::Done => crate::screens::done::draw(f, self, content),
        }

        let buttons: Vec<Btn> = self.buttons().into_iter().map(|(b, _)| b).collect();
        if !buttons.is_empty() {
            let row = Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1);
            widgets::render_buttons(f, row, &self.palette, &buttons, self.button_focus);
        }
    }

    fn draw_shortcuts(&self, f: &mut Frame, area: Rect) {
        let toggle = format!("v comandos: {}", if self.show_commands { "on" } else { "off" });
        let (line, color) = if let Some(status) = &self.status {
            (status.clone(), self.palette.warning)
        } else {
            match self.screen {
                Screen::Summary => (format!("no se puede deshacer · {toggle}"), self.palette.warning),
                Screen::Done => (
                    format!("retira la ISO antes de reiniciar · {toggle}"),
                    self.palette.muted,
                ),
                Screen::Installing => (format!("ctrl+c cancelar · {toggle}"), self.palette.muted),
                Screen::Network => (
                    format!("{} · {toggle}", crate::screens::network::shortcut(self)),
                    self.palette.muted,
                ),
                _ => {
                    let mut parts: Vec<&str> = match self.screen {
                        Screen::Welcome => vec!["enter continuar", "esc salir"],
                        Screen::License => vec!["espacio aceptar", "l leer completa"],
                        Screen::Host | Screen::User | Screen::Password | Screen::Git => {
                            vec!["↑↓ campo", "tab botones"]
                        }
                        Screen::Timezone | Screen::Locale => vec!["↑↓ lista", "/ buscar"],
                        Screen::Disk => vec!["↑↓ disco", "espacio borrar", "u utilidad"],
                        _ => vec!["tab botones"],
                    };
                    parts.push("v comandos");
                    (format!("{} · {toggle}", parts.join(" · ")), self.palette.muted)
                }
            }
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(line, Style::default().fg(color))))
                .alignment(Alignment::Center),
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
                .map(|l| Line::from(Span::styled(format!("$ {l}"), Style::default().fg(self.palette.muted))))
                .collect()
        };
        f.render_widget(Paragraph::new(visible).wrap(Wrap { trim: true }), inner);
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
        widgets::dim(f, area, &self.palette);
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
            Line::from(Span::styled(message, Style::default().fg(self.palette.text)))
                .alignment(Alignment::Center),
            Line::from(""),
            Line::from(Span::styled("s / enter · n / esc", Style::default().fg(self.palette.muted)))
                .alignment(Alignment::Center),
        ];
        f.render_widget(Paragraph::new(text), inner);
    }

    pub fn command_list(&self) -> Vec<String> {
        match self.screen {
            Screen::Welcome => crate::screens::welcome::commands(self),
            Screen::Network => crate::screens::network::commands(self),
            Screen::License => crate::screens::license::commands(self),
            Screen::Host => crate::screens::host::commands(self),
            Screen::Timezone => crate::screens::timezone::commands(self),
            Screen::Locale => crate::screens::locale::commands(self),
            Screen::User => crate::screens::user::commands(self),
            Screen::Password => crate::screens::password::commands(self),
            Screen::Git => crate::screens::git::commands(self),
            Screen::Disk => crate::screens::disk::commands(self),
            Screen::Summary => crate::screens::summary::commands(self),
            Screen::Installing => crate::screens::installing::commands(self),
            Screen::Done => crate::screens::done::commands(self),
        }
    }
}


