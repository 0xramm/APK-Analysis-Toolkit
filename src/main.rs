mod modules;
mod util;

use std::io;
use std::panic;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use modules::{apps, emulator, mitm, ssl_pin, template, update};
use util::shell;

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Main,
    Emulator,
    SslPin,
    Mitm,
    Apps,
}

#[derive(Clone, Copy, PartialEq)]
enum PendingAction {
    SslBypass,
    StopBypass,
    PackageInfo,
    Launch,
    InstallApk,
    Uninstall,
    Disable,
    Enable,
    SelectDevice,
}

const MAIN_ITEMS: [&str; 7] = [
    "Run full setup (template)",
    "Emulator",
    "SSL pinning bypass",
    "mitmproxy",
    "App manager (adb)",
    "Select ADB device",
    "Quit",
];
const EMULATOR_ITEMS: [&str; 6] = [
    "Detect & list AVDs",
    "Create new AVD (if none exist)",
    "Launch existing AVD (writable system)",
    "Root device (adb root)",
    "De-root device (adb unroot)",
    "Back",
];
const SSLPIN_ITEMS: [&str; 5] = [
    "Install frida-tools & objection",
    "Push & start frida-server on device",
    "Bypass pinning for a package",
    "Stop bypass / re-pin a package",
    "Back",
];
const MITM_ITEMS: [&str; 4] = [
    "Install mitmproxy",
    "Start proxy & point device at it",
    "Wait for cert & install as system CA",
    "Back",
];
const APPS_ITEMS: [&str; 9] = [
    "List installed packages",
    "Toggle system-package filter (com.android.*/com.google.*)",
    "Package info (detailed report)",
    "Launch package",
    "Install APK",
    "Uninstall package",
    "Disable package",
    "Enable package",
    "Back",
];

struct AwaitingInput {
    prompt: &'static str,
    buffer: String,
    action: PendingAction,
}

struct Picker {
    items: Vec<String>,
    labels: Vec<String>,
    state: ListState,
    action: PendingAction,
}

struct App {
    screen: Screen,
    state: ListState,
    log: Vec<String>,
    log_scroll: u16,
    input: Option<AwaitingInput>,
    picker: Option<Picker>,
    job: Option<Receiver<String>>,
    busy: bool,
    spinner: usize,
}

fn wrap_index(current: usize, delta: i32, len: usize) -> usize {
    let len = len as i32;
    let i = current as i32;
    (((i + delta) % len + len) % len) as usize
}

