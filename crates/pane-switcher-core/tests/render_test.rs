use std::collections::BTreeMap;

use pane_switcher_core::config::Config;
use pane_switcher_core::model::{LivePane, NodeKey};
use pane_switcher_core::render::{render_frame, RenderOptions};
use pane_switcher_core::rows::{FieldMatches, ProgressState, Row, Rows};
use pane_switcher_core::style::{ColorSpec, ThemePalette};
use unicode_width::UnicodeWidthStr;

fn row(id: u32, title: &str, dir: &str, command: &str) -> Row {
    let key = NodeKey::Pane {
        session: "work".into(),
        tab_id: 11,
        pane_id: id,
    };
    Row {
        key: key.clone(),
        session: "work".into(),
        tab_name: "code".into(),
        tab_position: 0,
        title: title.into(),
        dir: dir.into(),
        full_dir: dir.into(),
        command: command.into(),
        full_command: command.into(),
        pane: LivePane {
            key,
            id,
            title: title.into(),
            is_focused: id == 1,
            is_floating: false,
            is_fullscreen: false,
            is_suppressed: false,
            exited: false,
            initial_command: Some(command.into()),
        },
        is_current_session: true,
        progress: ProgressState::Running,
        score: 100,
        matches: FieldMatches::default(),
    }
}

fn render_one(selected: bool, query: &str) -> String {
    let config = Config::default();
    let mut first = row(1, "src renderer", "src/ui", "codex --full-auto");
    if !query.is_empty() {
        first.matches.title = vec![0];
    }
    let panes = Rows { rows: vec![first] };
    let frame = render_frame(
        &panes,
        selected.then_some(0),
        60,
        12,
        query,
        &config,
        1,
        RenderOptions::default(),
    );
    frame.lines.join("\n")
}

#[test]
fn progress_and_type_icons_use_semantic_colors() {
    let rendered = render_one(false, "");

    assert!(rendered.contains("\u{1b}[38;5;51m"));
    assert!(rendered.contains("\u{1b}[38;5;201m"));
}

#[test]
fn row_fields_use_distinct_semantic_colors() {
    let config = Config::from_map(&BTreeMap::from([(
        "row_format".into(),
        "{title} {dir} {session} {tab} {tab_id} {pane_id} {command}".into(),
    )]));
    let mut shell = row(1, "src renderer", "src/ui", "zsh");
    shell.progress = ProgressState::Idle;
    let rows = Rows { rows: vec![shell] };
    let frame = render_frame(
        &rows,
        None,
        80,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );
    let rendered = &frame.lines[2];

    assert!(rendered.contains("\u{1b}[38;5;51msrc renderer\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;75msrc/ui\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;201mwork\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;208mcode\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;240m11\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;46mzsh\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;240m1\u{1b}[0m"));
}

#[test]
fn selected_rows_keep_field_colors_over_the_selection_background() {
    let config = Config::from_map(&BTreeMap::from([(
        "row_format".into(),
        "{title} {dir} {session} {tab} {tab_id} {pane_id} {command}".into(),
    )]));
    let mut shell = row(1, "src renderer", "src/ui", "zsh");
    shell.progress = ProgressState::Idle;
    let rows = Rows { rows: vec![shell] };
    let frame = render_frame(
        &rows,
        Some(0),
        80,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );
    let rendered = &frame.lines[2];

    assert!(rendered.contains("\u{1b}[38;5;51;48;5;24msrc renderer\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;75;48;5;24msrc/ui\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;201;48;5;24mwork\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;208;48;5;24mcode\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;240;48;5;24m11\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;46;48;5;24mzsh\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;240;48;5;24m1\u{1b}[0m"));
}

#[test]
fn nested_colors_style_template_fragments() {
    let config = Config::from_map(&BTreeMap::from([
        (
            "row_format".into(),
            "{title}  {tab_id}-{pane_id}  {dir}".into(),
        ),
        (
            "colors".into(),
            r#""{title}" "red"
"{tab_id}-{pane_id}" "yellow""#
                .into(),
        ),
    ]));
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex")],
    };
    let frame = render_frame(
        &rows,
        None,
        80,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );
    let rendered = &frame.lines[2];

    assert!(rendered.contains("\u{1b}[38;5;196msrc renderer\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;226m11-1\u{1b}[0m"));
}

