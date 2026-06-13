pub mod chat;
pub mod sidebar;
pub mod input;
pub mod statusbar;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;
use crate::app::App;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    let sidebar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(chunks[1]);

    f.render_widget(sidebar::render(&app.state), chunks[0]);

    chat::render(f, &app.state.messages, sidebar_chunks[1]);

    input::render(f, &app.state.input_buffer, chunks[2]);

    if let Some(panel) = &app.state.active_panel {
        statusbar::render(f, panel, chunks[0]);
    }
}
