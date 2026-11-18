use ::ratatui::buffer::Buffer;
use ::ratatui::layout::Rect;
use ::ratatui::style::{Color, Modifier, Style};
use ::ratatui::widgets::Widget;

use crate::ui::text::{centered_x, width};

use crate::ui::grid::draw_filled_rect;
use libdiskonaut::FileToDelete;
use libdiskonaut::format::truncate_middle;
use libdiskonaut::tiles::FileType;

// The full path if it fits, else the file name, shortened in the middle.
// Names need not be UTF-8: they are shown lossily.
fn truncated_file_name_line(file_to_delete: &FileToDelete, max_len: u16) -> String {
    let full_path = file_to_delete.full_path().to_string_lossy().into_owned();
    let file_name = file_to_delete
        .path_to_file
        .last()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    if max_len > width(&full_path) {
        full_path
    } else {
        truncate_middle(&file_name, max_len)
    }
}

fn render_deletion_prompt(buf: &mut Buffer, message_rect: &Rect, file_to_delete: &FileToDelete) {
    let max_text_len = message_rect.width.saturating_sub(4);
    let file_name_line = truncated_file_name_line(file_to_delete, max_text_len);
    let text_style = Style::default()
        .bg(Color::Black)
        .fg(Color::Red)
        .add_modifier(Modifier::BOLD);
    let question_line = match file_to_delete.file_type {
        FileType::File => {
            if max_text_len >= 17 {
                "Delete this file?".to_string()
            } else if max_text_len >= 3 {
                "Delete?".to_string()
            } else {
                unreachable!("should not render if terminal is so small");
            }
        }
        FileType::Folder => {
            let children = file_to_delete
                .num_descendants
                .expect("folder should have descendants");
            let full_line = format!("Delete folder with {} children?", children);
            let short_line = "Delete folder?".to_string();
            if max_text_len >= width(&full_line) {
                full_line
            } else if max_text_len >= width(&short_line) {
                short_line
            } else {
                unreachable!("should not render if terminal is so small");
            }
        }
    };
    let y_n_line = "(y/n)";
    let question_line_start_position = centered_x(message_rect, &question_line);
    let file_name_line_start_position = centered_x(message_rect, &file_name_line);
    let y_n_line_start_position = centered_x(message_rect, y_n_line);
    buf.set_string(
        question_line_start_position,
        message_rect.y + message_rect.height / 2 - 3,
        question_line,
        text_style,
    );
    buf.set_string(
        file_name_line_start_position,
        message_rect.y + message_rect.height / 2,
        file_name_line,
        text_style,
    );
    buf.set_string(
        y_n_line_start_position,
        message_rect.y + message_rect.height / 2 + 3,
        y_n_line,
        text_style,
    );
}

fn render_deletion_in_progress(
    buf: &mut Buffer,
    message_rect: &Rect,
    file_to_delete: &FileToDelete,
) {
    let max_text_len = message_rect.width.saturating_sub(4);
    let file_name_line = truncated_file_name_line(file_to_delete, max_text_len);
    let deleting_line = "Deleting";
    let text_style = Style::default()
        .bg(Color::Black)
        .fg(Color::Red)
        .add_modifier(Modifier::BOLD);
    let deleting_line_start_position = centered_x(message_rect, deleting_line);
    let file_line_start_position = centered_x(message_rect, &file_name_line);
    buf.set_string(
        deleting_line_start_position,
        message_rect.y + message_rect.height / 2 - 1,
        deleting_line,
        text_style,
    );
    buf.set_string(
        file_line_start_position,
        message_rect.y + message_rect.height / 2 + 1,
        file_name_line,
        text_style,
    );
}

pub struct MessageBox<'a> {
    file_to_delete: &'a FileToDelete,
    deletion_in_progress: bool,
}

impl<'a> MessageBox<'a> {
    pub fn new(file_to_delete: &'a FileToDelete, deletion_in_progress: bool) -> Self {
        Self {
            file_to_delete,
            deletion_in_progress,
        }
    }
}

impl<'a> Widget for MessageBox<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (width, height) = if area.width > 150 {
            (150, 10)
        } else if area.width >= 50 {
            (area.width / 2, 10)
        } else {
            unreachable!("app should not be rendered if window is so small")
        };

        // position self in the middle of the rect
        let x = ((area.x + area.width) / 2) - width / 2;
        let y = ((area.y + area.height) / 2) - height / 2;

        let message_rect = Rect {
            x,
            y,
            width,
            height,
        };
        let fill_style = Style::default()
            .bg(Color::Black)
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD);

        draw_filled_rect(buf, fill_style, &message_rect);
        if self.deletion_in_progress {
            render_deletion_in_progress(buf, &message_rect, self.file_to_delete);
        } else {
            render_deletion_prompt(buf, &message_rect, self.file_to_delete);
        }
    }
}
