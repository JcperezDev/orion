use ratatui::widgets::{List, ListItem};
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui::style::{Color, Style};

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub fn render(f: &mut Frame, messages: &[Message], area: Rect) {
    let items: Vec<ListItem> = messages
        .iter()
        .map(|m| {
            let style = if m.role == "user" {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Green)
            };
            ListItem::new(m.content.clone()).style(style)
        })
        .collect();

    let list = List::new(items).block(ratatui::widgets::Block::default().title("Chat"));
    f.render_widget(list, area);
}
