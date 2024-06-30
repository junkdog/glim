use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Margin, Rect};
use ratatui::prelude::{Line, Text, Widget};
use ratatui::widgets::{Block, Borders, BorderType, Clear};
use crate::theme::theme;
use crate::ui::popup::utility::CenteredShrink;

/// alert.rs popup
#[derive(Clone)]
pub struct AlertPopup {
    message: String,
}

impl AlertPopup {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl Widget for AlertPopup {
    fn render(self, area: Rect, buf: &mut Buffer) where Self: Sized {
        let message_width = self.message.chars().count() as u16;
        let area = area.inner_centered(message_width + 4, 5);
        let text = Text::from(vec![
            Line::from(self.message).style(theme().alert_message),
            Line::from(""),
            Line::from("press any key to close").style(theme().alert_hint),
        ]).alignment(Alignment::Center);

        Clear.render(area, buf);
        Block::default()
            .style(theme().background)
            .border_style(theme().border.alert_border)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .render(area, buf);
        Widget::render(text, area.inner(Margin::new(2, 1)), buf);
    }
}
