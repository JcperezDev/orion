use crate::app::AppState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, state: &AppState, area: Rect) {
    let items = vec![
        ListItem::new(Text::raw("🔧 MCPs")),
        ListItem::new(Text::raw("📁 Memory")),
        ListItem::new(Text::raw("📊 Stats")),
        ListItem::new(Text::raw("⚙️ Config")),
    ];

    let list = List::new(items)
        .block(Block::default().title("Sidebar").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);

    let info = Paragraph::new(Text::raw(format!(
        "Model: {}\nTokens: {}\nCost: ${:.4}",
        state.current_model, state.token_count, state.cost_total
    )))
    .block(Block::default().title("Info").borders(Borders::ALL))
    .style(Style::default().fg(Color::Yellow));

    let info_height = 4u16;
    let info_area = Rect::new(
        area.x,
        area.bottom().saturating_sub(info_height),
        area.width,
        info_height,
    );
    f.render_widget(info, info_area);
}