impl App {
    fn items(&self) -> &'static [&'static str] {
        match self.screen {
            Screen::Main => &MAIN_ITEMS,
            Screen::Emulator => &EMULATOR_ITEMS,
            Screen::SslPin => &SSLPIN_ITEMS,
            Screen::Mitm => &MITM_ITEMS,
            Screen::Apps => &APPS_ITEMS,
        }
    }

    fn title(&self) -> &'static str {
        match self.screen {
            Screen::Main => "APK Analysis Toolkit",
            Screen::Emulator => "Emulator",
            Screen::SslPin => "SSL Pinning Bypass",
            Screen::Mitm => "mitmproxy",
            Screen::Apps => "App Manager (adb)",
        }
    }

    fn move_sel(&mut self, delta: i32) {
        let len = self.items().len();
        let i = self.state.selected().unwrap_or(0);
        self.state.select(Some(wrap_index(i, delta, len)));
    }

    fn reset_sel(&mut self) {
        self.state.select(Some(0));
    }

    fn go_back(&mut self) {
        self.screen = Screen::Main;
        self.reset_sel();
    }

    /// Runs `job` on a background thread, streaming its log lines in live.
    /// ponytail: one job at a time (blocked below) — per-screen job queues
    /// are the upgrade if parallel actions are ever needed.
    fn start_job<F>(&mut self, job: F)
    where
        F: FnOnce(Sender<String>) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        self.log.clear();
        self.log_scroll = 0;
        self.job = Some(rx);
        self.busy = true;
        thread::spawn(move || job(tx));
    }

    fn poll_job(&mut self) {
        let Some(rx) = &self.job else { return };
        loop {
            match rx.try_recv() {
                // A leading \r means "replace the last line" — used for
                // live download/install progress bars that update in place.
                Ok(line) => match line.strip_prefix('\r') {
                    Some(text) => match self.log.last_mut() {
                        Some(last) => *last = text.to_string(),
                        None => self.log.push(text.to_string()),
                    },
                    None => self.log.push(line),
                },
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.busy = false;
                    self.job = None;
                    break;
                }
            }
        }
    }

    /// Returns true if the app should quit.
    fn select(&mut self) -> bool {
        if self.busy {
            return false;
        }
        let i = self.state.selected().unwrap_or(0);
        match self.screen {
            Screen::Main => match i {
                0 => self.start_job(|tx| template::run(&tx)),
                1 => {
                    self.screen = Screen::Emulator;
                    self.reset_sel();
                }
                2 => {
                    self.screen = Screen::SslPin;
                    self.reset_sel();
                }
                3 => {
                    self.screen = Screen::Mitm;
                    self.reset_sel();
                }
                4 => {
                    self.screen = Screen::Apps;
                    self.reset_sel();
                }
                5 => self.open_picker(PendingAction::SelectDevice),
                6 => return true,
                _ => {}
            },
            Screen::Emulator => match i {
                0 => self.start_job(|tx| emulator::detect(&tx)),
                1 => self.start_job(|tx| { emulator::create_avd(&tx); }),
                2 => self.start_job(|tx| { emulator::launch_avd(&tx); }),
                3 => self.start_job(|tx| emulator::root_device(&tx)),
                4 => self.start_job(|tx| emulator::deroot_device(&tx)),
                5 => self.go_back(),
                _ => {}
            },
            Screen::SslPin => match i {
                0 => self.start_job(|tx| { ssl_pin::install_tools(&tx); }),
                1 => self.start_job(|tx| { ssl_pin::push_frida_server(&tx); }),
                2 => self.open_picker(PendingAction::SslBypass),
                3 => self.open_picker(PendingAction::StopBypass),
                4 => self.go_back(),
                _ => {}
            },
            Screen::Mitm => match i {
                0 => self.start_job(|tx| { mitm::install(&tx); }),
                1 => self.start_job(|tx| { mitm::start_and_set_proxy(&tx); }),
                2 => self.start_job(|tx| { mitm::install_cert(&tx); }),
                3 => self.go_back(),
                _ => {}
            },
            Screen::Apps => match i {
                0 => self.start_job(|tx| apps::list_packages(&tx)),
                1 => self.log = vec![apps::toggle_system_filter()],
                2 => self.open_picker(PendingAction::PackageInfo),
                3 => self.open_picker(PendingAction::Launch),
                4 => self.ask("APK path: ", PendingAction::InstallApk),
                5 => self.open_picker(PendingAction::Uninstall),
                6 => self.open_picker(PendingAction::Disable),
                7 => self.open_picker(PendingAction::Enable),
                8 => self.go_back(),
                _ => {}
            },
        }
        false
    }

    fn ask(&mut self, prompt: &'static str, action: PendingAction) {
        self.input = Some(AwaitingInput {
            prompt,
            buffer: String::new(),
            action,
        });
    }

    /// Lists installed packages (or active bypass sessions) and lets the user
    /// pick one instead of typing a name. Annotates bypass status so it's
    /// visible which apps already have an active unpinning session running.
    fn open_picker(&mut self, action: PendingAction) {
        let (items, labels): (Vec<String>, Vec<String>) = match action {
            PendingAction::StopBypass => {
                let items = ssl_pin::active_bypasses();
                if items.is_empty() {
                    self.log = vec!["[!] No active bypass sessions tracked by this tool.".into()];
                    return;
                }
                let labels = items.clone();
                (items, labels)
            }
            PendingAction::SelectDevice => {
                let mut items = vec![String::new()];
                let mut labels = vec!["Auto (let the tool pick)".to_string()];
                for (serial, state) in shell::adb_devices() {
                    items.push(serial.clone());
                    labels.push(format!("{serial}  ({state})"));
                }
                (items, labels)
            }
            _ => {
                let items = apps::packages();
                if items.is_empty() {
                    self.log = vec!["[FAIL] No device connected, or no packages found.".into()];
                    return;
                }
                let labels = items
                    .iter()
                    .map(|p| {
                        if action == PendingAction::SslBypass && ssl_pin::is_bypassed(p) {
                            format!("{p}  [bypass active]")
                        } else {
                            p.clone()
                        }
                    })
                    .collect();
                (items, labels)
            }
        };
        let mut state = ListState::default();
        state.select(Some(0));
        self.picker = Some(Picker { items, labels, state, action });
    }
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

