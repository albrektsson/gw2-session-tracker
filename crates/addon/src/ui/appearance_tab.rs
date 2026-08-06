use nexus::imgui::{ColorEdit, Slider, TreeNodeFlags, Ui};
use session_tracker_core::config::RowField;

use crate::app_handle::AppHandle;

fn row_field_label(field: RowField) -> &'static str {
    match field {
        RowField::Icon => "Icon",
        RowField::Name => "Name",
        RowField::Session => "Session",
        RowField::Lifetime => "Lifetime",
        RowField::Rate => "Rate",
    }
}

pub fn render_appearance_tab(ui: &Ui, app: &AppHandle) {
    if ui.collapsing_header("Text & Color", TreeNodeFlags::DEFAULT_OPEN) {
        ui.indent();
        render_text_and_color_section(ui, app);
        ui.unindent();
    }

    if ui.collapsing_header("Row Format", TreeNodeFlags::DEFAULT_OPEN) {
        ui.indent();
        render_row_format_section(ui, app);
        ui.unindent();
    }

    if ui.collapsing_header("Window Sizing", TreeNodeFlags::DEFAULT_OPEN) {
        ui.indent();
        render_window_sizing_section(ui, app);
        ui.unindent();
    }
}

fn render_text_and_color_section(ui: &Ui, app: &AppHandle) {
    ui.text("Main window text size:");
    let mut text_scale = app.lock().config.text_scale;
    if Slider::new("##text_scale", 0.5f32, 3.0f32).build(ui, &mut text_scale) {
        app.mutate_and_persist(|state| state.config.text_scale = text_scale);
    }

    let mut bold_text = app.lock().config.bold_text;
    if ui.checkbox("Bold text", &mut bold_text) {
        app.mutate_and_persist(|state| state.config.bold_text = bold_text);
    }

    let mut label_color = app.lock().config.label_color;
    if ColorEdit::new("Label color (Name field)", &mut label_color).build(ui) {
        app.mutate_and_persist(|state| state.config.label_color = label_color);
    }

    let mut value_color = app.lock().config.value_color;
    if ColorEdit::new("Value color", &mut value_color).build(ui) {
        app.mutate_and_persist(|state| state.config.value_color = value_color);
    }

    let mut icon_color = app.lock().config.icon_color;
    if ColorEdit::new("Icon color", &mut icon_color).build(ui) {
        app.mutate_and_persist(|state| state.config.icon_color = icon_color);
    }

    let mut background_color = app.lock().config.background_color;
    if ColorEdit::new("Background color", &mut background_color).build(ui) {
        app.mutate_and_persist(|state| state.config.background_color = background_color);
    }

    ui.text("Main window background opacity:");
    let mut opacity = app.lock().config.background_opacity;
    if Slider::new("##background_opacity", 0.0f32, 1.0f32).build(ui, &mut opacity) {
        app.mutate_and_persist(|state| state.config.background_opacity = opacity);
    }
}

fn render_row_format_section(ui: &Ui, app: &AppHandle) {
    let wrap_token = ui.push_text_wrap_pos();
    ui.text(
        "Fields shown per row, in order. Fields that don't apply to a given stat (e.g. \
        Lifetime for Session Timer) are skipped automatically.",
    );
    wrap_token.pop(ui);

    let fields = app.lock().config.row_fields.clone();
    let separator_visible = app.lock().config.row_separator_visible.clone();
    for (i, field) in fields.iter().enumerate() {
        if ui.small_button(format!("Up##row_field_up_{i}")) && i > 0 {
            app.mutate_and_persist(|state| state.config.row_fields.swap(i, i - 1));
        }
        ui.same_line();
        if ui.small_button(format!("Down##row_field_down_{i}")) && i + 1 < fields.len() {
            app.mutate_and_persist(|state| state.config.row_fields.swap(i, i + 1));
        }
        ui.same_line();
        ui.text(row_field_label(*field));
        ui.same_line();
        if ui.small_button(format!("Remove##row_field_remove_{i}")) {
            app.mutate_and_persist(move |state| state.config.remove_row_field(i));
        }

        if let Some(visible) = separator_visible.get(i) {
            let mut visible = *visible;
            ui.indent();
            if ui.checkbox(format!("Separator after {}##row_separator_visible_{i}", row_field_label(*field)), &mut visible) {
                app.mutate_and_persist(move |state| {
                    if let Some(v) = state.config.row_separator_visible.get_mut(i) {
                        *v = visible;
                    }
                });
            }
            ui.unindent();
        }
    }

    ui.text("Add field:");
    for field in [RowField::Icon, RowField::Name, RowField::Session, RowField::Lifetime, RowField::Rate] {
        if !fields.contains(&field) {
            if ui.small_button(format!("+ {}##row_field_add", row_field_label(field))) {
                app.mutate_and_persist(move |state| state.config.push_row_field(field));
            }
            ui.same_line();
        }
    }
    ui.new_line();

    ui.text("Separator:");
    let mut separator = app.lock().config.row_separator.clone();
    ui.set_next_item_width(80.0);
    if ui.input_text("##row_separator", &mut separator).build() {
        app.mutate_and_persist(|state| state.config.row_separator = separator.clone());
    }
    for preset in ["|", "/", "-", " "] {
        ui.same_line();
        let label = if preset == " " { "(space)".to_string() } else { format!("{preset}##row_separator_preset") };
        if ui.small_button(label) {
            let preset = preset.to_string();
            app.mutate_and_persist(move |state| state.config.row_separator = preset.clone());
        }
    }
}

fn render_window_sizing_section(ui: &Ui, app: &AppHandle) {
    let mut fixed_window_height = app.lock().config.fixed_window_height;
    if ui.checkbox("Fixed window height", &mut fixed_window_height) {
        app.mutate_and_persist(|state| state.config.fixed_window_height = fixed_window_height);
    }
    if fixed_window_height {
        let mut window_height = app.lock().config.window_height;
        if Slider::new("Window height", 50.0f32, 800.0f32).build(ui, &mut window_height) {
            app.mutate_and_persist(|state| state.config.window_height = window_height);
        }
    }

    let mut margin = app.lock().config.window_right_margin;
    if Slider::new("Right margin", 0.0f32, 50.0f32).build(ui, &mut margin) {
        app.mutate_and_persist(|state| state.config.window_right_margin = margin);
    }

    let mut padding = app.lock().config.padding;
    if Slider::new("Padding", 0.0f32, 30.0f32).build(ui, &mut padding) {
        app.mutate_and_persist(|state| state.config.padding = padding);
    }

    let mut fix_label_width = app.lock().config.fix_label_width;
    if ui.checkbox("Fixed label column width", &mut fix_label_width) {
        app.mutate_and_persist(|state| state.config.fix_label_width = fix_label_width);
    }
    if fix_label_width {
        let mut label_width = app.lock().config.label_width;
        if Slider::new("Label width", 20.0f32, 300.0f32).build(ui, &mut label_width) {
            app.mutate_and_persist(|state| state.config.label_width = label_width);
        }
    }
}
