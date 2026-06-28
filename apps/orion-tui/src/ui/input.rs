use crate::app::AppState;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, state: &AppState, area: Rect, theme: &Theme) {
    let input_text = if state.is_processing {
        " Processing… (Ctrl+C to cancel)".to_string()
    } else if state.input_buffer.is_empty() {
        "› ".to_string()
    } else {
        format!("› {}", state.input_buffer)
    };

    let border_style = if state.is_processing {
        Style::default().fg(theme.warning_amber)
    } else {
        Style::default().fg(theme.border)
    };

    let input = Paragraph::new(Text::raw(input_text))
        .block(
            Block::default()
                .title(" Input ")
                .borders(Borders::ALL)
                .border_style(border_style)
                .title_style(Style::default().fg(theme.text_dim)),
        )
        .style(Style::default().fg(theme.text_light));

    f.render_widget(input, area);
}
