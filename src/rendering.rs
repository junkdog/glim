use ratatui::{
    layout::{Constraint, Layout, Margin},
    prelude::{Direction, Rect},
    Frame,
};
use tachyonfx::Duration;
use tracing::debug;

use crate::{
    dispatcher::Dispatcher,
    effect_registry::EffectRegistry,
    glim_app::GlimApp,
    theme::theme,
    ui::{
        popup::{ConfigPopup, PipelineActionsPopup, ProjectDetailsPopup},
        widget::{Notification, ProjectsTable},
        StatefulWidgets,
    },
};

pub fn render_main_ui(
    f: &mut Frame,
    app: &GlimApp,
    widget_states: &mut StatefulWidgets,
    effects: &mut EffectRegistry,
) {
    effects.update_screen_area(f.area());

    let last_tick = widget_states.last_frame;
    let frame_area = f.area();
    let layout =
        Layout::new(Direction::Horizontal, [Constraint::Percentage(100)]).split(frame_area);

    render_projects_table(f, app, widget_states, layout[0]);
    render_popups(f, widget_states, layout[0], last_tick);
    render_effects(f, effects, last_tick, frame_area);

    // Handle screen capture if requested
    if widget_states.capture_screen_requested {
        handle_screen_capture(f, app, widget_states);
    }
}

fn render_projects_table(
    f: &mut Frame,
    app: &GlimApp,
    widget_states: &mut StatefulWidgets,
    area: Rect,
) {
    let config = app.load_config().unwrap_or_default();
    let effective_filter = widget_states.effective_filter(&config.search_filter);
    let (filtered_projects, filtered_indices) = app.filtered_projects(&effective_filter);
    widget_states.update_filtered_indices(filtered_indices);

    let projects = ProjectsTable::new(
        &filtered_projects,
        widget_states.filter_input_active,
        &widget_states.filter_input_text,
    );
    f.render_stateful_widget(projects, area, &mut widget_states.project_table_state);
}

fn render_popups(
    f: &mut Frame,
    widget_states: &mut StatefulWidgets,
    area: Rect,
    last_tick: Duration,
) {
    if let Some(project_details) = widget_states.project_details.as_mut() {
        let popup = ProjectDetailsPopup::new();
        let popup_area = area.inner(Margin::new(6, 2));
        f.render_stateful_widget(popup, popup_area, project_details);
    }

    if let Some(pipeline_actions) = widget_states.pipeline_actions.as_mut() {
        let popup = PipelineActionsPopup::new();
        f.render_stateful_widget(popup, area, pipeline_actions);
    }

    if let Some(config_popup) = &mut widget_states.config_popup_state {
        render_config_popup(f, config_popup, last_tick, area);
    }

    if let Some(notification) = &mut widget_states.notice {
        f.render_stateful_widget(Notification::new(), area, notification);
    }
}

fn render_effects(
    f: &mut Frame,
    effects: &mut EffectRegistry,
    last_tick: Duration,
    frame_area: Rect,
) {
    effects.process_effects(last_tick, f.buffer_mut(), frame_area);
}

fn render_config_popup(
    f: &mut Frame,
    config_popup: &mut crate::ui::popup::ConfigPopupState,
    _last_tick: Duration,
    layout: Rect,
) {
    let popup = ConfigPopup::new();
    f.render_stateful_widget(popup, layout, config_popup);

    let cursor = config_popup.cursor_position;
    f.buffer_mut()
        .set_style(Rect::new(cursor.x, cursor.y, 1, 1), theme().input_selected);
    f.set_cursor_position(cursor);
}

fn handle_screen_capture(f: &mut Frame, app: &GlimApp, widget_states: &mut StatefulWidgets) {
    widget_states.capture_screen_requested = false;

    debug!("Converting screen buffer to ANSI string using tachyonfx");

    // Use tachyonfx's built-in function to convert buffer to ANSI string
    let ansi_output = tachyonfx::buffer_to_ansi_string(f.buffer_mut(), false);

    // Dispatch event to copy to clipboard since we can't access app's clipboard here
    app.dispatch(crate::event::GlimEvent::ScreenCaptureToClipboard(
        ansi_output,
    ));
}
