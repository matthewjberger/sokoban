use crate::theme::*;
use nightshade::prelude::*;

pub fn build(tree: &mut UiTreeBuilder, label: &str) -> Entity {
    let button = tree
        .add_node()
        .flow_child(Ab(MENU_BUTTON_SIZE))
        .with_rect(3.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG)
        .color_raw::<UiHover>(PANEL_HOVER)
        .color_raw::<UiPressed>(PANEL_PRESSED)
        .color_raw::<UiFocused>(ACCENT_DIM)
        .with_transition::<UiHover>(14.0, 8.0)
        .with_transition::<UiPressed>(20.0, 12.0)
        .with_transition::<UiFocused>(14.0, 8.0)
        .with_interaction()
        .with_cursor_icon(winit::window::CursorIcon::Pointer)
        .entity();
    tree.in_parent(button, |tree| {
        tree.add_node()
            .window(
                Ab(vec2(18.0, 0.0)) + Rl(vec2(0.0, 50.0)),
                Ab(vec2(3.0, MENU_BUTTON_HEIGHT - 18.0)),
                Anchor::CenterLeft,
            )
            .with_rect(0.0, 0.0, TRANSPARENT)
            .color_raw::<UiBase>(ACCENT)
            .color_raw::<UiFocused>(ACCENT_HOT)
            .with_transition::<UiFocused>(14.0, 8.0)
            .entity();
        tree.add_node()
            .window(
                Ab(vec2(38.0, 0.0)) + Rl(vec2(0.0, 50.0)),
                Ab(vec2(MENU_BUTTON_SIZE.x - 56.0, MENU_BUTTON_HEIGHT)),
                Anchor::CenterLeft,
            )
            .with_text(label, 19.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_COLOR)
            .color_raw::<UiFocused>(WHITE)
            .with_transition::<UiFocused>(14.0, 8.0)
            .entity();
    });
    button
}

/// A compact button, handing back both the button and its label so a caller
/// that retitles the button at runtime does not have to hunt for the child.
pub fn build_sized(
    tree: &mut UiTreeBuilder,
    label: &str,
    size: Vec2,
    font_size: f32,
) -> (Entity, Entity) {
    let button = tree
        .add_node()
        .flow_child(Ab(size))
        .with_rect(3.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG)
        .color_raw::<UiHover>(PANEL_HOVER)
        .color_raw::<UiPressed>(PANEL_PRESSED)
        .with_transition::<UiHover>(14.0, 8.0)
        .with_transition::<UiPressed>(20.0, 12.0)
        .with_interaction()
        .with_cursor_icon(winit::window::CursorIcon::Pointer)
        .entity();
    let mut text = Entity::default();
    tree.in_parent(button, |tree| {
        text = tree
            .add_node()
            .window(Rl(vec2(50.0, 50.0)), Ab(size), Anchor::Center)
            .with_text(label, font_size)
            .text_center()
            .color_raw::<UiBase>(TEXT_COLOR)
            .entity();
    });
    (button, text)
}
