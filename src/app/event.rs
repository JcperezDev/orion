use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

#[derive(Debug)]
pub enum EventType {
    Input(String),
    Tick,
    Quit,
}

pub struct EventLoop {
    #[allow(dead_code)]
    pub tick_rate: std::time::Duration,
}

impl EventLoop {
    pub fn new() -> Self {
        Self {
            tick_rate: std::time::Duration::from_millis(100),
        }
    }

    pub async fn next_event(&self) -> EventType {
        if let Ok(true) = crossterm::event::poll(std::time::Duration::from_millis(50)) {
            if let Event::Key(key) = event::read().unwrap() {
                if key.kind == KeyEventKind::Press {
                    return self.handle_key_event(key.code, key.modifiers);
                }
            }
        }
        EventType::Tick
    }

    fn handle_key_event(&self, code: KeyCode, modifiers: KeyModifiers) -> EventType {
        if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
            return EventType::Quit;
        }

        match code {
            KeyCode::Char(c) => EventType::Input(c.to_string()),
            KeyCode::Enter => EventType::Input("\n".to_string()),
            KeyCode::Backspace => EventType::Input("\x08".to_string()),
            KeyCode::Left => EventType::Input("\x1b[D".to_string()),
            KeyCode::Right => EventType::Input("\x1b[C".to_string()),
            KeyCode::Up => EventType::Input("\x1b[A".to_string()),
            KeyCode::Down => EventType::Input("\x1b[B".to_string()),
            KeyCode::Tab => EventType::Input("\t".to_string()),
            KeyCode::Esc => EventType::Input("\x1b".to_string()),
            _ => EventType::Tick,
        }
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}
