use std::collections::BTreeMap;

use pane_switcher_core::app::NavigationTarget;
use pane_switcher_core::config::Config;
use pane_switcher_core::model::{
    DeadSessionInput, PaneInput, PaneManifestInput, SessionInput, TabInput,
};
use pane_switcher_core::style::{ColorSpec, ThemePalette};
use zellij_tile::prelude::{
    BareKey, FloatingPaneCoordinates, KeyModifier, KeyWithModifier, Mouse, PaletteColor, PaneId,
    PaneInfo, PaneManifest, SessionInfo, Style, TabInfo,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NavigationAction {
    SwitchSession {
        session: String,
    },
    SwitchCurrentTab {
        tab_position: usize,
    },
    SwitchOtherTab {
        session: String,
        tab_position: usize,
    },
    FocusCurrentPane {
        pane_id: PaneId,
    },
    SwitchSessionWithPane {
        session: String,
        tab_position: usize,
        pane_id: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputIntent {
    Insert(char),
    Backspace,
    DeleteWord,
    Clear,
    Up,
    Down,
    PageUp,
    PageDown,
    Enter,
    CreateTab,
    Close,
    Ignored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseIntent {
    ActivateRow(usize),
    HoverSlot(Option<usize>),
    Up,
    Down,
    Ignored,
}

pub fn configured_floating_coordinates(config: &Config) -> Option<FloatingPaneCoordinates> {
    FloatingPaneCoordinates::new(
        None,
        None,
        Some(config.width.clone()),
        Some(config.height.clone()),
        None,
        None,
    )
}

pub fn mouse_intent(mouse: &Mouse, viewport_start: usize, viewport_end: usize) -> MouseIntent {
    match *mouse {
        Mouse::LeftClick(line, _) => visible_row(line, viewport_start, viewport_end)
            .map_or(MouseIntent::Ignored, MouseIntent::ActivateRow),
        Mouse::Hover(line, _) => MouseIntent::HoverSlot(visible_slot(line)),
        Mouse::ScrollUp(_) => MouseIntent::Up,
        Mouse::ScrollDown(_) => MouseIntent::Down,
        _ => MouseIntent::Ignored,
    }
}

fn visible_row(line: isize, viewport_start: usize, viewport_end: usize) -> Option<usize> {
    let line = usize::try_from(line).ok()?;
    let offset = line.checked_sub(2)?;
    let index = viewport_start.saturating_add(offset);
    (index < viewport_end).then_some(index)
}

fn visible_slot(line: isize) -> Option<usize> {
    usize::try_from(line).ok()?.checked_sub(2)
}

pub fn key_intent(key: &KeyWithModifier) -> InputIntent {
    let has_ctrl = key.key_modifiers.contains(&KeyModifier::Ctrl);
    let has_other_modifier = key.key_modifiers.contains(&KeyModifier::Alt)
        || key.key_modifiers.contains(&KeyModifier::Super);

    if let BareKey::Char(character) = key.bare_key {
        if !character.is_control() && !character.is_ascii() {
            return InputIntent::Insert(character);
        }
    }

    if has_ctrl && !has_other_modifier && matches!(key.bare_key, BareKey::Enter) {
        return InputIntent::CreateTab;
    }

    if has_ctrl && !has_other_modifier {
        if let BareKey::Char(character) = key.bare_key {
            return match character.to_ascii_lowercase() {
                'p' => InputIntent::Up,
                'n' => InputIntent::Down,
                'w' => InputIntent::DeleteWord,
                'u' => InputIntent::Clear,
                _ => InputIntent::Ignored,
            };
        }
    }
    if has_ctrl || has_other_modifier {
        return InputIntent::Ignored;
    }

    match key.bare_key {
        BareKey::Up => InputIntent::Up,
        BareKey::Down => InputIntent::Down,
        BareKey::Tab if key.key_modifiers.contains(&KeyModifier::Shift) => InputIntent::Up,
        BareKey::Tab => InputIntent::Down,
        BareKey::PageUp => InputIntent::PageUp,
        BareKey::PageDown => InputIntent::PageDown,
        BareKey::Backspace => InputIntent::Backspace,
        BareKey::Enter => InputIntent::Enter,
        BareKey::Esc => InputIntent::Close,
        BareKey::Char(character) if !character.is_control() => InputIntent::Insert(character),
        _ => InputIntent::Ignored,
    }
}

pub fn session_input(session: &SessionInfo) -> SessionInput {
    SessionInput {
        name: session.name.clone(),
        is_current: session.is_current_session,
        connected_clients: session.connected_clients,
        tabs: tab_inputs(&session.tabs),
        panes: pane_manifest_input(&session.panes),
    }
}

pub fn tab_inputs(tabs: &[TabInfo]) -> Vec<TabInput> {
    let mut tabs = tabs
        .iter()
        .map(|tab| TabInput {
            position: tab.position,
            tab_id: tab.tab_id,
            name: tab.name.clone(),
            active: tab.active,
        })
        .collect::<Vec<_>>();
    tabs.sort_by_key(|tab| (tab.position, tab.tab_id));
    tabs
}

pub fn pane_manifest_input(manifest: &PaneManifest) -> PaneManifestInput {
    PaneManifestInput {
        panes: manifest
            .panes
            .iter()
            .map(|(position, panes)| {
                let mut panes = panes.iter().map(pane_input).collect::<Vec<_>>();
                panes.sort_by_key(|pane| pane.id);
                (*position, panes)
            })
            .collect::<BTreeMap<_, _>>(),
    }
}

pub fn dead_session_inputs(sessions: &[(String, std::time::Duration)]) -> Vec<DeadSessionInput> {
    let mut sessions = sessions
        .iter()
        .map(|(name, age)| DeadSessionInput {
            name: name.clone(),
            age: *age,
        })
        .collect::<Vec<_>>();
    sessions.sort_by(|left, right| left.name.cmp(&right.name));
    sessions
}

pub fn theme_palette(style: &Style) -> ThemePalette {
    let colors = &style.colors;
    let text = colors.text_unselected;
    let selected = colors.text_selected;
    ThemePalette {
        default: palette_color(text.base),
        gray: palette_color(text.base),
        green: palette_color(text.emphasis_2),
        cyan: palette_color(text.emphasis_1),
        magenta: palette_color(text.emphasis_3),
        orange: palette_color(text.emphasis_0),
        blue: palette_color(colors.multiplayer_user_colors.player_2),
        red: palette_color(colors.exit_code_error.base),
        yellow: palette_color(colors.multiplayer_user_colors.player_4),
        dark_gray: palette_color(text.base),
        border: palette_color(colors.frame_selected.base),
        selected_foreground: palette_color(selected.base),
        selected_background: palette_color(selected.background),
        hover_background: hover_background(&text, &selected),
    }
}

fn hover_background(
    normal: &zellij_tile::prelude::StyleDeclaration,
    selected: &zellij_tile::prelude::StyleDeclaration,
) -> ColorSpec {
    let normal_rgb = palette_rgb(normal.background);
    let selected_rgb = palette_rgb(selected.background);
    let mixed = blend_rgb(normal_rgb, selected_rgb, 1, 3);
    if mixed != normal_rgb {
        return ColorSpec::Rgb(mixed);
    }

    let foreground_fallback = blend_rgb(normal_rgb, palette_rgb(normal.base), 1, 8);
    ColorSpec::Rgb(foreground_fallback)
}

fn blend_rgb(
    base: (u8, u8, u8),
    accent: (u8, u8, u8),
    accent_weight: u32,
    total_weight: u32,
) -> (u8, u8, u8) {
    let mix = |base: u8, accent: u8| {
        let base = u32::from(base);
        let accent = u32::from(accent);
        ((base * (total_weight - accent_weight) + accent * accent_weight) / total_weight) as u8
    };
    (
        mix(base.0, accent.0),
        mix(base.1, accent.1),
        mix(base.2, accent.2),
    )
}

fn palette_rgb(color: PaletteColor) -> (u8, u8, u8) {
    match color {
        PaletteColor::Rgb(rgb) => rgb,
        PaletteColor::EightBit(index) => indexed_rgb(index),
    }
}

fn indexed_rgb(index: u8) -> (u8, u8, u8) {
    const SYSTEM: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    if let Some(rgb) = SYSTEM.get(usize::from(index)) {
        return *rgb;
    }
    if index < 232 {
        let index = index - 16;
        let channel = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
        return (
            channel(index / 36),
            channel((index % 36) / 6),
            channel(index % 6),
        );
    }
    let gray = 8 + (index - 232) * 10;
    (gray, gray, gray)
}

fn palette_color(color: PaletteColor) -> ColorSpec {
    match color {
        PaletteColor::Rgb(rgb) => ColorSpec::Rgb(rgb),
        PaletteColor::EightBit(index) => ColorSpec::Indexed(index),
    }
}

pub fn navigation_action(
    target: &NavigationTarget,
    current_session: Option<&str>,
) -> NavigationAction {
    match target {
        NavigationTarget::SwitchSession { session } => NavigationAction::SwitchSession {
            session: session.clone(),
        },
        NavigationTarget::SwitchTab {
            session,
            tab_position,
        } if current_session == Some(session.as_str()) => NavigationAction::SwitchCurrentTab {
            tab_position: *tab_position,
        },
        NavigationTarget::SwitchTab {
            session,
            tab_position,
        } => NavigationAction::SwitchOtherTab {
            session: session.clone(),
            tab_position: *tab_position,
        },
        NavigationTarget::FocusPane {
            session,
            tab_position,
            pane_id,
        } if current_session == Some(session.as_str()) => NavigationAction::FocusCurrentPane {
            pane_id: PaneId::Terminal(*pane_id),
        },
        NavigationTarget::FocusPane {
            session,
            tab_position,
            pane_id,
        } => NavigationAction::SwitchSessionWithPane {
            session: session.clone(),
            tab_position: *tab_position,
            pane_id: *pane_id,
        },
    }
}

fn pane_input(pane: &PaneInfo) -> PaneInput {
    PaneInput {
        id: pane.id,
        title: pane.title.clone(),
        is_focused: pane.is_focused,
        is_floating: pane.is_floating,
        is_fullscreen: pane.is_fullscreen,
        is_suppressed: pane.is_suppressed,
        exited: pane.exited,
        is_selectable: pane.is_selectable,
        is_plugin: pane.is_plugin,
        initial_command: pane.terminal_command.clone(),
    }
}
