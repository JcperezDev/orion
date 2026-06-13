use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, panel: &str, area: Rect) {
    let status = Paragraph::new(Text::raw(format!(
        "[{}] | tokens: 0 | cost: $0.00 | model: claude",
        panel
    )))
    .block(Block::default().borders(Borders::ALL))
    .style(Style::default().fg(Color::Blue));

    f.render_widget(status, area);
}
