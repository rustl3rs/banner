use std::{error::Error, io, str::FromStr, sync::Arc, time::Duration};

use async_recursion::async_recursion;
use banner_engine::{Engine, EventType, ExecutionStatus, SystemEventScope, SystemEventType};
use crossterm::{
    event::{DisableMouseCapture, Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_timer::Delay;
use futures_util::future::FutureExt;
use futures_util::stream::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    prelude::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame, Terminal,
};
use tokio::runtime::Handle;
use tokio::{select, sync::broadcast::Sender};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget, TuiWidgetState};

use super::{
    pipeline::{IdentifierListItem, PipelineSpecification, PipelineWidget, Status},
    state::{FrameSplits, UiLayout},
};

pub async fn create_terminal_ui(
    engine: &Arc<dyn Engine + Send + Sync>,
    tx: Sender<banner_engine::Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    let mut ui_layout = UiLayout::MultiPanelLayout(FrameSplits {
        log_and_event_frame: 50,
        pipeline_frame: 40,
    });

    // Here is the main loop
    let mut reader = EventStream::new();
    loop {
        let delay = Delay::new(Duration::from_millis(300)).fuse();
        let event = reader.next().fuse();

        select! {
            () = delay => {
                terminal.draw(|f| tokio::task::block_in_place(move || {
                    Handle::current().block_on(async move {
                        ui(f, &ui_layout, engine).await;
                    });
                }))?;
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
                            banner_engine::Event::new_builder(EventType::System(SystemEventType::Trigger(SystemEventScope::Pipeline)))
                                .with_pipeline_name("banner")
                                .send_from(&tx);
                        }

                        if event == Event::Key(KeyCode::Char('h').into()) {
                            log::info!(target: "task_log", "Print EventHandler UI Event received");
                            banner_engine::Event::new_builder(EventType::UserDefined)
                                .with_pipeline_name("banner")
                                .send_from(&tx);
                        }

                        if event == Event::Key(KeyCode::Char('L').into()) {
                            log::info!(target: "task_log", "FullScreen logs event received");
                            banner_engine::Event::new_builder(EventType::UserDefined)
                                .with_pipeline_name("banner")
                                .send_from(&tx);
                            ui_layout.set_full_screen_logs();
                        }
                        if event == Event::Key(KeyCode::Char('m').into()) {
                            log::info!(target: "task_log", "Multi panel screen event received");
                            banner_engine::Event::new_builder(EventType::UserDefined)
                                .with_pipeline_name("banner")
                                .send_from(&tx);
                            ui_layout.set_multi_panel();
                        }

                        if event == Event::Key(KeyCode::Char('E').into()) {
                            log::info!(target: "task_log", "FullScreen Events event received");
                            banner_engine::Event::new_builder(EventType::UserDefined)
                                .with_pipeline_name("banner")
                                .send_from(&tx);
                            ui_layout.set_full_screen_events();
                        }

                        if event == Event::Key(KeyCode::Char('P').into()) {
                                                    log::info!(target: "task_log", "FullScreen Pipeline event received");
                                                    banner_engine::Event::new_builder(EventType::UserDefined)
                                                        .with_pipeline_name("banner")
                                                        .send_from(&tx);
                                                    ui_layout.set_full_screen_pipeline();
                                                }
                    }
                    Some(Err(e)) => log::error!(target: "task_log", "Error: {:?}\r", e),
                    None => break,
                };
                terminal.draw(|f| tokio::task::block_in_place(move || {
                    Handle::current().block_on(async move {
                        ui(f, &ui_layout, engine).await;
                    });
                }))?;
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

async fn set_statuses_on_jobs(
    pipeline_with_metadata: &mut PipelineSpecification,
    engine: &Arc<dyn Engine + Send + Sync>,
) {
    for job in &mut pipeline_with_metadata.jobs {
        set_status_on_job(&pipeline_with_metadata.name, job, engine).await;
    }
}

#[async_recursion]
async fn set_status_on_job(
    pipeline_name: &str,
    job: &mut IdentifierListItem,
    engine: &Arc<dyn Engine + Send + Sync>,
) {
    match job {
        super::pipeline::IdentifierListItem::Identifier(j) => {
            let job_status = engine
                .get_state_for_id(&format!("{}/{}/{}", "", pipeline_name, j.name))
                .await;
            let js = match job_status {
                Some(s) => {
                    let status = ExecutionStatus::from_str(s.as_str()).unwrap();
                    match status {
                        ExecutionStatus::Success => Status::Success,
                        ExecutionStatus::Failed => Status::Failed,
                        ExecutionStatus::Running => Status::Running,
                        _ => Status::Pending,
                    }
                }
                None => Status::Pending,
            };

            j.set_status(js);
        }
        super::pipeline::IdentifierListItem::SequentialList(list) => {
            for job in &mut *list {
                set_status_on_job(pipeline_name, job, engine).await;
            }
        }
        IdentifierListItem::ParallelList(list) => {
            for job in &mut *list {
                set_status_on_job(pipeline_name, job, engine).await;
            }
        }
    }
}

async fn ui(f: &mut Frame<'_>, ui_layout: &UiLayout, engine: &Arc<dyn Engine + Send + Sync>) {
    match ui_layout {
        UiLayout::FullScreenLogs => full_screen_logs(f),
        UiLayout::FullScreenEvents => full_screen_events(f),
        UiLayout::FullScreenPipelines => full_screen_pipeline(f, engine).await,
        UiLayout::MultiPanelLayout(ui_state) => multi_panel_layout(f, ui_state, engine).await,
    }
}

async fn full_screen_pipeline(f: &mut Frame<'_>, engine: &Arc<dyn Engine + Send + Sync>) {
    let mut spec = PipelineSpecification::from(&engine.get_pipeline_specification()[0]);
    set_statuses_on_jobs(&mut spec, engine).await;
    let pipe = PipelineWidget::default().block(
        Block::default()
            .title(" Pipeline (test) - ('q' to quit; 's' to start pipeline) - Pipeline ('m' to return to multi panel view) ")
            .title_alignment(Alignment::Center)
            .borders(Borders::TOP),
    )
    .pipeline(&spec);

    f.render_widget(pipe, f.size());
}

fn full_screen_events(f: &mut Frame) {
    let ews = TuiWidgetState::new()
        .set_level_for_target("event_log", log::LevelFilter::Debug)
        .set_default_display_level(log::LevelFilter::Off);
    let ewidget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .title(" Pipeline (test) - Events ('m' to return to multi panel view) ")
                .title_alignment(Alignment::Center)
                .borders(Borders::TOP | Borders::BOTTOM),
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

    f.render_widget(ewidget, f.size());
}

fn full_screen_logs(f: &mut Frame) {
    let lws = TuiWidgetState::new()
        .set_level_for_target("task_log", log::LevelFilter::Debug)
        .set_default_display_level(log::LevelFilter::Off);
    let lwidget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .title(" Pipeline (test) - Logs ('m' to return to multi panel view) ")
                .title_alignment(Alignment::Center)
                .borders(Borders::TOP | Borders::BOTTOM),
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

    f.render_widget(lwidget, f.size());
}

async fn multi_panel_layout(
    f: &mut Frame<'_>,
    ui_layout: &FrameSplits,
    engine: &Arc<dyn Engine + Send + Sync>,
) {
    let constraints = split_frame(ui_layout.pipeline_frame);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(f.size());

    let mut spec = PipelineSpecification::from(&engine.get_pipeline_specification()[0]);
    set_statuses_on_jobs(&mut spec, engine).await;
    let pipe = PipelineWidget::default().block(
        Block::default()
            .title(" Pipeline (test) - ('q' to quit; 's' to start pipeline) - Pipeline ('m' to return to multi panel view) ")
            .borders(Borders::ALL),
    )
    .pipeline(&spec);
    f.render_widget(pipe, chunks[0]);

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
    vec![
        Constraint::Percentage(split_at.into()),
        Constraint::Percentage((100 - split_at).into()),
    ]
}
