use banner_engine::IdentifierListItem;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Widget},
};

use super::job::{Job, Status};

/// The Canvas widget may be used to draw more detailed figures using braille patterns (each
/// cell can have a braille character in 8 different positions).
/// # Examples
///
/// ```
/// # use ratatui::widgets::{Block, Borders};
/// # use ratatui::layout::Rect;
/// # use ratatui::widgets::canvas::{Canvas, Shape, Line, Rectangle, Map, MapResolution};
/// # use ratatui::style::Color;
/// Canvas::default()
///     .block(Block::default().title("Canvas").borders(Borders::ALL))
///     .x_bounds([-180.0, 180.0])
///     .y_bounds([-90.0, 90.0])
///     .paint(|ctx| {
///         ctx.draw(&Map {
///             resolution: MapResolution::High,
///             color: Color::White
///         });
///         ctx.layer();
///         ctx.draw(&Line {
///             x1: 0.0,
///             y1: 10.0,
///             x2: 10.0,
///             y2: 10.0,
///             color: Color::White,
///         });
///         ctx.draw(&Rectangle {
///             x: 10.0,
///             y: 20.0,
///             width: 10.0,
///             height: 10.0,
///             color: Color::Red
///         });
///     });
/// ```
#[derive(Debug, Clone)]
pub struct PipelineWidget<'a> {
    block: Option<Block<'a>>,
    pipeline: Option<&'a banner_engine::PipelineSpecification>,
}

impl<'a> Default for PipelineWidget<'a> {
    fn default() -> PipelineWidget<'a> {
        PipelineWidget {
            block: None,
            pipeline: None,
        }
    }
}

impl<'a> PipelineWidget<'a> {
    pub fn block(mut self, block: Block<'a>) -> PipelineWidget<'a> {
        self.block = Some(block);
        self
    }

    pub fn pipeline(
        mut self,
        pipeline: &'a banner_engine::PipelineSpecification,
    ) -> PipelineWidget<'a> {
        self.pipeline = Some(pipeline);
        self
    }
}

impl<'a> Widget for PipelineWidget<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        if let Some(block) = self.block {
            block.render(area, buf);
        }
        if let Some(pipeline) = self.pipeline {
            let mut x = 5;
            let y = 5;
            let mut job_number = 1;
            let mut previous_job: Option<&IdentifierListItem> = None;
            for job in pipeline.jobs.iter() {
                log::debug!(target: "task_log", "Rendering job: {:?}", job);
                render_job(job, x, y, job_number, buf);
                if previous_job.is_some() {
                    let previous_xy = (x - 10, y);
                    render_connector(previous_job.unwrap(), previous_xy, job, (x, y), buf);
                }
                x += 10;
                job_number += 1;
                previous_job = Some(job);
            }
        }
    }
}

fn render_job(
    job: &banner_engine::IdentifierListItem,
    x: u16,
    y: u16,
    mut job_number: u16,
    buf: &mut Buffer,
) {
    match job {
        banner_engine::IdentifierListItem::Identifier(id) => {
            log::debug!(target: "task_log", "Rendering job: {:?}", id);
            let job_ui = Job::new(
                (x, y),
                Status::Pending,
                Status::Pending,
                format!("J{job_number}"),
            );
            job_ui.draw(buf);
            job_number += 1;
        }
        banner_engine::IdentifierListItem::SequentialList(list) => {
            for job in list.iter() {
                let x = x + 10;
                render_job(job, x, y, job_number, buf);
            }
        }
        banner_engine::IdentifierListItem::ParallelList(list) => {
            for job in list.iter() {
                let y = y + 5;
                render_job(job, x, y, job_number, buf);
            }
        }
    }
}

fn render_connector(
    previous_job: &IdentifierListItem,
    previous_xy: (u16, u16),
    current_job: &IdentifierListItem,
    current_xy: (u16, u16),
    buf: &mut Buffer,
) {
    match current_job {
        banner_engine::IdentifierListItem::Identifier(id) => {
            // println!("Identifier: {:?}", id);
            log::debug!(target: "task_log", "Rendering job: {:?}", id);
            let cj = Job::new(
                (current_xy.0, current_xy.1),
                Status::Pending,
                Status::Pending,
                String::from(""),
            );
            match previous_job {
                IdentifierListItem::Identifier(_) => {
                    let pj = Job::new(
                        (previous_xy.0, previous_xy.1),
                        Status::Pending,
                        Status::Pending,
                        String::from(""),
                    );
                    pj.connect(&cj, buf);
                }
                IdentifierListItem::SequentialList(list) => {
                    let pj = list.last().unwrap();
                    render_connector(pj, previous_xy, current_job, current_xy, buf);
                }
                IdentifierListItem::ParallelList(list) => {
                    for pj in list.iter() {
                        let pxy = (previous_xy.0, previous_xy.1 + 5);
                        render_connector(pj, pxy, current_job, current_xy, buf);
                    }
                }
            }
        }
        banner_engine::IdentifierListItem::SequentialList(list) => {
            let cj = list.first().unwrap();
            render_connector(previous_job, previous_xy, cj, current_xy, buf);
        }
        banner_engine::IdentifierListItem::ParallelList(list) => {
            for cj in list.iter() {
                let cxy = (current_xy.0, current_xy.1 + 5);
                render_connector(previous_job, previous_xy, cj, cxy, buf);
            }
        }
    }
}
