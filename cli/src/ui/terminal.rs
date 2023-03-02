use std::{error::Error, io, time::Duration};

use banner_engine::{EventType, Metadata, TaskEvent};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_timer::Delay;
use futures_util::future::FutureExt;
use futures_util::stream::StreamExt;
use tokio::{select, sync::mpsc::Sender};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame, Terminal,
};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};

pub async fn create_terminal_ui(
    tx: Sender<banner_engine::Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // setup terminal
    let backend = {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        CrosstermBackend::new(stdout)
    };
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    // Here is the main loop
    let mut reader = EventStream::new();
    loop {
        let delay = Delay::new(Duration::from_millis(300)).fuse();
        let event = reader.next().fuse();

        select! {
            _ = delay => {
                terminal.draw(|f| {
                    ui(f);
                })?;
            },
            maybe_event = event => {
                            match maybe_event {
                                Some(Ok(event)) => {
                                    log::info!("Event received");
                                    if event == Event::Key(KeyCode::Char('q').into()) {
                                        break;
                                    }

                                    if event == Event::Key(KeyCode::Char('s').into()) {
                                        log::info!("Event received");
                                        TaskEvent::new(EventType::System)
                                            .with_metadata(Metadata::new("banner.io/task".to_string(), "unit-test".to_string()))
                                            .send_from(&tx).await;
                                    }
                                }
                                Some(Err(e)) => log::error!("Error: {:?}\r", e),
                                None => break,
                            };
                            terminal.draw(|f| {
                                ui(f);
                            })?;
                        }
        }
    }

    // restore terminal
    terminal.show_cursor()?;
    terminal.clear()?;
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui<B: Backend>(f: &mut Frame<B>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
        .split(f.size());
    let block = Block::default()
        .title(" Pipeline (test) ")
        .borders(Borders::ALL);
    f.render_widget(block, chunks[0]);

    let logs_events = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    {
        // let lws = TuiWidgetState::new().set_level_for_target("task_log", log::LevelFilter::Debug);
        // let mut tui_lw: TuiLoggerWidget = TuiLoggerWidget::default();
        // let tui_lw = tui_lw.state(&lws);
        let widget = TuiLoggerWidget::default()
            .block(
                Block::default()
                    .title(" Logs ")
                    .border_style(Style::default().fg(Color::White).bg(Color::Black))
                    .borders(Borders::ALL),
            )
            .style_error(Style::default().fg(Color::Red))
            .style_debug(Style::default().fg(Color::Green))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_trace(Style::default().fg(Color::Magenta))
            .style_info(Style::default().fg(Color::Cyan))
            .output_separator('|')
            // .output_timestamp(Some("%F %H:%M:%S%.3f".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(false)
            .output_file(false)
            .output_line(false)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(widget, logs_events[0]);
    }

    // let block = Block::default().title(" Logs ").borders(Borders::ALL);
    // f.render_widget(block, logs_events[0]);
    let block = Block::default().title(" Events ").borders(Borders::ALL);
    f.render_widget(block, logs_events[1]);
}