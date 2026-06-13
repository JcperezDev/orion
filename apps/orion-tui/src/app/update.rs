use crate::app::App;
use crate::ui::chat::Message;

impl App {
    pub fn update(&mut self, msg: Message) {
        self.state.messages.push(msg);
    }

    pub fn transition(&mut self, event: &str) {
        match event {
            "scroll_up" => {
                if self.state.scroll_offset > 0 {
                    self.state.scroll_offset -= 1;
                }
            }
            "scroll_down" => {
                self.state.scroll_offset += 1;
            }
            _ => {}
        }
    }
}