fn main() -> anyhow::Result<()> {
    // Only auto-update on a bare launch — never during --check or any other
    // flag-driven diagnostic run, so those stay fast and offline-safe.
    if std::env::args().count() == 1 {
        update::check_and_apply(VERSION);
    }

    if std::env::args().any(|a| a == "--check") {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || emulator::detect(&tx));
        for line in rx.iter() {
            println!("{line}");
        }
        return Ok(());
    }

    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App {
        screen: Screen::Main,
        state: ListState::default(),
        log: vec!["↑/↓ or j/k move · Enter select · Esc/Backspace back · q quit".into()],
        log_scroll: 0,
        input: None,
        picker: None,
        job: None,
        busy: false,
        spinner: 0,
    };
    app.reset_sel();

    let result = run_loop(&mut terminal, &mut app);

    restore_terminal();
    result
}

fn run_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    loop {
        app.poll_job();
        if app.busy {
            app.spinner = (app.spinner + 1) % SPINNER.len();
        }

        terminal.draw(|f| draw(f, app))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if let Some(picker) = &mut app.picker {
            match key.code {
                KeyCode::Esc => app.picker = None,
                KeyCode::Down | KeyCode::Char('j') => {
                    let len = picker.items.len();
                    let i = picker.state.selected().unwrap_or(0);
                    picker.state.select(Some(wrap_index(i, 1, len)));
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let len = picker.items.len();
                    let i = picker.state.selected().unwrap_or(0);
                    picker.state.select(Some(wrap_index(i, -1, len)));
                }
                KeyCode::Enter => {
                    let pkg = picker.items[picker.state.selected().unwrap_or(0)].clone();
                    let action = picker.action;
                    app.picker = None;
                    match action {
                        PendingAction::SslBypass => app.start_job(move |tx| ssl_pin::bypass(&pkg, &tx)),
                        PendingAction::StopBypass => app.start_job(move |tx| ssl_pin::stop_bypass(&pkg, &tx)),
                        PendingAction::Uninstall => app.start_job(move |tx| { apps::uninstall(&pkg, &tx); }),
                        PendingAction::Disable => app.start_job(move |tx| { apps::disable(&pkg, &tx); }),
                        PendingAction::Enable => app.start_job(move |tx| { apps::enable(&pkg, &tx); }),
                        PendingAction::PackageInfo => app.start_job(move |tx| apps::package_info(&pkg, &tx)),
                        PendingAction::Launch => app.start_job(move |tx| apps::launch(&pkg, &tx)),
                        PendingAction::SelectDevice => {
                            if pkg.is_empty() {
                                shell::set_selected_device(None);
                                app.log = vec!["[OK] cleared manual device selection — auto-picking again.".into()];
                            } else {
                                shell::set_selected_device(Some(pkg.clone()));
                                app.log = vec![format!("[OK] now targeting {pkg} for all adb actions.")];
                            }
                        }
                        PendingAction::InstallApk => {}
                    }
                }
                _ => {}
            }
            continue;
        }

        if let Some(input) = &mut app.input {
            match key.code {
                KeyCode::Esc => app.input = None,
                KeyCode::Enter => {
                    let value = input.buffer.clone();
                    let action = input.action;
                    app.input = None;
                    match action {
                        PendingAction::SslBypass => app.start_job(move |tx| ssl_pin::bypass(&value, &tx)),
                        PendingAction::InstallApk => app.start_job(move |tx| apps::install_apk(&value, &tx)),
                        PendingAction::Uninstall => app.start_job(move |tx| apps::uninstall(&value, &tx)),
                        PendingAction::Disable => app.start_job(move |tx| apps::disable(&value, &tx)),
                        PendingAction::Enable => app.start_job(move |tx| apps::enable(&value, &tx)),
                        PendingAction::StopBypass | PendingAction::PackageInfo | PendingAction::Launch | PendingAction::SelectDevice => {} // picker-only
                    }
                }
                KeyCode::Backspace => {
                    input.buffer.pop();
                }
                KeyCode::Char(c) => input.buffer.push(c),
                _ => {}
            }
            continue;
        }

        match key.code {
            KeyCode::Char('q') => return Ok(()),
            KeyCode::Down | KeyCode::Char('j') => app.move_sel(1),
            KeyCode::Up | KeyCode::Char('k') => app.move_sel(-1),
            KeyCode::PageDown => app.log_scroll = app.log_scroll.saturating_add(10),
            KeyCode::PageUp => app.log_scroll = app.log_scroll.saturating_sub(10),
            KeyCode::Char('c') => {
                let report = app.log.join("\n");
                match shell::copy_to_clipboard(&report) {
                    Ok(()) => app.log.push("[OK] copied to clipboard".into()),
                    Err(e) => app.log.push(format!("[FAIL] clipboard: {e}")),
                }
            }
            KeyCode::Esc | KeyCode::Backspace => {
                if app.screen != Screen::Main {
                    app.go_back();
                }
            }
            KeyCode::Enter if app.select() => return Ok(()),
            KeyCode::Enter => {}
            _ => {}
        }
    }
}

