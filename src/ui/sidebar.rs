use ratatui::widgets::{List, ListItem, Block, Borders};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use crate::app::AppState;

pub fn render(_state: &AppState) -> impl ratatui::widgets::Widget {
    let items = vec![
        ListItem::new(Text::raw("🔧 MCPs")),
        ListItem::new(Text::raw("📁 Memory")),
        ListItem::new(Text::raw("📊 Stats")),
        ListItem::new(Text::raw("⚙️ Config")),
    ];

    List::new(items)
        .block(Block::default().title("Sidebar").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
}
