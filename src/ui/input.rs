use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui::style::{Color, Style};
use ratatui::text::Text;

pub fn render(f: &mut Frame, buffer: &str, area: Rect) {
    let input = Paragraph::new(Text::raw(format!("> {}", buffer)))
        .block(Block::default().title("Input").borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(input, area);
}
