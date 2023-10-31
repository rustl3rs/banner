use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Widget},
};

use super::{
    job::{Job, Status},
    pipeline_metadata::{IdentifierListItem, PipelineSpecification},
};

// TODO: I need to think about this more. I'm not certain this isn't the right name
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Default)]
pub struct PipelineWidget<'a> {
    block: Option<Block<'a>>,
    pipeline: Option<&'a PipelineSpecification>,
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
    fn render(self, area: Rect, buf: &mut Buffer) {
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
        for job in &pipeline.jobs {
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

// struct Position {
//     x: u16,
//     y: u16,
// }

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
            for job in list {
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
            for job in list {
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
            let status = id.get_status();
            let cj = Job::new((tx, ty), status, Status::Pending, String::new());
            match previous_job {
                IdentifierListItem::Identifier(pid) => {
                    let (x, y) = pid.get_position().unwrap();
                    let tx = x * 10 + 5;
                    let ty = y * 5 + 3;
                    let status = pid.get_status();
                    let pj = Job::new((tx, ty), status, Status::Pending, String::new());
                    pj.connect(&cj, buf);
                }
                IdentifierListItem::SequentialList(list) => {
                    let pj = list.last();
                    render_connector(pj, current_job, buf);
                }
                IdentifierListItem::ParallelList(list) => {
                    for pj in list {
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
            for cj in list {
                render_connector(Some(previous_job), cj, buf);
            }
        }
    }
}