#[test]
fn nested_progress_colors_are_selected_by_state() {
    let config = Config::from_map(&BTreeMap::from([
        ("row_format".into(), "{progress} {title}".into()),
        (
            "colors".into(),
            r#"progress {
    running "green"
    idle "yellow"
    exited "red"
    unknown "magenta"
}"#
            .into(),
        ),
    ]));
    let mut idle = row(2, "idle", ".", "zsh");
    idle.progress = ProgressState::Idle;
    let mut exited = row(3, "exited", ".", "zsh");
    exited.progress = ProgressState::Exited;
    let mut unknown = row(4, "unknown", ".", "?");
    unknown.progress = ProgressState::Unknown;
    let rows = Rows {
        rows: vec![row(1, "running", ".", "codex"), idle, exited, unknown],
    };
    let frame = render_frame(
        &rows,
        None,
        60,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );

    assert!(frame.lines[2].contains("\u{1b}[38;5;46m◐\u{1b}[0m"));
    assert!(frame.lines[3].contains("\u{1b}[38;5;226m✓\u{1b}[0m"));
    assert!(frame.lines[4].contains("\u{1b}[38;5;196m✕\u{1b}[0m"));
    assert!(frame.lines[5].contains("\u{1b}[38;5;201m?\u{1b}[0m"));
}

#[test]
fn idle_progress_uses_a_green_check_mark() {
    let mut idle = row(1, "idle", ".", "zsh");
    idle.progress = ProgressState::Idle;
    let rows = Rows { rows: vec![idle] };
    let frame = render_frame(
        &rows,
        None,
        80,
        12,
        "",
        &Config::default(),
        0,
        RenderOptions::default(),
    );

    assert!(frame.lines[2].contains("\u{1b}[38;5;46m✓\u{1b}[0m"));
}

#[test]
fn exited_progress_uses_a_red_cross_mark() {
    let mut exited = row(1, "failed", ".", "false");
    exited.progress = ProgressState::Exited;
    let rows = Rows { rows: vec![exited] };
    let frame = render_frame(
        &rows,
        None,
        80,
        12,
        "",
        &Config::default(),
        0,
        RenderOptions::default(),
    );

    assert!(frame.lines[2].contains("\u{1b}[38;5;196m✕\u{1b}[0m"));
}

#[test]
fn color_rules_prefer_the_longest_match_and_ignore_invalid_values() {
    let config = Config::from_map(&BTreeMap::from([
        ("row_format".into(), "{tab_id}-{pane_id}  {title}".into()),
        (
            "colors".into(),
            r#""{tab_id}" "red"
"{tab_id}-{pane_id}" "yellow"
"{title}" "not-a-color""#
                .into(),
        ),
    ]));
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex")],
    };
    let frame = render_frame(
        &rows,
        None,
        60,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );
    let rendered = &frame.lines[2];

    assert!(rendered.contains("\u{1b}[38;5;226m11-1\u{1b}[0m"));
    assert!(rendered.contains("\u{1b}[38;5;51msrc renderer\u{1b}[0m"));
}

#[test]
fn equally_specific_color_rules_use_the_later_definition() {
    let config = Config::from_map(&BTreeMap::from([
        ("row_format".into(), "{title}".into()),
        (
            "colors".into(),
            r#""{title}" "red"
"{title}" "green""#
                .into(),
        ),
    ]));
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex")],
    };
    let frame = render_frame(
        &rows,
        None,
        60,
        12,
        "",
        &config,
        0,
        RenderOptions::default(),
    );

    assert!(frame.lines[2].contains("\u{1b}[38;5;46msrc renderer\u{1b}[0m"));
}

#[test]
fn shell_type_icons_use_a_visible_green_instead_of_idle_gray() {
    let mut shell = row(1, "shell", ".", "zsh");
    shell.progress = ProgressState::Idle;
    let rows = Rows { rows: vec![shell] };
    let frame = render_frame(
        &rows,
        Some(0),
        80,
        12,
        "",
        &Config::default(),
        0,
        RenderOptions::default(),
    );
    let rendered = frame.lines.join("\n");

    assert!(rendered.contains("38;5;46"));
    assert!(rendered.contains("$"));
    assert!(rendered.contains("✓"));
}

