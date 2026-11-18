use ::ratatui::buffer::Buffer;
use ::ratatui::layout::Rect;
use ::ratatui::style::{Color, Modifier, Style};
use ::ratatui::widgets::Widget;
use ::std::path::{Path, PathBuf};

use libdiskonaut::format::{DisplaySize, truncate_middle};

use crate::ui::KeyHints;
use crate::ui::text::width;
use libdiskonaut::tiles::{FileType, Tile};

fn render_currently_selected(buf: &mut Buffer, currently_selected: &Tile, max_len: u16, y: u16) {
    let file_name = currently_selected.name.to_string_lossy();
    let size = DisplaySize(currently_selected.size as f64);
    let descendants = currently_selected.descendants;
    let (style, lines) = match currently_selected.file_type {
        FileType::File => (
            Style::default().add_modifier(Modifier::BOLD),
            vec![
                format!("SELECTED: {} ({})", file_name, size),
                format!("SELECTED: {}", file_name),
                format!("{}", file_name),
            ],
        ),
        FileType::Folder => (
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            vec![
                format!(
                    "SELECTED: {} ({}, {} files)",
                    file_name,
                    size,
                    descendants.expect("a folder should have descendants")
                ),
                format!("SELECTED: {} ({})", file_name, size),
                format!("SELECTED: {}", file_name),
                format!("{}", file_name),
            ],
        ),
    };
    // The name alone may not fit either: then it is shortened in the middle.
    let line = lines
        .into_iter()
        .find(|line| width(line) < max_len)
        .unwrap_or_else(|| truncate_middle(&file_name, max_len.saturating_sub(1)));
    buf.set_string(1, y, line, style);
}

fn render_last_read_path(buf: &mut Buffer, last_read_path: &Path, max_len: u16, y: u16) {
    let last_read_path = last_read_path.to_string_lossy();
    if width(&last_read_path) < max_len {
        buf.set_string(1, y, last_read_path, Style::default());
    } else {
        buf.set_string(
            1,
            y,
            truncate_middle(&last_read_path, max_len),
            Style::default(),
        );
    }
}

fn render_controls_legend(
    buf: &mut Buffer,
    hints: &KeyHints,
    hide_delete: bool,
    max_len: u16,
    y: u16,
) {
    let KeyHints {
        enter,
        parent,
        delete,
        zoom,
        quit,
        ..
    } = hints;
    let (long_controls_line, short_controls_line) = if hide_delete {
        (
            format!(
                "<arrows> - move around, <{enter}> - enter folder, <{parent}> - parent folder, <{zoom}> - zoom in/out/reset, <{quit}> - quit"
            ),
            format!("←↓↑→/<{enter}>/<{parent}>: navigate"),
        )
    } else {
        (
            format!(
                "<arrows> - move around, <{enter}> - enter folder, <{parent}> - parent folder, <{delete}> - delete, <{zoom}> - zoom in/out/reset, <{quit}> - quit"
            ),
            format!("←↓↑→/<{enter}>/<{parent}>: navigate, <{delete}>: del"),
        )
    };
    let line = if max_len >= width(&long_controls_line) {
        long_controls_line
    } else if max_len >= width(&short_controls_line) {
        short_controls_line
    } else {
        "(...)".to_string()
    };
    buf.set_string(1, y, line, Style::default().add_modifier(Modifier::BOLD));
}

fn render_small_files_legend(buf: &mut Buffer, x: u16, y: u16, small_files_legend: &str) {
    buf.set_string(
        x,
        y,
        small_files_legend,
        Style::default()
            .fg(Color::Reset)
            .bg(Color::Reset)
            .remove_modifier(Modifier::all()),
    );
    let small_files_legend_character = &mut buf[(x + 1, y)];
    small_files_legend_character.set_style(Style::default().bg(Color::White).fg(Color::Black));
}

pub struct BottomLine<'a> {
    hints: &'a KeyHints,
    hide_delete: bool,
    hide_small_files_legend: bool,
    currently_selected: Option<&'a Tile>,
    last_read_path: Option<&'a PathBuf>,
}

impl<'a> BottomLine<'a> {
    pub fn new(hints: &'a KeyHints) -> Self {
        Self {
            hints,
            hide_delete: false,
            hide_small_files_legend: false,
            currently_selected: None,
            last_read_path: None,
        }
    }
    pub fn hide_delete(mut self) -> Self {
        self.hide_delete = true;
        self
    }
    pub fn hide_small_files_legend(mut self, should_hide_small_files_legend: bool) -> Self {
        self.hide_small_files_legend = should_hide_small_files_legend;
        self
    }
    pub fn currently_selected(mut self, currently_selected: Option<&'a Tile>) -> Self {
        self.currently_selected = currently_selected;
        self
    }
    pub fn last_read_path(mut self, last_read_path: Option<&'a PathBuf>) -> Self {
        self.last_read_path = last_read_path;
        self
    }
}

impl<'a> Widget for BottomLine<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let small_files_legend = "(x = Small files)";
        let small_files_len = if self.hide_small_files_legend {
            0
        } else {
            small_files_legend.chars().count() as u16
        };
        let max_status_len = area.width - small_files_len - 1;
        let max_controls_len = area.width - 1;
        let status_line_y = area.y + area.height - 2;
        let controls_line_y = status_line_y + 1;
        if let Some(currently_selected) = self.currently_selected {
            render_currently_selected(buf, currently_selected, max_status_len, status_line_y);
        } else if let Some(last_read_path) = self.last_read_path {
            render_last_read_path(buf, last_read_path, max_status_len, status_line_y);
        }

        if !self.hide_small_files_legend {
            render_small_files_legend(
                buf,
                area.width - small_files_len - 1,
                status_line_y,
                small_files_legend,
            );
        }

        render_controls_legend(
            buf,
            self.hints,
            self.hide_delete,
            max_controls_len,
            controls_line_y,
        );
    }
}
