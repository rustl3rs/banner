use ratatui::{
    buffer::Buffer,
    style::{self, Color, Modifier},
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Status {
    Pending,
    Running,
    Success,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StatusStyle {
    status: Status,
    border_color: Color,
    background_color: Color,
    label_color: Color,
}

impl From<Status> for StatusStyle {
    fn from(status: Status) -> Self {
        match status {
            Status::Pending => Self {
                status,
                border_color: Color::Gray,
                background_color: Color::Reset,
                label_color: Color::Gray,
            },
            Status::Running => Self {
                status,
                border_color: Color::Yellow,
                background_color: Color::Yellow,
                label_color: Color::Yellow,
            },
            Status::Success => Self {
                status,
                border_color: Color::Green,
                background_color: Color::Green,
                label_color: Color::Green,
            },
            Status::Failed => Self {
                status,
                border_color: Color::Red,
                background_color: Color::Red,
                label_color: Color::Red,
            },
        }
    }
}

impl StatusStyle {
    pub fn border_style(&self) -> style::Style {
        style::Style::default()
            .fg(self.border_color)
            .add_modifier(Modifier::BOLD)
    }

    pub fn label_style(&self) -> style::Style {
        style::Style::default()
            .fg(self.label_color)
            .bg(self.background_color)
            .add_modifier(Modifier::BOLD)
    }

    pub fn inner_style(&self) -> style::Style {
        style::Style::default()
            .bg(self.background_color)
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_style(&self) -> style::Style {
        style::Style::default()
            .fg(self.label_color)
            .bg(Color::Reset)
            .add_modifier(Modifier::BOLD)
    }

    pub fn connector_style(&self) -> style::Style {
        style::Style::default()
            .fg(self.border_color)
            .add_modifier(Modifier::BOLD)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub(crate) top_left: (u16, u16),
    pub(crate) status: StatusStyle,
    pub(crate) previous_status: StatusStyle,
    pub(crate) label: String,
}

#[allow(non_snake_case)]
mod Box {
    pub(crate) const TOP_LEFT: &str = "┌";
    pub(crate) const TOP_RIGHT: &str = "┐";
    pub(crate) const BOTTOM_LEFT: &str = "└";
    pub(crate) const BOTTOM_RIGHT: &str = "┘";
    pub(crate) const VERTICAL: &str = "│";
    pub(crate) const HORIZONTAL: &str = "─";
    pub(crate) const SHADOW: &str = "░";
}

#[allow(non_snake_case)]
mod Connector {
    pub(crate) const VERTICAL: &str = "│";
    pub(crate) const HORIZONTAL: &str = "─";
    pub(crate) const LEFT: &str = "├";
    pub(crate) const TOP: &str = "┬";
    pub(crate) const BOTTOM: &str = "┴";
    pub(crate) const CROSS: &str = "┼";
    pub(crate) const LEFT_BOTTOM: &str = "└";
    pub(crate) const RIGHT_BOTTOM: &str = "┘";
    pub(crate) const ARROW_RIGHT: &str = "➤";
}

impl Job {
    pub(crate) fn new(
        top_left: (u16, u16),
        status: Status,
        previous_status: Status,
        label: String,
    ) -> Self {
        Self {
            top_left,
            status: status.into(),
            previous_status: previous_status.into(),
            label,
        }
    }

    pub fn draw(&self, buf: &mut Buffer) {
        let (x, y) = self.top_left;
        let border_style = self.status.border_style();
        let inner_style = self.status.inner_style();
        let text_style = self.status.text_style();

        // draw the box.
        buf.set_string(
            x,
            y,
            format!(
                "{}{}{}{}{}{}",
                Box::TOP_LEFT,
                Box::HORIZONTAL,
                Box::HORIZONTAL,
                Box::HORIZONTAL,
                Box::HORIZONTAL,
                Box::TOP_RIGHT
            ),
            border_style,
        );
        for i in 1..=3 {
            buf.set_string(
                x,
                y + i,
                format!("{}    {}", Box::VERTICAL, Box::VERTICAL),
                border_style,
            );
            buf.set_string(x + 1, y + i, format!("    "), inner_style);
        }
        buf.set_string(
            x,
            y + 4,
            format!(
                "{}{}{}{}{}{}",
                Box::BOTTOM_LEFT,
                Box::HORIZONTAL,
                Box::HORIZONTAL,
                Box::HORIZONTAL,
                Box::HORIZONTAL,
                Box::BOTTOM_RIGHT
            ),
            border_style,
        );

        buf.set_string(x + 2, y + 2, &self.label, text_style);
    }

    pub(crate) fn connect(&self, right: &Job, buf: &mut Buffer) {
        let left = self;
        let (lx, ly) = (left.top_left.0 + 5, left.top_left.1 + 2);
        let (rx, ry) = (right.top_left.0, right.top_left.1 + 2);

        let connector_style = self.status.connector_style();

        match (left, right) {
            (left, right) if left.top_left.1 == right.top_left.1 => {
                for i in lx..rx {
                    let symbol = buf.get(i, ry).symbol.clone();
                    match symbol.as_str() {
                        " " => buf.set_string(i, ly, Connector::HORIZONTAL, connector_style),
                        _ => (),
                    }
                }
                buf.set_string(rx - 1, ry, Connector::ARROW_RIGHT, connector_style);
            }
            (left, right) if left.top_left.1 < right.top_left.1 => {
                let midpoint = (lx + rx) / 2;
                for i in midpoint..rx {
                    buf.set_string(i, ry, Connector::HORIZONTAL, connector_style);
                }
                buf.set_string(rx - 1, ry, Connector::ARROW_RIGHT, connector_style);

                buf.set_string(midpoint, ly, Connector::TOP, connector_style);
                buf.set_string(midpoint, ry, Connector::LEFT_BOTTOM, connector_style);
                for i in (ly + 1)..ry {
                    if buf.get(midpoint, i).symbol == Connector::LEFT_BOTTOM {
                        buf.set_string(midpoint, i, Connector::LEFT, connector_style);
                    } else {
                        buf.set_string(midpoint, i, Connector::VERTICAL, connector_style);
                    }
                }
            }
            (left, right) if left.top_left.1 > right.top_left.1 => {
                let midpoint = (lx + rx) / 2;
                for i in midpoint..rx {
                    let symbol = buf.get(i, ry).symbol.clone();
                    match symbol.as_str() {
                        " " => buf.set_string(i, ry, Connector::HORIZONTAL, connector_style),
                        _ => (),
                    }
                }
                buf.set_string(rx - 1, ry, Connector::ARROW_RIGHT, connector_style);

                let symbol = buf.get(midpoint, ly).symbol.clone();
                match symbol.as_str() {
                    " " => buf.set_string(midpoint, ly, Connector::RIGHT_BOTTOM, connector_style),
                    Connector::LEFT_BOTTOM => {
                        buf.set_string(midpoint, ly, Connector::BOTTOM, connector_style);
                    }
                    _ => (),
                }

                for i in (lx + 1)..midpoint {
                    let symbol = buf.get(midpoint, i).symbol.clone();
                    match symbol.as_str() {
                        " " => buf.set_string(i, ly, Connector::HORIZONTAL, connector_style),
                        _ => (),
                    }
                }

                for i in (ry)..(ly) {
                    let symbol = buf.get(midpoint, i).symbol.clone();
                    match symbol.as_str() {
                        " " => buf.set_string(midpoint, i, Connector::VERTICAL, connector_style),
                        Connector::LEFT_BOTTOM => {
                            buf.set_string(midpoint, i, Connector::LEFT, connector_style);
                        }
                        Connector::BOTTOM => {
                            buf.set_string(midpoint, i, Connector::CROSS, connector_style);
                        }
                        Connector::HORIZONTAL => {
                            buf.set_string(midpoint, i, Connector::TOP, connector_style);
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }
}
