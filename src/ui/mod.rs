pub mod chat;
pub mod input;
pub mod sidebar;
pub mod statusbar;

use crate::app::App;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App) {
    let total_height = f.size().height;
    let input_height = 3u16;
    let status_height = 1u16;
    let chat_height = total_height.saturating_sub(input_height + status_height);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(status_height),
            Constraint::Length(chat_height),
            Constraint::Length(input_height),
        ])
        .split(f.size());

    let main_area = chunks[1];
    let sidebar_width = 20u16;
    let chat_width = f.size().width.saturating_sub(sidebar_width);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(chat_width),
            Constraint::Length(sidebar_width),
        ])
        .split(main_area);

    let chat_area = Rect::new(main_chunks[0].x, main_chunks[0].y, chat_width, chat_height);
    let sidebar_area = Rect::new(
        main_chunks[0].right(),
        main_chunks[0].y,
        sidebar_width,
        chat_height,
    );

    statusbar::render(f, "ORION ready. Type /help for commands", chunks[0]);

    chat::render(f, &app.state.messages, chat_area);

    sidebar::render(f, &app.state, sidebar_area);

    input::render(f, &app.state.input_buffer, chunks[2]);
}
