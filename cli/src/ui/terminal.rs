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
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame, Terminal,
};
use tokio::{
    select,
    sync::{mpsc::Sender, oneshot::Receiver},
};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget, TuiWidgetState};

use super::state::{UiLayout, UiState};

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
        execute!(stdout, EnterAlternateScreen)?;
        CrosstermBackend::new(stdout)
    };
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let mut ui_layout = UiLayout::MultiPanelLayout(UiState {
        log_and_event_frame: 50,
        pipeline_frame: 40,
    });

    // Here is the main loop
    let mut reader = EventStream::new();
    loop {
        let delay = Delay::new(Duration::from_millis(300)).fuse();
        let event = reader.next().fuse();

        select! {
            _ = delay => {
                terminal.draw(|f| {
                    ui(f, &ui_layout);
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
                            log::info!(target: "task_log", "Start Pipeline UI Event received");
                            banner_engine::Event::new(EventType::System(SystemEventType::Trigger(SystemEventScope::Pipeline)))
                                .with_pipeline_name("banner")
                                .send_from(&tx).await;
                        }

                        if event == Event::Key(KeyCode::Char('h').into()) {
                            log::info!(target: "task_log", "Print EventHandler UI Event received");
                            banner_engine::Event::new(EventType::UserDefined)
                                .with_pipeline_name("banner")
                                .send_from(&tx).await;
                        }

                        if event == Event::Key(KeyCode::Char('L').into()) {
                            log::info!(target: "task_log", "FullScreen logs event received");
                            banner_engine::Event::new(EventType::UserDefined)
                                .with_pipeline_name("banner")
                                .send_from(&tx).await;
                            ui_layout.set_full_screen_logs();
                        }
                        if event == Event::Key(KeyCode::Char('m').into()) {
                            log::info!(target: "task_log", "Multi panel screen event received");
                            banner_engine::Event::new(EventType::UserDefined)
                                .with_pipeline_name("banner")
                                .send_from(&tx).await;
                            ui_layout.set_multi_panel();
                        }

                        if event == Event::Key(KeyCode::Char('E').into()) {
                            log::info!(target: "task_log", "FullScreen Events event received");
                            banner_engine::Event::new(EventType::UserDefined)
                                .with_pipeline_name("banner")
                                .send_from(&tx).await;
                            ui_layout.set_full_screen_events();
                        }

                        if event == Event::Key(KeyCode::Char('P').into()) {
                                                    log::info!(target: "task_log", "FullScreen Pipeline event received");
                                                    banner_engine::Event::new(EventType::UserDefined)
                                                        .with_pipeline_name("banner")
                                                        .send_from(&tx).await;
                                                    ui_layout.set_full_screen_pipeline();
                                                }
                    }
                    Some(Err(e)) => log::error!(target: "task_log", "Error: {:?}\r", e),
                    None => break,
                };
                terminal.draw(|f| {
                    ui(f, &ui_layout);
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

fn ui<B: Backend>(f: &mut Frame<B>, ui_layout: &UiLayout) {
    match ui_layout {
        UiLayout::FullScreenLogs => full_screen_logs(f),
        UiLayout::FullScreenEvents => full_screen_events(f),
        UiLayout::FullScreenPipelines => full_screen_pipeline(f),
        UiLayout::MultiPanelLayout(ui_state) => multi_panel_layout(f, ui_state),
    }
}

fn full_screen_pipeline<B: Backend>(f: &mut Frame<B>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());

    let block = Block::default()
        .title(" Pipeline (test) - ('q' to quit; 's' to start pipeline) - Pipeline ('m' to return to multi panel view) ")
        .borders(Borders::ALL);
    f.render_widget(block, chunks[0]);
}

fn full_screen_events<B: Backend>(f: &mut Frame<B>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());

    let ews = TuiWidgetState::new()
        .set_level_for_target("event_log", log::LevelFilter::Debug)
        .set_default_display_level(log::LevelFilter::Off);
    let ewidget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .title(" Pipeline (test) - Events ('m' to return to multi panel view) ")
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

    f.render_widget(ewidget, chunks[0]);
}

fn full_screen_logs<B: Backend>(f: &mut Frame<B>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());

    let lws = TuiWidgetState::new()
        .set_level_for_target("task_log", log::LevelFilter::Debug)
        .set_default_display_level(log::LevelFilter::Off);
    let lwidget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .title(" Pipeline (test) - Logs ('m' to return to multi panel view) ")
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

    f.render_widget(lwidget, chunks[0]);
}

fn multi_panel_layout<B: Backend>(f: &mut Frame<B>, ui_layout: &UiState) {
    let constraints = split_frame(ui_layout.pipeline_frame);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(f.size());
    let block = Block::default()
        .title(" Pipeline (test) - ('q' to quit; 's' to start pipeline) ")
        .borders(Borders::ALL);
    f.render_widget(block, chunks[0]);

    let constraints = split_frame(ui_layout.log_and_event_frame);
    let logs_events = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints(constraints)
        .split(chunks[1]);

    {
        let lws = TuiWidgetState::new()
            .set_level_for_target("task_log", log::LevelFilter::Debug)
            .set_default_display_level(log::LevelFilter::Off);
        let lwidget = TuiLoggerWidget::default()
            .block(
                Block::default()
                    .title(" Logs - ('L' to go full screen) ")
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
                    .title(" Events - ('E' to go full screen)")
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

pub fn split_frame(split_at: u8) -> Vec<Constraint> {
    let mut constraints = Vec::new();
    constraints.push(Constraint::Percentage(split_at.into()));
    constraints.push(Constraint::Percentage((100 - split_at).into()));
    constraints
}
