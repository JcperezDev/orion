use crate::app::AppState;
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, List, ListItem};

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
