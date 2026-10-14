//! Application state and the terminal chrome shared by every screen.
//!
//! Chrome is always: one header line, a 36-cell text progress bar (3 cells per
//! step, `n/12`), one centered rounded panel with its title embedded in the top
//! border, a button row, and an optional `v` commands column.

use crate::detect::{self, Disk};
use crate::disk::{self, NamedPartition};
use crate::install::{self, Msg as InstallMsg, Plan, Step};
use crate::net;
use crate::settings::Settings;
use crate::theme::{self, Palette};
use crate::widgets::{self, Btn, Kind};
use color_eyre::eyre::Result;
use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
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
    /// Startup splash: logo + prompt + a single Continuar button.
    Splash,
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
        Screen::License,
        Screen::Network,
        Screen::Locale,
        Screen::Timezone,
        Screen::Host,
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
            Screen::Splash => "Inicio",
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
            Screen::Splash => Some(Screen::Welcome),
            Screen::Welcome => Some(Screen::License),
            Screen::License => Some(Screen::Network),
            Screen::Network => Some(Screen::Locale),
            Screen::Locale => Some(Screen::Timezone),
            Screen::Timezone => Some(Screen::Host),
            Screen::Host => Some(Screen::User),
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
            Screen::Splash | Screen::Welcome | Screen::Installing | Screen::Done => None,
            Screen::License => Some(Screen::Welcome),
            Screen::Network => Some(Screen::License),
            Screen::Locale => Some(Screen::Network),
            Screen::Timezone => Some(Screen::Locale),
            Screen::Host => Some(Screen::Timezone),
            Screen::User => Some(Screen::Host),
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
    Connect,
    Rescan,
    NetMenu,
    NetRetry,
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
    /// Index into `filtered`. `None` until the user picks a row.
    pub selected: Option<usize>,
    pub query: Input,
}

impl Picker {
    pub fn new(items: Vec<PickItem>) -> Self {
        let filtered = (0..items.len()).collect();
        Self {
            items,
            filtered,
            selected: None,
            query: Input::default(),
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
        // Keep the selection only if it still points at a valid row.
        if let Some(sel) = self.selected {
            if sel >= self.filtered.len() {
                self.selected = None;
            }
        }
    }

    pub fn up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(0) => self.filtered.len() - 1,
            Some(s) => s - 1,
        });
    }

    pub fn down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            None => 0,
            Some(s) if s + 1 >= self.filtered.len() => 0,
            Some(s) => s + 1,
        });
    }

    pub fn current(&self) -> Option<&PickItem> {
        let idx = self.filtered.get(self.selected?)?;
        self.items.get(*idx)
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
    /// 0 = nombre, 1 = usuario. Focus starts on the name.
    pub focus: usize,
    /// Once the user edits the username by hand it stops being regenerated
    /// from the name (unless it is emptied).
    pub edited: bool,
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
    /// A Windows install was detected on some disk.
    pub windows: bool,
    /// 0 = erase the whole disk, 1 = install alongside Windows.
    pub option: usize,
    /// Confirmation for the destructive full erase.
    pub confirmed: bool,
}

