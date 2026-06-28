use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

#[derive(Debug)]
pub enum EventType {
    Input(InputEvent),
    Tick,
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    Char(char),
    Enter,
    Backspace,
    CtrlC,
    CtrlQ,
    Unknown,
}

pub struct EventLoop;

impl EventLoop {
    pub fn new() -> Self {
        Self
    }

    pub async fn next_event(&self) -> EventType {
        if let Ok(true) = crossterm::event::poll(std::time::Duration::from_millis(50)) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    return self.handle_key_event(key.code, key.modifiers);
                }
            }
        }
        EventType::Tick
    }

    fn handle_key_event(&self, code: KeyCode, modifiers: KeyModifiers) -> EventType {
        if modifiers.contains(KeyModifiers::CONTROL) {
            match code {
                KeyCode::Char('c') => return EventType::Input(InputEvent::CtrlC),
                KeyCode::Char('q') => return EventType::Input(InputEvent::CtrlQ),
                _ => {}
            }
        }

        match code {
            KeyCode::Char(c) => {
                if c.is_control() {
                    EventType::Tick
                } else {
                    EventType::Input(InputEvent::Char(c))
                }
            }
            KeyCode::Enter => EventType::Input(InputEvent::Enter),
            KeyCode::Backspace => EventType::Input(InputEvent::Backspace),
            KeyCode::Tab => EventType::Tick,
            KeyCode::Esc => EventType::Tick,
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => EventType::Tick,
            KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown => EventType::Tick,
            KeyCode::Insert | KeyCode::Delete => EventType::Tick,
            KeyCode::F(_) => EventType::Tick,
            _ => EventType::Tick,
        }
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}
