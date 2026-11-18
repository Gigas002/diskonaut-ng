mod app;
mod cli;
mod config;
mod error;
mod input;
mod messages;
mod state;
mod ui;

use ::std::io::{self, IsTerminal};
use ::std::path::PathBuf;
use ::std::process;
use ::std::sync::Arc;
use ::std::sync::atomic::{AtomicBool, Ordering};
use ::std::sync::mpsc;
use ::std::sync::mpsc::{Receiver, SyncSender};
use ::std::thread::park_timeout;
use ::std::{thread, time};
use clap::Parser;
use cli::Opt;
use error::Error;
use libdiskonaut::{ScanItem, ScanOptions, scan_folder};

use ::ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::Event as BackEvent;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use app::{App, UiMode};
use config::DiskonautConfig;
use input::{TerminalEvents, needs_quit_delay};
use messages::{Event, Instruction, handle_events};

fn main() {
    if let Err(err) = try_main() {
        eprintln!("Error: {}", err);
        process::exit(2);
    }
}

/// Leaves raw mode when dropped, however `try_main` ends.
struct RawMode;

impl RawMode {
    fn enable() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// Give the terminal back (cooked mode, a visible cursor) before a panic
/// message is printed: a panic would otherwise leave the shell unusable.
fn restore_terminal_on_panic() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = ratatui::crossterm::execute!(io::stdout(), ratatui::crossterm::cursor::Show);
        default_hook(info);
    }));
}

fn try_main() -> Result<(), Error> {
    let opts = Opt::parse();

    let config_path = opts
        .config
        .clone()
        .or_else(config::default_config_path)
        .unwrap_or_else(|| PathBuf::from("config"));
    let diskonaut_config =
        DiskonautConfig::load(opts.config.as_deref()).map_err(|source| Error::Config {
            path: config_path.clone(),
            source,
        })?;
    let keybinds = diskonaut_config
        .keybinds()
        .map_err(|source| Error::Config {
            path: config_path,
            source,
        })?;
    let scan_options = ScanOptions {
        parallel: true,
        show_apparent_size: opts.apparent_size || diskonaut_config.base.apparent_size,
        one_file_system: opts.one_file_system || diskonaut_config.base.one_file_system,
    };

    // Everything that can fail before the UI runs goes first, while the
    // terminal is still in cooked mode.
    let folder = opts.resolve_folder()?;
    // The UI draws to stdout and reads the cursor position back through
    // the terminal: piped, it waited for a reply that never came and panicked.
    if !io::stdout().is_terminal() {
        return Err(Error::NoStdout);
    }

    restore_terminal_on_panic();
    let _raw_mode = RawMode::enable()?;
    start(
        CrosstermBackend::new(io::stdout()),
        Box::new(TerminalEvents {}),
        folder,
        scan_options,
        keybinds,
    );
    Ok(())
}

fn start<B>(
    terminal_backend: B,
    terminal_events: Box<dyn Iterator<Item = BackEvent> + Send>,
    path: PathBuf,
    scan_options: ScanOptions,
    keybinds: config::Keybinds,
) where
    B: Backend + Send + 'static,
{
    let mut active_threads = vec![];

    let (event_sender, event_receiver): (SyncSender<Event>, Receiver<Event>) =
        mpsc::sync_channel(1);
    let (instruction_sender, instruction_receiver): (
        SyncSender<Instruction>,
        Receiver<Instruction>,
    ) = mpsc::sync_channel(100);

    let running = Arc::new(AtomicBool::new(true));
    let loaded = Arc::new(AtomicBool::new(false));

    // Constructed before any thread reads stdin: `App::new` clears the terminal, which
    // queries the cursor position by writing a CPR request and reading its reply straight off
    // stdin. If `stdin_handler` were already polling stdin for input events, it could steal that
    // reply out from under the query, leaving it blocked until the next real keypress arrived and
    // the screen showing nothing in the meantime.
    let mut app = App::new(
        terminal_backend,
        path.clone(),
        event_sender,
        scan_options.show_apparent_size,
        keybinds.clone(),
    );

    active_threads.push(
        thread::Builder::new()
            .name("event_executer".to_string())
            .spawn({
                let instruction_sender = instruction_sender.clone();
                || handle_events(event_receiver, instruction_sender)
            })
            .unwrap(),
    );

    active_threads.push(
        thread::Builder::new()
            .name("stdin_handler".to_string())
            .spawn({
                let instruction_sender = instruction_sender.clone();
                let running = running.clone();
                let keybinds = keybinds.clone();
                move || {
                    for evt in terminal_events {
                        if let BackEvent::Resize(_x, _y) = evt {
                            let _ = instruction_sender.send(Instruction::ResetUiMode);
                            let _ = instruction_sender.send(Instruction::Render);
                            continue;
                        }

                        let delay =
                            matches!(&evt, BackEvent::Key(_)) && needs_quit_delay(&evt, &keybinds);
                        if instruction_sender.send(Instruction::Keypress(evt)).is_err() {
                            break;
                        }
                        if delay {
                            // not ideal, but works in a pinch
                            park_timeout(time::Duration::from_millis(100));
                            // if we don't wait, the app won't have time to quit
                            if !running.load(Ordering::Acquire) {
                                // sometimes ctrl-c doesn't shut down the app
                                // (eg. dismissing an error message)
                                // in order not to be aware of those particularities
                                // we check "running"
                                break;
                            }
                        }
                    }
                }
            })
            .unwrap(),
    );

    active_threads.push(
        thread::Builder::new()
            .name("hd_scanner".to_string())
            .spawn({
                let path = path.clone();
                let instruction_sender = instruction_sender.clone();
                let loaded = loaded.clone();
                move || {
                    'scanning: for item in scan_folder(&path, scan_options) {
                        let instruction_sent = match item {
                            ScanItem::Entry {
                                metadata: file_metadata,
                                path: entry_path,
                            } => instruction_sender.send(Instruction::AddEntryToBaseFolder((
                                file_metadata,
                                entry_path,
                            ))),
                            ScanItem::ReadError => {
                                instruction_sender.send(Instruction::IncrementFailedToRead)
                            }
                        };
                        if instruction_sent.is_err() {
                            // if we fail to send an instruction here, this likely means the program has
                            // ended and we need to break this loop as well in order not to hang
                            break 'scanning;
                        };
                    }
                    let _ = instruction_sender.send(Instruction::StartUi);
                    loaded.store(true, Ordering::Release);
                }
            })
            .unwrap(),
    );

    active_threads.push(
        thread::Builder::new()
            .name("loading_loop".to_string())
            .spawn({
                let instruction_sender = instruction_sender.clone();
                let running = running.clone();
                move || {
                    while running.load(Ordering::Acquire) && !loaded.load(Ordering::Acquire) {
                        let _ = instruction_sender.send(Instruction::ToggleScanningVisualIndicator);
                        let _ = instruction_sender.send(Instruction::RenderAndUpdateBoard);
                        park_timeout(time::Duration::from_millis(100));
                    }
                }
            })
            .unwrap(),
    );

    app.start(instruction_receiver);
    running.store(false, Ordering::Release);

    for thread_handler in active_threads {
        thread_handler.join().unwrap();
    }
}