impl DiskState {
    /// The options offered for the current machine.
    pub fn options(&self) -> &'static [&'static str] {
        if self.windows {
            &["Borrar todo el disco", "Instalar junto a Windows"]
        } else {
            &["Borrar todo el disco"]
        }
    }
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

    /// Machine class (thinkpad | utm), detected from arch + virt.
    pub profile: detect::Profile,

    // welcome
    pub arch: Option<String>,
    pub virt: Option<String>,

    // network
    pub net: NetState,

    // license
    pub license_accepted: bool,
    /// Scroll offset (wrapped lines) of the license text.
    pub license_scroll: usize,
    pub license_max_scroll: usize,
    /// 0 = text, 1 = the Acepto checkbox.
    pub license_focus: usize,

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

    /// Button selection index. `focus_buttons` is false while the content
    /// (a field or the list) holds the focus.
    pub button_focus: usize,
    pub focus_buttons: bool,
    /// On-screen x-ranges of the enabled buttons, for mouse hover/click.
    button_hitboxes: Vec<(u16, u16, usize)>,
    pub show_commands: bool,
    pub status: Option<String>,
    pub quit: Option<Quit>,
    pub should_quit: bool,
    pub tick: u64,
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
        let profile = detect::Profile::detect();
        // Hostname defaults per machine profile; everything else starts empty
        // (only stateVersion is kept from the repo).
        settings.host_name = profile.default_hostname().to_string();
        settings.username = String::new();
        settings.userdescription = String::new();
        settings.git_name = None;
        settings.git_email = None;
        settings.time_zone = String::new();
        settings.default_locale = String::new();

        let disks = detect::disks();
        let mut app = Self {
            palette,
            screen: Screen::Splash,
            dry_run,
            repo_spec,
            repo_dir,
            source_ok: false,
            network: false,
            settings_ok,
            profile,
            host: HostState {
                input: Input::new(settings.host_name.clone()),
            },
            user: UserState {
                username: Input::default(),
                name: Input::default(),
                focus: 0,
                edited: false,
            },
            password: PasswordState {
                p1: Input::default(),
                p2: Input::default(),
                focus: 0,
            },
            git: GitState {
                enabled: false,
                name: Input::default(),
                email: Input::default(),
                focus: 0,
            },
            disks: DiskState {
                disks,
                index: 0,
                layout: Vec::new(),
                windows: detect::windows_present(),
                option: 0,
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
            license_scroll: 0,
            license_max_scroll: 0,
            license_focus: 0,
            timezones: None,
            loc: None,
            keep_uutils: false,
            install: InstallView::new(),
            install_rx: None,
            done: None,
            password_hash: None,
            button_focus: 1,
            // Welcome has no content, so the buttons start focused.
            focus_buttons: true,
            button_hitboxes: Vec::new(),
            show_commands: false,
            status: None,
            quit: None,
            should_quit: false,
            tick: 0,
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
            self.tick = self.tick.wrapping_add(1);
            self.poll_install();
            self.poll_network();

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    // -- data -----------------------------------------------------------------

    pub fn detected_system(&self) -> String {
        self.profile.system().to_string()
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

    /// Whether the Disk screen's choice is complete.
    fn disk_ready(&self) -> bool {
        if self.selected_disk().is_none() {
            return false;
        }
        if self.disks.option == 1 {
            // "Instalar junto a Windows" needs no extra confirmation.
            true
        } else {
            self.disks.confirmed
        }
    }

    fn ensure_timezones(&mut self) {
        if self.timezones.is_some() {
            return;
        }
        let items = crate::screens::timezone::load_items();
        self.timezones = Some(Picker::new(items));
    }

    fn ensure_locales(&mut self) {
        if self.loc.is_some() {
            return;
        }
        let items = crate::screens::locale::load_items();
        self.loc = Some(Picker::new(items));
    }

    pub fn hostname_valid(&self) -> bool {
        let name = self.host.input.value();
        !name.is_empty()
            && name.len() <= 63
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    }

    /// Regenerates the username from the name (while it is still automatic).
    pub fn regenerate_username(&mut self) {
        if self.user.edited {
            return;
        }
        let generated = detect::generate_username(self.user.name.value());
        self.user.username = Input::new(generated);
    }

    /// The username is only "automatic" until the user edits it by hand.
    pub fn username_is_auto(&self) -> bool {
        !self.user.edited
    }

    /// uutils is x86_64-only (see modules/common/uutils.nix), so the option is
    /// only offered on the ThinkPad profile.
    pub fn uutils_available(&self) -> bool {
        self.profile == detect::Profile::Thinkpad
    }

    /// The panel title, which for Contraseña names the resulting user.
    pub fn screen_title(&self) -> String {
        match self.screen {
            Screen::License => "Licencia · GNU GPL v2.0".to_string(),
            s => s.title().to_string(),
        }
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
                if self.user.username.value().is_empty() {
                    self.user.edited = false;
                    self.regenerate_username();
                }
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

        let wifi = match &self.net.phase {
            RedFase::Conectado { ssid } if ssid != "ethernet" => Some(install::WifiSave {
                ssid: ssid.clone(),
                networkmanager: self.net.backend.name() == "nmcli",
            }),
            _ => None,
        };

        let plan = Plan {
            source: self.repo_dir.clone(),
            root,
            settings: self.settings.clone(),
            password_hash: hash,
            device: disk.path(),
            // uutils is only offered (and only applied) on the ThinkPad.
            keep_uutils: self.keep_uutils && self.uutils_available(),
            dry_run: self.dry_run,
            profile: self.profile,
            wifi,
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
                self.reset_focus();
            }
        } else {
            self.install_rx = Some(rx);
        }
    }

    // -- events ---------------------------------------------------------------

    pub fn handle_event(&mut self, event: Event) {
        if let Event::Mouse(mouse) = event {
            self.handle_mouse(mouse);
            return;
        }
        let Event::Key(key) = event else { return };
        if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
            return;
        }

        // Quit confirmation modal owns the keyboard.
        if self.quit.is_some() {
            self.handle_quit(key);
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
            // Tab / Shift+Tab move between content and buttons and between
            // buttons; ←/→ sneak into the buttons from the content.
            KeyCode::Tab => self.tab_focus(false),
            KeyCode::BackTab => self.tab_focus(true),
            KeyCode::Left | KeyCode::Right => {
                if self.focus_buttons {
                    self.move_button_key(key.code);
                } else {
                    self.focus_buttons_on_primary();
                }
            }
            KeyCode::Enter => {
                if self.focus_buttons {
                    self.activate_focused();
                } else {
                    // No content consumed it: move the focus to Continuar.
                    self.focus_buttons_on_primary();
                }
            }
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
            Screen::Splash => false,
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

    /// Index of the primary button (first enabled primary, else first enabled).
    fn primary_index(&self) -> usize {
        let buttons = self.buttons();
        buttons
            .iter()
            .position(|(b, _)| b.kind == Kind::Primary && b.enabled)
            .or_else(|| buttons.iter().position(|(b, _)| b.enabled))
            .unwrap_or(0)
    }

    /// Moves the focus to the primary button.
    pub fn focus_buttons_on_primary(&mut self) {
        self.focus_buttons = true;
        self.button_focus = self.primary_index();
    }

    /// The next enabled button in `dir` (skipping the current one), if any.
    fn next_enabled(&self, from: usize, dir: isize) -> Option<usize> {
        let buttons = self.buttons();
        let n = buttons.len();
        if n == 0 {
            return None;
        }
        let mut idx = from as isize;
        for _ in 0..n {
            idx = (idx + dir).rem_euclid(n as isize);
            let i = idx as usize;
            if buttons[i].0.enabled && i != from {
                return Some(i);
            }
        }
        None
    }

    fn tab_focus(&mut self, back: bool) {
        if self.buttons().is_empty() {
            self.focus_buttons = false;
            return;
        }
        if !self.focus_buttons {
            self.focus_buttons_on_primary();
            return;
        }
        let from = self.button_focus;
        match self.next_enabled(from, if back { -1 } else { 1 }) {
            Some(i) => self.button_focus = i,
            None => self.focus_buttons = false,
        }
    }

    fn move_button_key(&mut self, code: KeyCode) {
        let dir = if code == KeyCode::Left { -1 } else { 1 };
        if let Some(i) = self.next_enabled(self.button_focus, dir) {
            self.button_focus = i;
        }
    }

    pub fn buttons(&self) -> Vec<(Btn, Action)> {
        match self.screen {
            Screen::Splash => vec![(Btn::primary("Continuar ›"), Action::Next)],
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
        if let Some((button, action)) = buttons.get(self.button_focus) {
            if button.enabled {
                let action = *action;
                self.activate(action);
            }
        }
    }

    /// Mouse hover selects a button; a left click runs it. Disabled buttons are
    /// not in the hitboxes and are therefore ignored.
    fn handle_mouse(&mut self, mouse: MouseEvent) {
        // Wheel scrolls the license text.
        if self.screen == Screen::License {
            match mouse.kind {
                MouseEventKind::ScrollDown => {
                    self.license_scroll = (self.license_scroll + 1).min(self.license_max_scroll);
                    return;
                }
                MouseEventKind::ScrollUp => {
                    self.license_scroll = self.license_scroll.saturating_sub(1);
                    return;
                }
                _ => {}
            }
        }
        let hit = self
            .button_hitboxes
            .iter()
            .find(|(x0, x1, _)| mouse.column >= *x0 && mouse.column <= *x1)
            .map(|&(_, _, index)| index);
        match (mouse.kind, hit) {
            (MouseEventKind::Moved, Some(index)) => {
                self.focus_buttons = true;
                self.button_focus = index;
            }
            (MouseEventKind::Down(MouseButton::Left), Some(index)) => {
                self.focus_buttons = true;
                self.button_focus = index;
                self.activate_focused();
            }
            _ => {}
        }
    }

    fn activate(&mut self, action: Action) {
        match action {
            Action::Back => self.go_back(),
            Action::Next => self.advance(),
            Action::Quit => self.quit = Some(Quit::Normal),
            Action::Reboot => crate::screens::done::reboot(self),
            Action::Connect => crate::screens::network::connect(self),
            Action::Rescan => crate::screens::network::rescan(self),
            Action::NetMenu => crate::screens::network::to_menu(self),
            Action::NetRetry => crate::screens::network::retry(self),
            Action::None => {}
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
        self.button_focus = self.primary_index();
    }

    /// Resets the focus when entering a screen: the content (a field or the
    /// list) holds it, except on screens without content (Welcome, Done) where
    /// the buttons start focused.
    pub fn reset_focus(&mut self) {
        self.button_focus = self.primary_index();
        self.focus_buttons = matches!(self.screen, Screen::Splash | Screen::Welcome | Screen::Done);
        if self.screen == Screen::License {
            self.license_focus = 0;
        }
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.screen.previous() {
            self.screen = prev;
            self.reset_focus();
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
                self.focus_buttons = false;
                self.show_commands = true;
                self.start_install();
            }
            Some(next) => {
                self.screen = next;
                self.reset_focus();
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
            // On success, light up Continuar so Enter advances right away.
            if self.net.phase.is_connected() {
                self.focus_buttons_on_primary();
            } else {
                self.focus_primary();
            }
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
            Screen::Timezone => self
                .timezones
                .as_ref()
                .map(|p| p.selected.is_some())
                .unwrap_or(false),
            Screen::Locale => self.loc.as_ref().map(|p| p.selected.is_some()).unwrap_or(false),
            Screen::Disk => self.disk_ready(),
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

        if self.screen == Screen::Splash {
            self.draw_splash(f, area);
            if self.quit.is_some() {
                self.draw_quit(f, area);
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
        // The license panel is a touch wider (~64 columns).
        let base_width = if self.screen == Screen::License { 64 } else { PANEL_WIDTH };
        let width = base_width.min(area.width);
        // Inicio and Listo show no progress bar.
        let progress_h: u16 = if self.screen == Screen::Done { 0 } else { 1 };
        let gap: u16 = 1;
        let mut total = 1 + progress_h + gap + panel_h + gap + 1;
        if commands_h > 0 {
            total += gap + commands_h;
        }
        let top = area.y + area.height.saturating_sub(total) / 2;
        let x = area.x + area.width.saturating_sub(width) / 2;

        let header = Rect::new(x, top, width, 1);
        let mut cursor = top + 1;
        let progress = Rect::new(x, cursor, width, progress_h);
        cursor += progress_h + gap;
        let panel = Rect::new(x, cursor, width, panel_h);
        let shortcuts = Rect::new(x, panel.bottom() + gap, width, 1);
        let commands = Rect::new(x, shortcuts.bottom() + gap, width, commands_h);

        self.draw_header(f, header);
        if progress_h > 0 {
            self.draw_progress(f, progress);
        }
        let inner = self.draw_panel(f, panel);
        // The primary "Continuar" (Action::Next) is disabled until the
        // screen's precondition is met.
        let enabled = self.primary_enabled();
        let buttons: Vec<Btn> = self
            .buttons()
            .into_iter()
            .map(|(mut b, action)| {
                if action == Action::Next {
                    b.enabled = enabled;
                }
                b
            })
            .collect();
        if !buttons.is_empty() {
            let row = Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1);
            self.button_hitboxes = widgets::render_buttons(
                f,
                row,
                &self.palette,
                &buttons,
                self.button_focus,
                self.focus_buttons,
            );
        } else {
            self.button_hitboxes.clear();
        }
        self.draw_shortcuts(f, shortcuts);
        if self.show_commands {
            self.draw_commands(f, commands, &commands_lines, commands_h);
        }

        if self.quit.is_some() {
            self.draw_quit(f, area);
        }
    }

    /// The startup splash: NixOS logo + prompt + a single Continuar button,
    /// centered. No header, progress, shortcuts or commands panel.
    fn draw_splash(&mut self, f: &mut Frame, area: Rect) {
        let console = theme::console::is_linux_console();
        let show_logo = area.height >= 24 && area.width >= 40;

        let mut lines: Vec<Line> = Vec::new();
        if show_logo {
            lines.extend(crate::logo::nixos_logo(console));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Para iniciar la configuración del instalador, dale a Continuar",
            Style::default().fg(self.palette.text),
        )));

        let content_h = lines.len() as u16;
        let total_h = content_h + 2; // blank line + button row
        let top = area.y + area.height.saturating_sub(total_h) / 2;
        f.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            Rect::new(area.x, top, area.width, content_h),
        );

        let row = Rect::new(area.x, top + content_h + 1, area.width, 1);
        let buttons: Vec<Btn> = self.buttons().into_iter().map(|(b, _)| b).collect();
        self.button_hitboxes = widgets::render_buttons(
            f,
            row,
            &self.palette,
            &buttons,
            self.button_focus,
            self.focus_buttons,
        );
    }

    fn content_height(&self) -> u16 {
        match self.screen {
            Screen::Splash => 0,
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
        // 36 cells, 3 per step from Bienvenido (1/12) to Instalando (12/12).
        let cells: usize = 36;
        let step = (self.screen.index() + 1).min(STEPS);
        let filled = (step * 3).min(cells);
        let line = Line::from(vec![
            Span::styled("━".repeat(filled), Style::default().fg(self.palette.primary)),
            Span::styled(
                "─".repeat(cells - filled),
                // LBG, not muted.
                Style::default().fg(self.palette.row_bg),
            ),
            Span::styled(
                format!("  {step}/{STEPS}"),
                Style::default().fg(self.palette.muted),
            ),
        ])
        .alignment(Alignment::Center);
        f.render_widget(Paragraph::new(line), area);
    }

    fn draw_panel(&mut self, f: &mut Frame, area: Rect) -> Rect {
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
                format!(" {} ", self.screen_title()),
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
            Screen::Splash => {}
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

        inner
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
            Screen::Splash => Vec::new(),
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


