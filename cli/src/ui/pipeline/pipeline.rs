use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Widget},
};

use super::{
    job::{Job, Status},
    pipeline_metadata::{IdentifierListItem, PipelineSpecification},
};

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
    pipeline: Option<&'a PipelineSpecification>,
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
    pub(crate) fn block(mut self, block: Block<'a>) -> PipelineWidget<'a> {
        self.block = Some(block);
        self
    }

    pub(crate) fn pipeline(mut self, pipeline: &'a PipelineSpecification) -> PipelineWidget<'a> {
        self.pipeline = Some(pipeline);
        self
    }
}

impl<'a> Widget for PipelineWidget<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        if let Some(block) = self.block {
            block.render(area, buf);
        }
        if self.pipeline.is_none() {
            return;
        }

        let pipeline = self.pipeline.unwrap();

        let mut x = 0;
        let y = 0;
        let mut job_number: u16 = 1;
        let mut previous_job: Option<&IdentifierListItem> = None;
        for job in pipeline.jobs.iter() {
            // log::debug!(target: "task_log", "Rendering job: {:?}", job);
            let (nx, _ny) = render_job(previous_job, job, (x, y), &mut job_number, buf);
            render_connector(previous_job, job, buf);
            if nx > x {
                x = nx;
            }
            x += 1;
            previous_job = Some(job);
        }
    }
}

struct Position {
    x: u16,
    y: u16,
}

fn render_job(
    previous_job: Option<&IdentifierListItem>,
    job: &IdentifierListItem,
    current_xy: (u16, u16),
    job_number: &mut u16,
    buf: &mut Buffer,
) -> (u16, u16) {
    match job {
        IdentifierListItem::Identifier(id) => {
            let (x, y) = (current_xy.0, current_xy.1);
            // log::debug!(target: "task_log", "Rendering job: {:?}=J{job_number} / [{x}, {y}]", id);

            let tx = x * 10 + 5;
            let ty = y * 5 + 3;
            let status = id.get_status();
            let job_ui = Job::new((tx, ty), status.clone(), status, format!("J{job_number}"));
            job_ui.draw(buf);
            *job_number += 1;
            id.set_position(x, y);
            render_connector(previous_job, job, buf);
            (x, y)
        }
        IdentifierListItem::SequentialList(list) => {
            // TODO: come back and simplify this
            let mut pj = previous_job;
            let (mut x, y) = (current_xy.0, current_xy.1);
            let (mut lx, mut ly) = (current_xy.0, current_xy.1);
            for job in list.iter() {
                // log::debug!(target: "task_log", "SEQUENTIAL LIST: [{x}, {y}]");
                let (nx, ny) = render_job(pj, job, (x, y), job_number, buf);
                if nx > lx {
                    lx = nx;
                }
                if ny > ly {
                    ly = ny;
                }
                x = lx + 1;
                pj = Some(job);
            }
            (lx, ly)
        }
        IdentifierListItem::ParallelList(list) => {
            // TODO: come back and simplify this
            let (x, mut y) = (current_xy.0, current_xy.1);
            let (mut lx, mut ly) = (current_xy.0, current_xy.1);
            for job in list.iter() {
                // log::debug!(target: "task_log", "PARALLEL LIST: [{x}, {y}]");
                let (nx, ny) = render_job(previous_job, job, (x, y), job_number, buf);
                if nx > lx {
                    lx = nx;
                }
                if ny > ly {
                    ly = ny;
                }
                y = ly + 1;
            }
            (lx, ly)
        }
    }
}

fn render_connector(
    previous_job: Option<&IdentifierListItem>,
    current_job: &IdentifierListItem,
    buf: &mut Buffer,
) {
    if previous_job.is_none() {
        return;
    }

    let previous_job = previous_job.unwrap();

    match current_job {
        IdentifierListItem::Identifier(id) => {
            // println!("Identifier: {:?}", id);
            // log::debug!(target: "task_log", "Rendering job: {:?}", id);
            let (x, y) = id.get_position().unwrap();
            let tx = x * 10 + 5;
            let ty = y * 5 + 3;
            let cj = Job::new((tx, ty), Status::Pending, Status::Pending, String::from(""));
            match previous_job {
                IdentifierListItem::Identifier(pid) => {
                    let (x, y) = pid.get_position().unwrap();
                    let tx = x * 10 + 5;
                    let ty = y * 5 + 3;
                    let pj = Job::new((tx, ty), Status::Pending, Status::Pending, String::from(""));
                    pj.connect(&cj, buf);
                }
                IdentifierListItem::SequentialList(list) => {
                    let pj = list.last();
                    render_connector(pj, current_job, buf);
                }
                IdentifierListItem::ParallelList(list) => {
                    for pj in list.iter() {
                        render_connector(Some(pj), current_job, buf);
                    }
                }
            }
        }
        IdentifierListItem::SequentialList(list) => {
            let cj = list.first().unwrap();
            render_connector(Some(previous_job), cj, buf);
        }
        IdentifierListItem::ParallelList(list) => {
            for cj in list.iter() {
                render_connector(Some(previous_job), cj, buf);
            }
        }
    }
}
