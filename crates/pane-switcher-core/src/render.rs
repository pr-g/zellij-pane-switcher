use std::borrow::Cow;

use unicode_width::UnicodeWidthStr;

use crate::color::ProgressColors;
use crate::config::{Config, IconsMode};
use crate::icons::{classify_command, icon_for, PaneVisualState};
use crate::rows::{ProgressState, Row, Rows};
use crate::style::{
    pad_spans, spans_to_ansi_with_palette, truncate_spans, ForegroundColor, StyledSpan,
    ThemePalette,
};
use crate::template::TemplateField;

const BORDER_COLOR: ForegroundColor = ForegroundColor::Border;
const SEARCH_PREFIX: &str = " FIND  › ";
const SEARCH_HINT_FULL: &str = "Ctrl+Enter new tab";
const SEARCH_HINT_COMPACT: &str = "Ctrl+Enter";
const SEARCH_ESCAPE: &str = "Esc";
const MIN_SEARCH_QUERY_WIDTH: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderOptions {
    pub color: bool,
    pub palette: ThemePalette,
    pub hovered_slot: Option<usize>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            color: true,
            palette: ThemePalette::default(),
            hovered_slot: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RenderFrame {
    pub lines: Vec<String>,
    pub cursor: Option<(usize, usize)>,
    pub viewport_start: usize,
    pub viewport_end: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn render_frame(
    rows: &Rows,
    selected: Option<usize>,
    cols: usize,
    height: usize,
    query: &str,
    config: &Config,
    spinner_tick: u64,
    options: RenderOptions,
) -> RenderFrame {
    if cols == 0 || height == 0 {
        return RenderFrame::default();
    }

    let (search, cursor) = search_line(cols, query);
    let mut lines = Vec::with_capacity(height);
    lines.push(render_spans(search, options));
    lines.push(render_text_line(divider(cols), options));

    let row_height = height.saturating_sub(3);
    let selected_index = selected.filter(|index| *index < rows.rows.len());
    let start = selected_index
        .map(|index| (index + 1).saturating_sub(row_height))
        .unwrap_or(0)
        .min(rows.rows.len());
    let end = start.saturating_add(row_height).min(rows.rows.len());
    let hovered_index = options
        .hovered_slot
        .filter(|slot| *slot < row_height)
        .and_then(|slot| start.checked_add(slot))
        .filter(|index| *index < end && selected_index != Some(*index));
    for (index, row) in rows.rows[start..end].iter().enumerate() {
        let absolute_index = start + index;
        let spans = row_spans(row, config, spinner_tick);
        lines.push(render_spans(
            fitted_spans(
                spans,
                cols,
                selected_index == Some(absolute_index),
                hovered_index == Some(absolute_index),
            ),
            options,
        ));
    }

    while lines.len() < height.saturating_sub(1) {
        lines.push(render_spans(
            fitted_spans(Vec::new(), cols, false, false),
            options,
        ));
    }
    lines.push(render_text_line(
        fitted_text(
            " ↑↓/Tab move  Shift+Tab back  Enter/click jump  Ctrl+Enter new tab  wheel scroll  Esc close ",
            cols,
            BORDER_COLOR,
        ),
        options,
    ));
    lines.truncate(height);

    RenderFrame {
        lines,
        cursor,
        viewport_start: start,
        viewport_end: end,
    }
}

fn row_spans(row: &Row, config: &Config, spinner_tick: u64) -> Vec<StyledSpan> {
    let mut spans = Vec::new();
    for (part_index, part) in config.row_format.parts().iter().enumerate() {
        let part_color = config.colors.part_color(part_index);
        match part {
            crate::template::TemplatePart::Literal(text) => {
                push_span(
                    &mut spans,
                    text.clone(),
                    part_color.unwrap_or(ForegroundColor::Default),
                    false,
                );
            }
            crate::template::TemplatePart::Field(field) => match field {
                TemplateField::Progress => {
                    let (glyph, color) =
                        progress_glyph(row.progress, spinner_tick, &config.colors.progress);
                    push_span(&mut spans, glyph.to_owned(), color, false);
                }
                TemplateField::Icon => {
                    let (glyph, color) = type_icon(row, config.icons);
                    push_span(&mut spans, glyph, part_color.unwrap_or(color), false);
                }
                TemplateField::Title => {
                    push_matched_text(
                        &mut spans,
                        &row.title,
                        row.matches.get(*field),
                        part_color.unwrap_or(field_color(*field)),
                    );
                }
                TemplateField::Dir => {
                    push_matched_text(
                        &mut spans,
                        &row.dir,
                        row.matches.get(*field),
                        part_color.unwrap_or(field_color(*field)),
                    );
                }
                TemplateField::Session => {
                    push_matched_text(
                        &mut spans,
                        &row.session,
                        row.matches.get(*field),
                        part_color.unwrap_or(field_color(*field)),
                    );
                }
                TemplateField::Tab => {
                    push_matched_text(
                        &mut spans,
                        &row.tab_name,
                        row.matches.get(*field),
                        part_color.unwrap_or(field_color(*field)),
                    );
                }
                TemplateField::Command => {
                    push_matched_text(
                        &mut spans,
                        &row.command,
                        row.matches.get(*field),
                        part_color.unwrap_or(field_color(*field)),
                    );
                }
                TemplateField::TabId => {
                    let tab_id = row.key.tab_id().unwrap_or_default().to_string();
                    push_matched_text(
                        &mut spans,
                        &tab_id,
                        row.matches.get(*field),
                        part_color.unwrap_or(field_color(*field)),
                    );
                }
                TemplateField::PaneId => {
                    push_matched_text(
                        &mut spans,
                        &row.pane.id.to_string(),
                        row.matches.get(*field),
                        part_color.unwrap_or(field_color(*field)),
                    );
                }
            },
        }
    }
    spans
}

fn field_color(field: TemplateField) -> ForegroundColor {
    match field {
        TemplateField::Title => ForegroundColor::Cyan,
        TemplateField::Dir => ForegroundColor::Blue,
        TemplateField::Session => ForegroundColor::Magenta,
        TemplateField::Tab => ForegroundColor::Orange,
        TemplateField::Command => ForegroundColor::Green,
        TemplateField::TabId | TemplateField::PaneId => ForegroundColor::DarkGray,
        TemplateField::Progress | TemplateField::Icon => ForegroundColor::Default,
    }
}

fn progress_glyph(
    state: ProgressState,
    spinner_tick: u64,
    colors: &ProgressColors,
) -> (&'static str, ForegroundColor) {
    match state {
        ProgressState::Running => (
            match spinner_tick % 4 {
                0 => "◐",
                1 => "◓",
                2 => "◑",
                _ => "◒",
            },
            colors.running,
        ),
        ProgressState::Idle => ("✓", colors.idle),
        ProgressState::Exited => ("✕", colors.exited),
        ProgressState::Unknown => ("?", colors.unknown),
    }
}

fn type_icon(row: &Row, mode: IconsMode) -> (String, ForegroundColor) {
    let state = if row.full_command.trim().is_empty() || row.full_command.trim() == "?" {
        if row.pane.is_focused {
            PaneVisualState::Focused
        } else if row.pane.is_suppressed {
            PaneVisualState::Stacked
        } else if row.pane.is_fullscreen {
            PaneVisualState::Fullscreen
        } else if row.pane.is_floating {
            PaneVisualState::Floating
        } else {
            PaneVisualState::Unknown
        }
    } else {
        let command = row
            .full_command
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        classify_command(&command)
    };
    let color = match state {
        PaneVisualState::Focused => ForegroundColor::Green,
        PaneVisualState::Shell => ForegroundColor::Green,
        PaneVisualState::Command => ForegroundColor::Cyan,
        PaneVisualState::Codex => ForegroundColor::Magenta,
        PaneVisualState::Claude => ForegroundColor::Orange,
        PaneVisualState::Editor => ForegroundColor::Blue,
        PaneVisualState::Floating | PaneVisualState::Fullscreen | PaneVisualState::Stacked => {
            ForegroundColor::Blue
        }
        PaneVisualState::Exited => ForegroundColor::Red,
        PaneVisualState::Unknown => ForegroundColor::DarkGray,
    };
    (icon_for(state, mode).to_owned(), color)
}

fn push_matched_text(
    spans: &mut Vec<StyledSpan>,
    text: &str,
    positions: &[usize],
    foreground: ForegroundColor,
) {
    if positions.is_empty() {
        push_span(spans, text.to_owned(), foreground, false);
        return;
    }

    let mut position_index = 0;
    let mut segment = String::new();
    let mut segment_matched = false;
    for (character_index, character) in text.chars().enumerate() {
        while positions
            .get(position_index)
            .is_some_and(|position| *position < character_index)
        {
            position_index += 1;
        }
        let matched = positions.get(position_index) == Some(&character_index);
        if !segment.is_empty() && matched != segment_matched {
            push_span(
                spans,
                std::mem::take(&mut segment),
                field_segment_color(foreground, segment_matched),
                segment_matched,
            );
        }
        segment_matched = matched;
        segment.push(character);
    }
    if !segment.is_empty() {
        push_span(
            spans,
            segment,
            field_segment_color(foreground, segment_matched),
            segment_matched,
        );
    }
}

fn field_segment_color(base: ForegroundColor, matched: bool) -> ForegroundColor {
    if matched {
        ForegroundColor::Yellow
    } else {
        base
    }
}

fn push_span(spans: &mut Vec<StyledSpan>, text: String, foreground: ForegroundColor, bold: bool) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut() {
        if last.foreground == foreground
            && last.bold == bold
            && !last.selected_background
            && !last.hover_background
        {
            last.text.push_str(&text);
            return;
        }
    }
    spans.push(StyledSpan {
        text,
        foreground,
        selected_background: false,
        hover_background: false,
        bold,
    });
}

fn fitted_spans(
    spans: Vec<StyledSpan>,
    cols: usize,
    selected: bool,
    hovered: bool,
) -> Vec<StyledSpan> {
    if cols == 0 {
        return Vec::new();
    }
    let mut fitted = truncate_spans(spans, cols);
    fitted = pad_spans(fitted, cols);
    if selected {
        set_selected_background(&mut fitted);
    } else if hovered {
        set_hover_background(&mut fitted);
    }
    fitted
}

fn set_selected_background(spans: &mut [StyledSpan]) {
    for span in spans {
        span.selected_background = true;
    }
}

fn set_hover_background(spans: &mut [StyledSpan]) {
    for span in spans {
        span.hover_background = true;
    }
}

fn fitted_text(text: &str, cols: usize, foreground: ForegroundColor) -> Vec<StyledSpan> {
    fitted_spans(vec![StyledSpan::new(text, foreground)], cols, false, false)
}

fn search_line(cols: usize, query: &str) -> (Vec<StyledSpan>, Option<(usize, usize)>) {
    if cols == 0 {
        return (Vec::new(), None);
    }
    let prefix_width = UnicodeWidthStr::width(SEARCH_PREFIX);
    let hint = search_hint(cols, prefix_width);
    let suffix_width = search_suffix_width(hint);
    if cols < prefix_width + suffix_width {
        return (
            fitted_text(SEARCH_PREFIX, cols, ForegroundColor::Default),
            None,
        );
    }
    let available = cols.saturating_sub(prefix_width + suffix_width);

    let display_query = sanitize_search_query(query);
    let visible_query = tail_by_width(&display_query, available);
    let query_width = UnicodeWidthStr::width(visible_query.as_str());
    let padding = cols.saturating_sub(prefix_width + query_width + suffix_width);
    let mut spans = vec![StyledSpan::new(SEARCH_PREFIX, ForegroundColor::Default)];
    if !visible_query.is_empty() {
        spans.push(StyledSpan::new(visible_query, ForegroundColor::Default));
    }
    if padding > 0 {
        spans.push(StyledSpan::new(
            " ".repeat(padding),
            ForegroundColor::Default,
        ));
    }
    let suffix = hint.map_or_else(
        || format!("  {SEARCH_ESCAPE} "),
        |hint| format!("  {hint}  {SEARCH_ESCAPE} "),
    );
    spans.push(StyledSpan::new(suffix, BORDER_COLOR));
    let cursor = Some(((prefix_width + query_width).min(cols), 0));
    (pad_spans(spans, cols), cursor)
}

fn sanitize_search_query(query: &str) -> Cow<'_, str> {
    if query.chars().any(char::is_control) {
        Cow::Owned(
            query
                .chars()
                .map(|character| {
                    if character.is_control() {
                        ' '
                    } else {
                        character
                    }
                })
                .collect(),
        )
    } else {
        Cow::Borrowed(query)
    }
}

