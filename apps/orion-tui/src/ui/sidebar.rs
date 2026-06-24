use crate::app::AppState;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, state: &AppState, area: Rect, theme: &Theme) {
    let nav_items = vec![
        ("Chat", ">", theme.accent_blue, true),
        ("Memory", "*", theme.purple, false),
        ("Agents", "@", theme.success_green, false),
        ("MCP Hub", "#", theme.warning_amber, false),
        ("History", "~", theme.text_dim, false),
    ];

    let items: Vec<ListItem> = nav_items
        .iter()
        .map(|(label, icon, color, active)| {
            let prefix = if *active { ">" } else { " " };
            let text = format!("{}{} {}", prefix, icon, label);
            let style = if *active {
                Style::default().fg(*color).bg(Color::Rgb(17, 26, 46))
            } else {
                Style::default().fg(theme.text_muted)
            };
            ListItem::new(Text::raw(text)).style(style)
        })
        .collect();

    let nav_list = List::new(items)
        .block(
            Block::default()
                .title(" Navigation ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.text_dim)),
        )
        .style(Style::default().fg(theme.text_light));

    let nav_height = 7u16;
    let nav_area = Rect::new(area.x, area.y, area.width, nav_height.min(area.height));
    f.render_widget(nav_list, nav_area);

    let mcp_section_y = area.y + nav_height + 1;
    if mcp_section_y < area.bottom().saturating_sub(10) {
        let mcp_title = Paragraph::new(Span::styled(" MCPs ", Style::default().fg(theme.text_dim)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border)),
            );
        let mcp_title_area = Rect::new(area.x, mcp_section_y, area.width, 1);
        f.render_widget(mcp_title, mcp_title_area);

        let mcp_y = mcp_section_y + 1;
        let mcp_count = state.connected_mcps.len().max(1);
        let mcp_items: Vec<ListItem> = if state.connected_mcps.is_empty() {
            vec![ListItem::new(Text::raw(" ● filesystem (ok)"))
                .style(Style::default().fg(theme.text_muted))]
        } else {
            state
                .connected_mcps
                .iter()
                .take(5)
                .map(|mcp| {
                    ListItem::new(Text::raw(format!(" ● {} (ok)", mcp)))
                        .style(Style::default().fg(theme.success_green))
                })
                .collect()
        };

        let mcp_list = List::new(mcp_items).style(Style::default().fg(theme.text_muted));
        let mcp_area = Rect::new(area.x, mcp_y, area.width, mcp_count as u16 + 1);
        f.render_widget(mcp_list, mcp_area);
    }

    let info_y = area.bottom().saturating_sub(6);
    let info_text = Text::from(vec![
        Line::from(vec![Span::styled(
            " tokens ",
            Style::default().fg(theme.text_dim),
        )]),
        Line::from(vec![Span::styled(
            format!(" {} ", state.token_count),
            Style::default().fg(theme.accent_blue),
        )]),
        Line::from(vec![Span::styled(
            " cost ",
            Style::default().fg(theme.text_dim),
        )]),
        Line::from(vec![Span::styled(
            format!(" ${:.4} ", state.cost_total),
            Style::default().fg(theme.success_green),
        )]),
    ]);

    let info = Paragraph::new(info_text)
        .block(
            Block::default()
                .title(" Stats ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.text_dim)),
        )
        .style(Style::default().fg(theme.text_light));

    let info_area = Rect::new(area.x, info_y, area.width, 6);
    f.render_widget(info, info_area);
}
