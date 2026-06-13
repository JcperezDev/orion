use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

fn format_time(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%H:%M").to_string()
}

pub fn render(f: &mut Frame, messages: &[Message], area: Rect, theme: &Theme) {
    let items: Vec<ListItem> = messages
        .iter()
        .map(|m| {
            let (prefix, color) = match m.role.as_str() {
                "user" => ("YOU", theme.purple),
                "system" => ("SYSTEM", theme.warning_amber),
                _ => ("ORION", theme.accent_blue),
            };

            let time = format_time(&m.timestamp);
            let line = Line::from(vec![
                Span::styled(prefix, Style::default().fg(color).bold()),
                Span::styled(" ", Style::default().fg(theme.text_dim)),
                Span::styled(time, Style::default().fg(theme.text_dim)),
                Span::styled(" ", Style::default().fg(theme.text_dim)),
                Span::styled(m.content.clone(), Style::default().fg(theme.text_light)),
            ]);

            ListItem::new(Text::from(line))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Chat ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.accent_blue)),
        )
        .style(Style::default().fg(theme.text_light));

    f.render_widget(list, area);
}