#[test]
fn fuzzy_match_segments_are_bold_yellow() {
    let rendered = render_one(false, "s");

    assert!(rendered.contains("\u{1b}[1;38;5;226m"));
}

#[test]
fn selected_rows_have_a_full_width_dark_background() {
    let config = Config::default();
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex --full-auto")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        60,
        12,
        "",
        &config,
        1,
        RenderOptions::default(),
    );

    assert!(frame.lines[2].contains("48;5;24"));
    assert!(frame.lines[2].contains("38;5;51;48;5;24"));
    assert!(frame.lines[2].contains("38;5;201;48;5;24"));
    assert_line_widths(&frame.lines, 60);
}

#[test]
fn hovered_rows_use_a_distinct_full_width_background() {
    let rows = Rows {
        rows: vec![
            row(1, "selected", ".", "zsh"),
            row(2, "hovered", ".", "zsh"),
        ],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        60,
        12,
        "",
        &Config::default(),
        0,
        RenderOptions {
            palette: ThemePalette {
                selected_background: ColorSpec::Rgb((40, 50, 60)),
                hover_background: ColorSpec::Rgb((10, 20, 30)),
                ..ThemePalette::default()
            },
            hovered_slot: Some(1),
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("48;2;40;50;60"));
    assert!(!frame.lines[2].contains("48;2;10;20;30"));
    assert!(frame.lines[3].contains("48;2;10;20;30"));
    assert!(!frame.lines[3].contains("48;2;40;50;60"));
    assert_line_widths(&frame.lines, 60);
}

#[test]
fn selected_row_background_takes_priority_over_hover() {
    let rows = Rows {
        rows: vec![row(1, "selected", ".", "zsh")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        60,
        12,
        "",
        &Config::default(),
        0,
        RenderOptions {
            palette: ThemePalette {
                selected_background: ColorSpec::Rgb((40, 50, 60)),
                hover_background: ColorSpec::Rgb((10, 20, 30)),
                ..ThemePalette::default()
            },
            hovered_slot: Some(0),
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("48;2;40;50;60"));
    assert!(!frame.lines[2].contains("48;2;10;20;30"));
}

#[test]
fn hover_slot_tracks_the_row_under_the_pointer_when_the_viewport_moves() {
    let rows = Rows {
        rows: (1..=20)
            .map(|id| row(id, &format!("pane-{id}"), ".", "zsh"))
            .collect(),
    };
    let frame = render_frame(
        &rows,
        Some(10),
        60,
        6,
        "",
        &Config::default(),
        0,
        RenderOptions {
            palette: ThemePalette {
                selected_background: ColorSpec::Rgb((40, 50, 60)),
                hover_background: ColorSpec::Rgb((10, 20, 30)),
                ..ThemePalette::default()
            },
            hovered_slot: Some(0),
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("pane-9"));
    assert!(frame.lines[2].contains("48;2;10;20;30"));
    assert!(frame.lines[4].contains("pane-11"));
    assert!(frame.lines[4].contains("48;2;40;50;60"));
}

#[test]
fn detail_panel_text_is_not_rendered() {
    let rendered = render_one(false, "");

    assert!(!rendered.contains("selected\n"));
    assert!(!rendered.contains("command  "));
}

#[test]
fn relies_on_the_zellij_pane_frame_instead_of_drawing_an_inner_border() {
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        60,
        12,
        "",
        &Config::default(),
        1,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );
    let rendered = frame.lines.join("\n");

    assert!(frame.lines[0].contains("FIND"));
    assert!(!rendered.contains('╭'));
    assert!(!rendered.contains('╰'));
    assert_line_widths(&frame.lines, 60);
}

#[test]
fn footer_documents_tab_navigation() {
    let frame = render_frame(
        &Rows::default(),
        None,
        100,
        12,
        "",
        &Config::default(),
        0,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );
    let footer = frame.lines.last().expect("footer");

    assert!(footer.contains("Tab"));
    assert!(footer.contains("Shift+Tab"));
    assert!(footer.contains("click"));
    assert!(footer.contains("Ctrl+Enter new"));
    assert!(footer.contains("wheel"));
}

#[test]
fn search_line_places_the_full_new_tab_hint_before_escape() {
    let (line, cursor) = render_search_line(100, "build", false);

    assert!(line.ends_with("Ctrl+Enter new tab  Esc "));
    let hint_byte = line.find("Ctrl+Enter").expect("hint");
    assert_eq!(UnicodeWidthStr::width(&line[..hint_byte]), 76);
    assert_eq!(UnicodeWidthStr::width(line.as_str()), 100);
    assert_eq!(cursor, Some((14, 0)));
}

#[test]
fn search_line_responsively_shortens_the_new_tab_hint() {
    for (cols, expected, hidden) in [
        (22, "Esc ", Some("Ctrl+Enter")),
        (34, "Esc ", Some("Ctrl+Enter")),
        (35, "Ctrl+Enter  Esc ", None),
        (42, "Ctrl+Enter  Esc ", None),
        (43, "Ctrl+Enter new tab  Esc ", None),
        (100, "Ctrl+Enter new tab  Esc ", None),
    ] {
        let (line, cursor) = render_search_line(cols, "query", false);

        assert!(line.ends_with(expected), "width {cols}: {line:?}");
        if let Some(hidden) = hidden {
            assert!(!line.contains(hidden), "width {cols}: {line:?}");
        }
        assert_eq!(cursor, Some((14, 0)), "width {cols}");
        assert_eq!(UnicodeWidthStr::width(line.as_str()), cols);
    }
}

#[test]
fn search_line_switches_hint_exactly_at_each_layout_threshold() {
    for (cols, hint) in [
        (14, None),
        (15, None),
        (22, None),
        (23, None),
        (34, None),
        (35, Some("Ctrl+Enter")),
        (42, Some("Ctrl+Enter")),
        (43, Some("Ctrl+Enter new tab")),
    ] {
        let (line, cursor) = render_search_line(cols, "query", false);

        assert!(
            line.ends_with(if cols < 15 { "" } else { "Esc " }),
            "width {cols}: {line:?}"
        );
        if cols < 15 {
            assert!(!line.contains("Esc"), "width {cols}: {line:?}");
            assert_eq!(cursor, None, "width {cols}");
        } else if cols == 15 {
            assert_eq!(cursor, Some((9, 0)), "width {cols}");
        } else {
            assert_eq!(cursor, Some((14, 0)), "width {cols}");
        }
        match hint {
            Some(hint) => assert!(line.contains(hint), "width {cols}: {line:?}"),
            None => assert!(!line.contains("Ctrl+Enter"), "width {cols}: {line:?}"),
        }
        assert_eq!(UnicodeWidthStr::width(line.as_str()), cols);
    }
}

#[test]
fn long_queries_scroll_without_overlapping_the_new_tab_hint() {
    let (line, cursor) = render_search_line(60, "0123456789abcdefghijklmnopqrstuvwxyz", false);

    assert_eq!(
        line,
        " FIND  › bcdefghijklmnopqrstuvwxyz  Ctrl+Enter new tab  Esc "
    );
    assert_eq!(cursor, Some((34, 0)));
}

#[test]
fn unicode_queries_truncate_by_display_width_before_the_hint() {
    let (line, cursor) = render_search_line(40, "项目一项目二项目三项目四", false);

    assert!(line.contains("项目三项目四"));
    assert!(!line.contains('�'));
    assert!(line.ends_with("Ctrl+Enter  Esc "));
    assert_eq!(UnicodeWidthStr::width(line.as_str()), 40);
    assert_eq!(cursor, Some((21, 0)));
}

#[test]
fn combining_marks_and_emoji_preserve_display_width_before_the_hint() {
    for cols in [24, 40, 80] {
        let (line, cursor) = render_search_line(cols, "e\u{301}🙂中文🙂", false);
        let visible = strip_sgr(&line);

        assert_eq!(UnicodeWidthStr::width(visible.as_str()), cols);
        assert!(visible.ends_with("Esc "), "width {cols}: {visible:?}");
        assert!(!visible.contains('�'));
        assert!(
            cursor.is_none_or(|(column, row)| column <= cols && row == 0),
            "width {cols}, cursor {cursor:?}"
        );
    }
}

#[test]
fn pasted_control_characters_never_break_the_search_line() {
    let (line, cursor) = render_search_line(60, "first\nsecond\tthird\r\u{1b}[31m", false);

    assert!(!line.chars().any(char::is_control), "{line:?}");
    assert_eq!(UnicodeWidthStr::width(line.as_str()), 60);
    assert!(line.ends_with("Ctrl+Enter new tab  Esc "));
    assert!(cursor.is_none_or(|(column, row)| column <= 60 && row == 0));
}

#[test]
fn empty_query_keeps_the_cursor_at_the_start_and_shows_the_hint() {
    let (line, cursor) = render_search_line(60, "", false);

    assert!(line.starts_with(" FIND  › "));
    assert!(line.ends_with("Ctrl+Enter new tab  Esc "));
    assert_eq!(UnicodeWidthStr::width(line.as_str()), 60);
    assert_eq!(cursor, Some((9, 0)));
}

#[test]
fn search_line_hint_is_rendered_in_the_muted_border_color() {
    for (cols, hint) in [
        (100, "  Ctrl+Enter new tab  Esc "),
        (40, "  Ctrl+Enter  Esc "),
        (20, "  Esc "),
    ] {
        let (line, _) = render_search_line(cols, "", true);

        assert!(
            line.contains(&format!("\u{1b}[38;5;240m{hint}\u{1b}[0m")),
            "width {cols}: {line:?}"
        );
    }
}

#[test]
fn search_line_stays_width_safe_and_cursor_bound_at_every_small_width() {
    for cols in 1..=120 {
        let (line, cursor) = render_search_line(cols, "查询query", true);

        assert_eq!(
            UnicodeWidthStr::width(strip_sgr(&line).as_str()),
            cols,
            "width {cols}"
        );
        assert!(
            cursor.is_none_or(|(column, row)| column <= cols && row == 0),
            "width {cols}, cursor {cursor:?}"
        );
    }
}

#[test]
fn zero_width_search_has_no_output_or_cursor() {
    let (line, cursor) = render_search_line(0, "query", true);

    assert!(line.is_empty());
    assert_eq!(cursor, None);
}

#[test]
fn custom_template_controls_field_order_and_visible_values() {
    let config = Config::from_map(&BTreeMap::from([(
        "row_format".into(),
        "{icon} {session}/{tab} #{pane_id} {command} [{dir}] {title}".into(),
    )]));
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex --full-auto")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        80,
        12,
        "",
        &config,
        1,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("work/code #1 codex --full-auto [src/ui] src renderer"));
    assert!(!frame.lines[2].contains("fold"));
}

#[test]
fn default_row_shows_all_pane_information() {
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex --full-auto")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        120,
        12,
        "",
        &Config::default(),
        1,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("src renderer"));
    assert!(frame.lines[2].contains("src/ui"));
    assert!(frame.lines[2].contains("11-1"));
    assert!(frame.lines[2].contains("codex --full-auto"));
}

#[test]
fn default_row_uses_compact_tab_and_pane_id_before_the_title() {
    let rows = Rows {
        rows: vec![row(1, "src renderer", "src/ui", "codex --full-auto")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        120,
        12,
        "",
        &Config::default(),
        1,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("11-1  src renderer"));
    assert!(!frame.lines[2].contains("work/code"));
    assert!(!frame.lines[2].contains("#1"));
}

#[test]
fn long_command_does_not_hide_the_pane_id() {
    let rows = Rows {
        rows: vec![row(
            1,
            "app",
            ".",
            "codex --full-auto --model gpt-5 --sandbox danger --very-long-argument",
        )],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        80,
        12,
        "",
        &Config::default(),
        1,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );

    assert!(frame.lines[2].contains("11-1"));
    assert!(frame.lines[2].contains("codex"));
}

#[test]
fn full_command_is_only_ellipsized_when_the_row_width_is_insufficient() {
    let command = "codex --full-auto --model gpt-5 --sandbox danger";
    let rows = Rows {
        rows: vec![row(1, "app", ".", command)],
    };
    let wide = render_frame(
        &rows,
        Some(0),
        120,
        12,
        "",
        &Config::default(),
        1,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );
    let narrow = render_frame(
        &rows,
        Some(0),
        52,
        12,
        "",
        &Config::default(),
        1,
        RenderOptions {
            color: false,
            ..RenderOptions::default()
        },
    );

    assert!(wide.lines[2].contains(command));
    assert!(narrow.lines[2].contains("codex --full-auto"));
    assert!(!narrow.lines[2].contains(command));
    assert!(narrow.lines[2].contains('…'));
}

#[test]
fn every_line_is_unicode_width_safe_and_the_frame_is_exact_height() {
    let config = Config::default();
    let rows = Rows {
        rows: vec![row(
            1,
            "很长的项目标题abcdefghijklmnopqrstuvwxyz",
            "很长的目录abcdefghijklmnopqrstuvwxyz",
            "codex",
        )],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        24,
        9,
        "很长的查询abcdefghijklmnopqrstuvwxyz",
        &config,
        0,
        RenderOptions::default(),
    );

    assert_eq!(frame.lines.len(), 9);
    assert_line_widths(&frame.lines, 24);
    assert_eq!(frame.cursor.map(|(_, row)| row), Some(0));
}

#[test]
fn viewport_rendering_stays_bounded_for_ten_thousand_rows() {
    let config = Config::default();
    let rows = Rows {
        rows: (1..=10_000)
            .map(|id| row(id, &format!("pane-{id}"), ".", "zsh"))
            .collect(),
    };
    let frame = render_frame(
        &rows,
        Some(9_999),
        80,
        10,
        "",
        &config,
        0,
        RenderOptions::default(),
    );
    let rendered = frame.lines.join("\n");

    assert!(rendered.contains("pane-10000"));
    assert!(!rendered.contains("pane-1 "));
    assert!(!rendered.contains("pane-5000"));
    assert_line_widths(&frame.lines, 80);
}

#[test]
fn render_frame_reports_the_visible_row_range() {
    let rows = Rows {
        rows: (1..=20)
            .map(|id| row(id, &format!("pane-{id}"), ".", "zsh"))
            .collect(),
    };
    let frame = render_frame(
        &rows,
        Some(9),
        80,
        6,
        "",
        &Config::default(),
        0,
        RenderOptions::default(),
    );

    assert_eq!(frame.viewport_start, 7);
    assert_eq!(frame.viewport_end, 10);
}

#[test]
fn progress_states_have_distinct_semantic_colors() {
    let mut idle = row(1, "idle", ".", "zsh");
    idle.progress = ProgressState::Idle;
    let mut exited = row(2, "exited", ".", "vim");
    exited.progress = ProgressState::Exited;
    let mut unknown = row(3, "unknown", ".", "?");
    unknown.progress = ProgressState::Unknown;
    let rows = Rows {
        rows: vec![idle, exited, unknown],
    };
    let frame = render_frame(
        &rows,
        None,
        80,
        12,
        "",
        &Config::default(),
        0,
        RenderOptions::default(),
    );
    let rendered = frame.lines.join("\n");

    assert!(rendered.contains("\u{1b}[38;5;46m"));
    assert!(rendered.contains("\u{1b}[38;5;196m"));
    assert!(rendered.contains("\u{1b}[38;5;240m"));
}

#[test]
fn one_cell_terminals_still_receive_width_safe_output() {
    let rows = Rows {
        rows: vec![row(1, "wide 项目", "src", "zsh")],
    };
    let frame = render_frame(
        &rows,
        Some(0),
        1,
        1,
        "query",
        &Config::default(),
        0,
        RenderOptions::default(),
    );

    assert_eq!(frame.lines.len(), 1);
    assert_line_widths(&frame.lines, 1);
}

fn render_search_line(cols: usize, query: &str, color: bool) -> (String, Option<(usize, usize)>) {
    let frame = render_frame(
        &Rows::default(),
        None,
        cols,
        4,
        query,
        &Config::default(),
        0,
        RenderOptions {
            color,
            ..RenderOptions::default()
        },
    );
    (
        frame.lines.first().cloned().unwrap_or_default(),
        frame.cursor,
    )
}

fn assert_line_widths(lines: &[String], expected: usize) {
    for line in lines {
        let visible = strip_sgr(line);
        assert_eq!(UnicodeWidthStr::width(visible.as_str()), expected);
    }
}

fn strip_sgr(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            output.push(character);
        }
    }
    output
}
