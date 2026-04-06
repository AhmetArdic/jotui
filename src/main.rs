mod input;
mod layout;
mod render;
mod state;
mod transport;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::io;
use std::sync::mpsc;
use std::time::Duration;

use layout::{parse_layout, resolve_layout};
use render::render_widget;
use state::AppState;
use transport::{connect, load_defaults};

fn parse_port() -> u16 {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--port" {
            if let Some(val) = args.next() {
                return val.parse().expect("--port must be a number");
            }
        }
        if let Some(val) = arg.strip_prefix("--port=") {
            return val.parse().expect("--port must be a number");
        }
    }
    eprintln!("Usage: jotui --port <PORT>");
    std::process::exit(1);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = parse_port();

    // Setup terminal (stdout is the real terminal — no pipes)
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Panic hook for graceful restore
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    // Connect to backend via TCP
    let (tx, rx) = mpsc::channel::<Value>();
    connect(port, tx);

    let defaults = load_defaults();
    let mut app = AppState::new(defaults);
    let mut running = true;

    while running {
        // Process pending JSON-RPC messages from TCP
        while let Ok(msg) = rx.try_recv() {
            let method = msg.get("method").and_then(|v| v.as_str());
            let params = msg
                .get("params")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));

            match method {
                Some("render") => app.handle_render(&params),
                Some("patch") => app.handle_patch(&params),
                Some("navigate") => app.handle_navigate(&params),
                Some(other) => eprintln!("[warn] unknown method: {}", other),
                None => eprintln!("[warn] message missing 'method' field"),
            }
        }

        // Draw
        terminal.draw(|f| {
            if let Some(page) = app.active_page() {
                let layout_tree = parse_layout(&page.layout);
                let mut rects = Vec::new();
                resolve_layout(&layout_tree, f.area(), &mut rects);

                for (ref_id, area) in &rects {
                    if let Some(widget) = page.widgets.get(ref_id) {
                        let focused = page
                            .focus_index
                            .and_then(|i| page.focus_ring.get(i))
                            .map(|fid| fid == ref_id)
                            .unwrap_or(false);
                        render_widget(f, *area, widget, focused);
                    }
                }
            }
        })?;

        // Handle terminal events
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q')
                {
                    running = false;
                } else {
                    input::handle_key(&mut app, key.code);
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
