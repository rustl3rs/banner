use std::{error::Error, io, time::Duration};

use banner_engine::{EventType, SystemEventScope, SystemEventType};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_timer::Delay;
use futures_util::future::FutureExt;
use futures_util::stream::StreamExt;
use tokio::{
    select,
    sync::{mpsc::Sender, oneshot::Receiver},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame, Terminal,
};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget, TuiWidgetState};

pub async fn create_terminal_ui(
    tx: Sender<banner_engine::Event>,
    osrx: Receiver<bool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // wait for the signal to start doing this stuff.
    let _ = osrx.await;

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
                                        log::info!("Start Pipeline UI Event received");
                                        banner_engine::Event::new(EventType::System(SystemEventType::Trigger(SystemEventScope::Pipeline)))
                                            .with_pipeline_name("banner")
                                            .send_from(&tx).await;
                                    }
                                    if event == Event::Key(KeyCode::Char('h').into()) {
                                        log::info!("Print EventHandler UI Event received");
                                        banner_engine::Event::new(EventType::)
                                            .with_pipeline_name("banner")
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
        .constraints([Constraint::Percentage(10), Constraint::Percentage(90)].as_ref())
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
        let lws = TuiWidgetState::new()
            .set_level_for_target("task_log", log::LevelFilter::Debug)
            .set_default_display_level(log::LevelFilter::Off);
        let lwidget = TuiLoggerWidget::default()
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
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(true)
            .output_file(false)
            .output_line(false)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .state(&lws);

        f.render_widget(lwidget, logs_events[0]);
    }

    {
        let ews = TuiWidgetState::new()
            .set_level_for_target("event_log", log::LevelFilter::Debug)
            .set_default_display_level(log::LevelFilter::Off);
        let ewidget = TuiLoggerWidget::default()
            .block(
                Block::default()
                    .title(" Events ")
                    .border_style(Style::default().fg(Color::White).bg(Color::Black))
                    .borders(Borders::ALL),
            )
            .style_error(Style::default().fg(Color::Red))
            .style_debug(Style::default().fg(Color::Green))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_trace(Style::default().fg(Color::Magenta))
            .style_info(Style::default().fg(Color::Cyan))
            .output_separator('|')
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(true)
            .output_file(false)
            .output_line(false)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .state(&ews);

        f.render_widget(ewidget, logs_events[1]);
    }
}
