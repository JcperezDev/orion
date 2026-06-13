use ratatui::style::Color;

pub struct Theme {
    #[allow(dead_code)]
    pub bg: Color,
    pub panel: Color,
    pub border: Color,
    pub accent_blue: Color,
    pub success_green: Color,
    pub warning_amber: Color,
    #[allow(dead_code)]
    pub danger_red: Color,
    pub purple: Color,
    pub text_light: Color,
    pub text_muted: Color,
    pub text_dim: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg: Color::Rgb(8, 10, 15),
            panel: Color::Rgb(10, 12, 18),
            border: Color::Rgb(26, 31, 46),
            accent_blue: Color::Rgb(77, 157, 224),
            success_green: Color::Rgb(39, 201, 63),
            warning_amber: Color::Rgb(243, 156, 18),
            danger_red: Color::Rgb(231, 76, 60),
            purple: Color::Rgb(155, 89, 182),
            text_light: Color::Rgb(201, 209, 217),
            text_muted: Color::Rgb(90, 100, 120),
            text_dim: Color::Rgb(61, 69, 87),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
