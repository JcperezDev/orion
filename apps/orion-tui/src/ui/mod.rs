pub mod chat;
pub mod input;
pub mod sidebar;
pub mod statusbar;
pub mod theme;

use crate::app::App;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;
use theme::Theme;

pub fn render(f: &mut Frame, app: &App) {
    let theme = Theme::dark();
    let total_height = f.size().height;
    let total_width = f.size().width;

    let title_height = 1u16;
    let input_height = 3u16;
    let main_height = total_height.saturating_sub(title_height + input_height);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_height),
            Constraint::Length(main_height),
            Constraint::Length(input_height),
        ])
        .split(f.size());

    let title_area = chunks[0];
    let main_area = chunks[1];
    let input_area = chunks[2];

    let sidebar_width = 25u16.min(total_width / 4);
    let chat_width = total_width.saturating_sub(sidebar_width);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(chat_width),
            Constraint::Length(sidebar_width),
        ])
        .split(main_area);

    let chat_area = Rect::new(main_chunks[0].x, main_chunks[0].y, chat_width, main_height);
    let sidebar_area = Rect::new(
        main_chunks[1].x,
        main_chunks[1].y,
        sidebar_width,
        main_height,
    );

    statusbar::render(f, &app.state, title_area, &theme);

    chat::render(f, &app.state.messages, chat_area, &theme);

    sidebar::render(f, &app.state, sidebar_area, &theme);

    input::render(f, &app.state.input_buffer, input_area, &theme);
}
