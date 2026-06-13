use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, buffer: &str, area: Rect, theme: &Theme) {
    let input_text = if buffer.is_empty() {
        "› ".to_string()
    } else {
        format!("› {}", buffer)
    };

    let input = Paragraph::new(Text::raw(input_text))
        .block(
            Block::default()
                .title(" Input ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.text_dim)),
        )
        .style(Style::default().fg(theme.text_light));

    f.render_widget(input, area);
}
