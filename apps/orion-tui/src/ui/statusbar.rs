use crate::app::AppState;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, state: &AppState, area: Rect, theme: &Theme) {
    let title = format!(
        " {} ORION  │  Model: {}  │  Tokens: {}  │  Cost: ${:.4}  │  MCPs: {} ",
        "✦",
        state.current_model,
        state.token_count,
        state.cost_total,
        state.connected_mcps.len()
    );

    let status = Paragraph::new(Text::raw(title))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        )
        .style(Style::default().fg(theme.text_light).bg(theme.panel));

    f.render_widget(status, area);
}