fn search_hint(cols: usize, prefix_width: usize) -> Option<&'static str> {
    [SEARCH_HINT_FULL, SEARCH_HINT_COMPACT]
        .into_iter()
        .find(|hint| {
            cols >= prefix_width + MIN_SEARCH_QUERY_WIDTH + search_suffix_width(Some(hint))
        })
}

fn search_suffix_width(hint: Option<&str>) -> usize {
    2 + hint.map_or(0, UnicodeWidthStr::width)
        + if hint.is_some() { 2 } else { 0 }
        + UnicodeWidthStr::width(SEARCH_ESCAPE)
        + 1
}

fn divider(cols: usize) -> Vec<StyledSpan> {
    vec![StyledSpan::new("─".repeat(cols), BORDER_COLOR)]
}

fn tail_by_width(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_owned();
    }
    let mut tail = Vec::new();
    let mut used = 0;
    for character in text.chars().rev() {
        let char_width = unicode_width::UnicodeWidthChar::width(character).unwrap_or(0);
        if used + char_width > width {
            break;
        }
        tail.push(character);
        used += char_width;
    }
    tail.into_iter().rev().collect()
}

fn render_text_line(spans: Vec<StyledSpan>, options: RenderOptions) -> String {
    render_spans(spans, options)
}

fn render_spans(spans: Vec<StyledSpan>, options: RenderOptions) -> String {
    spans_to_ansi_with_palette(&spans, options.color, &options.palette)
}
