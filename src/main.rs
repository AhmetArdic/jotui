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
use std::io::{self, BufRead};
use std::sync::mpsc;
use std::time::Duration;

use layout::{parse_layout, resolve_layout};
use render::render_widget;
use state::AppState;
use transport::{init_event_output, load_defaults};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_event_output();

    let (tx, rx) = mpsc::channel::<Value>();

    // Stdin reader thread
    std::thread::spawn(move || {
        let stdin = io::stdin();
        let reader = stdin.lock();
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let l = l.trim().to_string();
                    if l.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<Value>(&l) {
                        Ok(val) => {
                            if tx.send(val).is_err() {
                                break;
                            }
                        }
                        Err(e) => eprintln!("[warn] invalid JSON: {}", e),
                    }
                }
                Err(e) => {
                    eprintln!("[error] stdin read error: {}", e);
                    break;
                }
            }
        }
    });

    // Setup terminal
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    // Panic hook for graceful restore
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stderr(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let defaults = load_defaults();
    let mut app = AppState::new(defaults);
    let mut running = true;

    while running {
        // Process pending JSON messages
        while let Ok(msg) = rx.try_recv() {
            match msg.get("msg").and_then(|v| v.as_str()) {
                Some("render") => app.handle_render(&msg),
                Some("patch") => app.handle_patch(&msg),
                Some("navigate") => app.handle_navigate(&msg),
                Some(other) => eprintln!("[warn] unknown msg type: {}", other),
                None => eprintln!("[warn] message missing 'msg' field"),
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
