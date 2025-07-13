use once_cell::sync::Lazy;
use ratatui::style::{Modifier, Style};

use crate::gruvbox::Gruvbox;

pub struct Theme {
    pub project_parents: Style,
    pub project_name: Style,
    pub project_description: Style,
    pub project_commits: [Style; 2], // [0] = count, [1] = "commits"
    pub project_size: [Style; 2],    // [0] = size, [1] = unit
    pub commit_title: Style,
    pub pipeline_source: Style,
    pub pipeline_branch: Style,
    pub pipeline_job: Style,
    pub pipeline_job_failed: Style,
    pub date: Style,
    pub time: Style,
    pub highlight_symbol: Style,
    pub table_border: Style,
    pub table_row_a: Style,
    pub table_row_b: Style,
    pub pipeline_action: Style,
    pub pipeline_action_selected: Style,
    pub background: Style,
    pub border_title: Style,
    pub notification: Style,
    pub notification_project: Style,
    pub input: Style,
    pub input_selected: Style,
    pub input_description: Style,
    pub input_description_em: Style,
    pub input_label: Style,
    pub configuration_error: Style,
    pub border: ThemeBorder,
}

pub struct ThemeBorder {
    pub config_border: Style,
    pub title: Style,
}

impl Theme {
    pub fn new() -> Theme {
        Theme {
            project_parents: Style::default().fg(Gruvbox::Orange.into()),
            project_name: Style::default()
                .fg(Gruvbox::OrangeBright.into())
                .add_modifier(Modifier::BOLD),
            project_description: Style::default()
                .fg(Gruvbox::Light4.into())
                .add_modifier(Modifier::ITALIC),
            project_size: [
                Style::default()
                    .fg(Gruvbox::BlueBright.into())
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Gruvbox::Blue.into()),
            ],
            project_commits: [
                Style::default()
                    .fg(Gruvbox::BlueBright.into())
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Gruvbox::Blue.into()),
            ],
            commit_title: Style::default()
                .fg(Gruvbox::Light4.into())
                .add_modifier(Modifier::ITALIC),
            pipeline_source: Style::default().fg(Gruvbox::BlueBright.into()),
            pipeline_branch: Style::default().fg(Gruvbox::Light2.into()),
            pipeline_job: Style::default().fg(Gruvbox::BlueBright.into()),
            pipeline_job_failed: Style::default().fg(Gruvbox::RedBright.into()),
            pipeline_action: Style::default().fg(Gruvbox::Orange.into()),
            pipeline_action_selected: Style::default()
                .fg(Gruvbox::OrangeBright.into())
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::REVERSED),
            date: Style::default().fg(Gruvbox::Gray244.into()),
            time: Style::default().fg(Gruvbox::Light2.into()),
            table_border: Style::default()
                .fg(Gruvbox::Orange.into())
                .bg(Gruvbox::Dark0.into()),
            table_row_a: Style::default().bg(Gruvbox::Dark0Hard.into()),
            table_row_b: Style::default().bg(Gruvbox::Dark0.into()),
            background: Style::default().bg(Gruvbox::Dark0.into()),
            border_title: Style::default()
                .fg(Gruvbox::Light2.into())
                .add_modifier(Modifier::BOLD),
            highlight_symbol: Style::default()
                .bg(Gruvbox::Dark1.into())
                .add_modifier(Modifier::BOLD),
            notification: Style::default()
                .bg(Gruvbox::Dark0.into())
                .fg(Gruvbox::Orange.into())
                .add_modifier(Modifier::BOLD),
            notification_project: Style::default()
                .fg(Gruvbox::OrangeBright.into())
                .add_modifier(Modifier::BOLD),
            input: Style::default()
                .fg(Gruvbox::Light2.into())
                .bg(Gruvbox::Dark0Hard.into())
                .add_modifier(Modifier::BOLD),
            input_selected: Style::default()
                .fg(Gruvbox::Light0Soft.into())
                .bg(Gruvbox::Dark0Hard.into())
                .add_modifier(Modifier::BOLD),
            input_label: Style::default()
                .fg(Gruvbox::Orange.into())
                .add_modifier(Modifier::BOLD),
            input_description: Style::default()
                .fg(Gruvbox::Gray244.into())
                .add_modifier(Modifier::ITALIC),
            input_description_em: Style::default()
                .fg(Gruvbox::Light4.into())
                .add_modifier(Modifier::ITALIC)
                .add_modifier(Modifier::BOLD),
            configuration_error: Style::default()
                .fg(Gruvbox::YellowBright.into())
                .add_modifier(Modifier::BOLD),
            border: ThemeBorder {
                title: Style::default()
                    .bg(Gruvbox::Orange.into())
                    .fg(Gruvbox::Dark0.into())
                    .add_modifier(Modifier::BOLD),
                config_border: Style::default()
                    .fg(Gruvbox::OrangeBright.into())
                    .bg(Gruvbox::Dark0.into()),
            },
        }
    }

    pub fn table_row(&self, idx: usize) -> Style {
        match idx % 2 {
            0 => self.table_row_a,
            _ => self.table_row_b,
        }
    }
}

static THEME: Lazy<Theme> = Lazy::new(Theme::new);
pub(crate) fn theme() -> &'static Theme {
    &THEME
}