fn draw(f: &mut Frame, app: &mut App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1), Constraint::Length(1)])
        .split(f.area());

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[0]);

    if let Some(picker) = &mut app.picker {
        let items: Vec<ListItem> = picker.labels.iter().map(|l| ListItem::new(l.as_str())).collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Select a package"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Green))
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[0], &mut picker.state);
    } else {
        let items: Vec<ListItem> = app.items().iter().map(|i| ListItem::new(*i)).collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(app.title()))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Green))
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[0], &mut app.state);
    }

    let log_title = if app.busy {
        format!("Log {} running... (PgUp/PgDn scroll, c copy)", SPINNER[app.spinner])
    } else {
        "Log (PgUp/PgDn scroll, c copy)".to_string()
    };
    let visible_height = chunks[1].height.saturating_sub(2); // minus top/bottom border
    let max_scroll = (app.log.len() as u16).saturating_sub(visible_height);
    app.log_scroll = app.log_scroll.min(max_scroll);

    let lines: Vec<Line> = app.log.iter().map(|l| style_log_line(l)).collect();
    let log_widget = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title(log_title))
        .wrap(Wrap { trim: false })
        .scroll((app.log_scroll, 0));
    f.render_widget(log_widget, chunks[1]);

    let help = if app.picker.is_some() {
        "↑/↓ or j/k move · Enter choose package · Esc cancel".to_string()
    } else {
        match &app.input {
            Some(input) => format!("{}{}_", input.prompt, input.buffer),
            None if app.busy => "Running… (Enter is disabled until this finishes; ↑/↓ still work)".to_string(),
            None => "↑/↓ or j/k move · Enter select · Esc/Backspace back · q quit".to_string(),
        }
    };
    f.render_widget(Paragraph::new(help).style(Style::default().fg(Color::DarkGray)), outer[1]);

    let footer_text = format!("0xramm    v{VERSION}");
    let footer = Block::default()
        .borders(Borders::TOP)
        .title_top(Line::from(footer_text).right_aligned().style(Style::default().fg(Color::DarkGray)));
    f.render_widget(footer, outer[2]);
}

/// Colors a log line by its status prefix and strips the `==` header markers.
fn style_log_line(line: &str) -> Line<'static> {
    if let Some(inner) = line.strip_prefix("==").and_then(|s| s.strip_suffix("==")) {
        return Line::styled(inner.trim().to_string(), Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan));
    }
    let style = if line.starts_with("[OK]") {
        Style::default().fg(Color::Green)
    } else if line.starts_with("[FAIL]") || line.starts_with("[MISSING]") {
        Style::default().fg(Color::Red)
    } else if line.starts_with("[!]") {
        Style::default().fg(Color::Yellow)
    } else if line.starts_with('>') {
        Style::default().fg(Color::Blue)
    } else if line.starts_with(' ') {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };
    Line::styled(line.to_string(), style)
}
