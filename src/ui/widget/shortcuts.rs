use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Offset, Rect},
    prelude::{Line, Modifier, Span, Style, Widget},
    widgets::Clear,
};

use crate::gruvbox::Gruvbox;

/// shortcuts widget
#[derive(Debug, Clone, Default)]
pub struct Shortcuts<'a> {
    values: Vec<(&'a str, &'a str)>,
    shortcut_label_style: Style,
    shortcut_key_style: Style,
    alignment: Alignment,
}

impl Shortcuts<'_> {
    pub fn from<'a>(values: Vec<(&'a str, &'a str)>) -> Shortcuts<'a> {
        Shortcuts {
            values,
            shortcut_label_style: Style::default()
                .fg(Gruvbox::OrangeDim.into())
                .add_modifier(Modifier::BOLD),
            shortcut_key_style: Style::default()
                .fg(Gruvbox::Orange.into())
                .add_modifier(Modifier::BOLD),
            alignment: Alignment::Right,
        }
    }

    pub fn as_line(&self) -> Line<'_> {
        let shortcuts = self
            .values
            .iter()
            .flat_map(|(key, label)| {
                if label.contains(key) {
                    self.spans_from_mnemonic(key, label)
                } else {
                    self.spans_from_shortcut(key, label)
                }
            })
            .collect::<Vec<Span>>();

        Line::from(shortcuts).alignment(self.alignment)
    }

    fn spans_from_shortcut<'a>(&self, key: &'a str, label: &'a str) -> Vec<Span<'a>> {
        vec![
            Span::from(" "),
            Span::from(key).style(self.shortcut_key_style),
            Span::from(" "),
            Span::from(label).style(self.shortcut_label_style),
            Span::from(" "),
        ]
    }

    fn spans_from_mnemonic<'a>(&'a self, key: &'a str, label: &'a str) -> Vec<Span<'a>> {
        let mnemonic = key.chars().next().unwrap();
        let mnemonic_idx = label.find(mnemonic).unwrap();

        let start = Span::from(&label[0..mnemonic_idx]).style(self.shortcut_label_style);
        let m = Span::from(&label[mnemonic_idx..mnemonic_idx + 1]).style(self.shortcut_key_style);
        let end =
            Span::from(&label[mnemonic_idx + 1..label.len()]).style(self.shortcut_label_style);

        vec![Span::from(" "), start, m, end, Span::from(" ")]
    }
}

impl Widget for Shortcuts<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let line = self.as_line();

        // let len = shortcuts.iter().map(|s| s.content.chars().count()).sum::<usize>() as i32;
        let len = line.width() as i32;
        let delta = area.width as i32 - len;
        let area_to_clear = area.offset(Offset { x: delta, y: 0 }).clamp(area);

        Clear.render(area_to_clear, buf);
        line.render(area, buf);
    }
}
