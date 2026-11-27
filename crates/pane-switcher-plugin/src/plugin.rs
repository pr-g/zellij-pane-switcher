use std::collections::BTreeMap;
use std::io::Write;

use pane_switcher_core::app::Finder;
use pane_switcher_core::config::Config;
use pane_switcher_core::render::{render_frame, RenderOptions};
use pane_switcher_core::style::ThemePalette;
use pane_switcher_core::template::TemplateField;
use zellij_tile::prelude::*;

use crate::adapter::{
    configured_floating_coordinates, dead_session_inputs, key_intent, mouse_intent,
    navigation_action, pane_manifest_input, session_input, tab_inputs, theme_palette, InputIntent,
    MouseIntent, NavigationAction,
};

const MAX_PANE_QUERY_FAILURES: u8 = 2;
const PLUGIN_PANE_NAME: &str = "pane-switcher";

fn new_tab_pane_title(query: &str) -> Option<String> {
    if query.trim().is_empty() {
        return None;
    }
    Some(
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
}

trait TabCreationHost {
    fn open_default_tab(&mut self) -> Option<usize>;
    fn focused_pane(&mut self) -> Result<(usize, PaneId), String>;
    fn rename_pane(&mut self, pane_id: PaneId, title: &str);
    fn close_plugin(&mut self);
}

struct ZellijTabCreationHost;

impl TabCreationHost for ZellijTabCreationHost {
    fn open_default_tab(&mut self) -> Option<usize> {
        new_tab(None::<String>, None::<String>)
    }

    fn focused_pane(&mut self) -> Result<(usize, PaneId), String> {
        get_focused_pane_info()
    }

    fn rename_pane(&mut self, pane_id: PaneId, title: &str) {
        rename_pane_with_id(pane_id, title);
    }

    fn close_plugin(&mut self) {
        close_self();
    }
}

fn create_tab_with_pane_title(query: &str, host: &mut impl TabCreationHost) -> bool {
    let Some(title) = new_tab_pane_title(query) else {
        return false;
    };
    if host.open_default_tab().is_none() {
        return false;
    }
    if let Ok((_, pane_id)) = host.focused_pane() {
        host.rename_pane(pane_id, &title);
    }
    host.close_plugin();
    true
}

#[derive(Default)]
pub struct PaneSwitcherPlugin {
    finder: Finder,
    permissions_granted: bool,
    visible: bool,
    timer_scheduled: bool,
    last_reported_cursor: Option<Option<(usize, usize)>>,
    pane_query_failures: BTreeMap<(String, u32), PaneQueryFailures>,
    theme: ThemePalette,
    viewport_start: usize,
    viewport_end: usize,
    hovered_slot: Option<usize>,
    floating_size_applied: bool,
}

#[derive(Default)]
struct PaneQueryFailures {
    command: u8,
    cwd: u8,
}

impl ZellijPlugin for PaneSwitcherPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        let config = Config::from_map(&configuration);
        self.finder = Finder::new(config);
        self.visible = true;

        subscribe(&[
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::SessionUpdate,
            EventType::CommandChanged,
            EventType::CwdChanged,
            EventType::Key,
            EventType::PastedText,
            EventType::Mouse,
            EventType::Timer,
            EventType::PermissionRequestResult,
            EventType::Visible,
            EventType::BeforeClose,
            EventType::ModeUpdate,
        ]);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        if self.should_report_cursor(None) {
            show_cursor(None);
        }
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                self.permissions_granted = true;
                self.name_plugin_pane();
                self.apply_configured_floating_size();
                self.query_current_session_values();
                self.schedule_spinner();
                true
            }
            Event::PermissionRequestResult(PermissionStatus::Denied) => {
                self.permissions_granted = false;
                true
            }
            Event::SessionUpdate(sessions, resurrectable) => {
                let snapshot = sessions.iter().map(session_input).collect::<Vec<_>>();
                let changed = self
                    .finder
                    .world_mut()
                    .apply_snapshot(snapshot, dead_session_inputs(&resurrectable));
                if changed {
                    self.finder.tree_changed();
                }
                changed | self.query_current_session_values()
            }
            Event::TabUpdate(tabs) => {
                let changed = self
                    .finder
                    .world_mut()
                    .update_current_tabs(tab_inputs(&tabs));
                if changed {
                    self.finder.tree_changed();
                }
                changed
            }
            Event::PaneUpdate(manifest) => {
                let changed = self
                    .finder
                    .world_mut()
                    .update_current_panes(pane_manifest_input(&manifest));
                if changed {
                    self.finder.tree_changed();
                }
                changed | self.query_current_session_values()
            }
            Event::CommandChanged(pane_id, command, is_foreground, _clients) => {
                if let (PaneId::Terminal(pane_id), Some(session)) = (
                    pane_id,
                    self.finder
                        .world()
                        .current_session_name()
                        .map(str::to_owned),
                ) {
                    let changed = self.finder.set_command_with_foreground(
                        &session,
                        pane_id,
                        command,
                        is_foreground,
                    );
                    self.schedule_spinner();
                    changed
                } else {
                    false
                }
            }
            Event::CwdChanged(pane_id, cwd, _clients) => {
                if let (PaneId::Terminal(pane_id), Some(session)) = (
                    pane_id,
                    self.finder
                        .world()
                        .current_session_name()
                        .map(str::to_owned),
                ) {
                    self.finder.set_cwd(&session, pane_id, cwd)
                } else {
                    false
                }
            }
            Event::Key(key) => self.on_key(key),
            Event::PastedText(text) => {
                self.finder.insert_str(&text);
                true
            }
            Event::Mouse(mouse) => self.on_mouse(mouse),
            Event::Timer(_) => {
                self.timer_scheduled = false;
                if self.visible && self.finder.has_running_visible_pane() {
                    self.finder.advance_spinner();
                    self.arm_spinner();
                    true
                } else {
                    false
                }
            }
            Event::Visible(visible) => {
                let changed = self.visible != visible;
                self.visible = visible;
                if visible {
                    self.query_current_session_values();
                    self.schedule_spinner();
                } else if self.should_report_cursor(None) {
                    show_cursor(None);
                }
                changed
            }
            Event::BeforeClose => {
                if self.should_report_cursor(None) {
                    show_cursor(None);
                }
                false
            }
            Event::ModeUpdate(mode) => {
                self.theme = theme_palette(&mode.style);
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.finder.refresh();
        let render_cols = cols.saturating_sub(1).max(1);
        let selected = self.finder.selection().index();
        let query = self.finder.query().to_owned();
        let ticks = self.finder.spinner_tick();
        let frame = render_frame(
            self.finder.rows_ref(),
            Some(selected),
            render_cols,
            rows,
            &query,
            self.finder.config(),
            ticks,
            RenderOptions {
                color: true,
                palette: self.theme,
                hovered_slot: self.hovered_slot,
            },
        );
        if self.should_report_cursor(frame.cursor) {
            show_cursor(frame.cursor);
        }
        self.viewport_start = frame.viewport_start;
        self.viewport_end = frame.viewport_end;
        let mut stdout = std::io::stdout().lock();
        let _ = write!(stdout, "\u{1b}[2J\u{1b}[H{}", frame.lines.join("\r\n"));
        let _ = stdout.flush();
    }
}

impl PaneSwitcherPlugin {
    fn should_report_cursor(&mut self, cursor: Option<(usize, usize)>) -> bool {
        if self.last_reported_cursor == Some(cursor) {
            return false;
        }
        self.last_reported_cursor = Some(cursor);
        true
    }

    fn name_plugin_pane(&self) {
        let plugin_id = get_plugin_ids().plugin_id;
        rename_plugin_pane(plugin_id, PLUGIN_PANE_NAME);
    }

    fn apply_configured_floating_size(&mut self) {
        if self.floating_size_applied {
            return;
        }
        let Some(coordinates) = configured_floating_coordinates(self.finder.config()) else {
            return;
        };
        let plugin_id = get_plugin_ids().plugin_id;
        change_floating_panes_coordinates(vec![(PaneId::Plugin(plugin_id), coordinates)]);
        self.floating_size_applied = true;
    }

    fn on_key(&mut self, key: KeyWithModifier) -> bool {
        match key_intent(&key) {
            InputIntent::Insert(character) => self.finder.insert_char(character),
            InputIntent::Backspace => self.finder.backspace(),
            InputIntent::DeleteWord => self.finder.delete_word(),
            InputIntent::Clear => self.finder.clear_query(),
            InputIntent::Up => self.finder.selection_up(),
            InputIntent::Down => self.finder.selection_down(),
            InputIntent::PageUp => self.finder.page_up(10),
            InputIntent::PageDown => self.finder.page_down(10),
            InputIntent::Enter => {
                self.navigate();
                return true;
            }
            InputIntent::CreateTab => {
                let mut host = ZellijTabCreationHost;
                return create_tab_with_pane_title(self.finder.query(), &mut host);
            }
            InputIntent::Close => {
                close_self();
                return true;
            }
            InputIntent::Ignored => return false,
        }
        true
    }

    fn on_mouse(&mut self, mouse: Mouse) -> bool {
        match mouse_intent(&mouse, self.viewport_start, self.viewport_end) {
            MouseIntent::ActivateRow(index) => {
                if !self.finder.select_index(index) {
                    return false;
                }
                self.navigate();
                true
            }
            MouseIntent::Up => {
                self.finder.selection_up();
                true
            }
            MouseIntent::Down => {
                self.finder.selection_down();
                true
            }
            MouseIntent::HoverSlot(slot) => {
                let changed = self.hovered_slot != slot;
                self.hovered_slot = slot;
                changed
            }
            MouseIntent::Ignored => false,
        }
    }

    fn navigate(&mut self) {
        let Some(target) = self.finder.navigation_target() else {
            return;
        };
        let action = navigation_action(&target, self.finder.world().current_session_name());
        match action {
            NavigationAction::SwitchSession { session } => switch_session(Some(&session)),
            NavigationAction::SwitchCurrentTab { tab_position } => {
                switch_tab_to((tab_position + 1) as u32)
            }
            NavigationAction::SwitchOtherTab {
                session,
                tab_position,
            } => switch_session_with_focus(&session, Some(tab_position), None),
            NavigationAction::FocusCurrentPane { pane_id } => match pane_id {
                PaneId::Terminal(pane_id) => {
                    focus_terminal_pane(pane_id, true, false);
                }
                PaneId::Plugin(pane_id) => focus_plugin_pane(pane_id, true, false),
            },
            NavigationAction::SwitchSessionWithPane {
                session,
                tab_position,
                pane_id,
            } => switch_session_with_focus(&session, Some(tab_position), Some((pane_id, false))),
        }
        close_self();
    }

    fn query_current_session_values(&mut self) -> bool {
        if !self.permissions_granted {
            return false;
        }
        let Some(session_name) = self
            .finder
            .world()
            .current_session_name()
            .map(str::to_owned)
        else {
            return false;
        };
        let pane_ids = self
            .finder
            .world()
            .live_session(&session_name)
            .map(|session| {
                session
                    .tabs
                    .iter()
                    .flat_map(|tab| tab.panes.iter().map(|pane| pane.id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        self.update_missing_pane_values(
            &session_name,
            pane_ids,
            |pane_id| get_pane_running_command(PaneId::Terminal(pane_id)).ok(),
            |pane_id| get_pane_cwd(PaneId::Terminal(pane_id)).ok(),
        )
    }

    fn update_missing_pane_values(
        &mut self,
        session_name: &str,
        pane_ids: Vec<u32>,
        mut query_command: impl FnMut(u32) -> Option<Vec<String>>,
        mut query_cwd: impl FnMut(u32) -> Option<std::path::PathBuf>,
    ) -> bool {
        let mut changed = false;
        let template = &self.finder.config().row_format;
        let command_visible = template.contains(TemplateField::Command)
            || template.contains(TemplateField::Progress)
            || template.contains(TemplateField::Icon);
        let cwd_visible = template.contains(TemplateField::Dir);
        for pane_id in pane_ids {
            let failures = self
                .pane_query_failures
                .entry((session_name.to_owned(), pane_id))
                .or_default();
            if command_visible
                && self
                    .finder
                    .values()
                    .command(session_name, pane_id)
                    .is_none()
                && failures.command < MAX_PANE_QUERY_FAILURES
            {
                if let Some(command) = query_command(pane_id) {
                    changed |= self.finder.set_command(session_name, pane_id, command);
                    failures.command = 0;
                } else {
                    failures.command += 1;
                }
            }
            if cwd_visible
                && self.finder.values().cwd(session_name, pane_id).is_none()
                && failures.cwd < MAX_PANE_QUERY_FAILURES
            {
                if let Some(cwd) = query_cwd(pane_id) {
                    changed |= self.finder.set_cwd(session_name, pane_id, cwd);
                    failures.cwd = 0;
                } else {
                    failures.cwd += 1;
                }
            }
        }
        changed
    }

    fn schedule_spinner(&mut self) {
        if self.timer_scheduled
            || !self.visible
            || !self.permissions_granted
            || !self.finder.has_running_visible_pane()
        {
            return;
        }
        self.arm_spinner();
    }

    fn arm_spinner(&mut self) {
        if self.timer_scheduled || !self.visible || !self.permissions_granted {
            return;
        }
        let fps = self.finder.config().spinner_fps.max(1) as f64;
        set_timeout(1.0 / fps);
        self.timer_scheduled = true;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use pane_switcher_core::app::Finder;
    use pane_switcher_core::config::Config;
    use pane_switcher_core::style::ColorSpec;
    use zellij_tile::prelude::{
        BareKey, Event, KeyWithModifier, ModeInfo, Mouse, PaletteColor, PaneId, PaneManifest,
        SessionInfo, ZellijPlugin,
    };

    use super::{
        create_tab_with_pane_title, new_tab_pane_title, PaneSwitcherPlugin, TabCreationHost,
    };

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum FakeTabCreationCall {
        OpenTab,
        QueryFocusedPane,
        RenamePane(PaneId, String),
        ClosePlugin,
    }

    struct FakeTabCreationHost {
        calls: Vec<FakeTabCreationCall>,
        open_result: Option<usize>,
        focused_result: Result<(usize, PaneId), String>,
    }

    impl FakeTabCreationHost {
        fn successful() -> Self {
            Self {
                calls: Vec::new(),
                open_result: Some(42),
                focused_result: Ok((1, PaneId::Terminal(7))),
            }
        }
    }

    impl TabCreationHost for FakeTabCreationHost {
        fn open_default_tab(&mut self) -> Option<usize> {
            self.calls.push(FakeTabCreationCall::OpenTab);
            self.open_result
        }

        fn focused_pane(&mut self) -> Result<(usize, PaneId), String> {
            self.calls.push(FakeTabCreationCall::QueryFocusedPane);
            self.focused_result.clone()
        }

        fn rename_pane(&mut self, pane_id: PaneId, title: &str) {
            self.calls
                .push(FakeTabCreationCall::RenamePane(pane_id, title.to_owned()));
        }

        fn close_plugin(&mut self) {
            self.calls.push(FakeTabCreationCall::ClosePlugin);
        }
    }

    #[test]
    fn new_tab_pane_title_preserves_the_query_text() {
        assert_eq!(
            new_tab_pane_title(" build api "),
            Some(" build api ".into())
        );
    }

    #[test]
    fn new_tab_pane_title_preserves_unicode_text() {
        assert_eq!(
            new_tab_pane_title("构建 API 🚀"),
            Some("构建 API 🚀".into())
        );
    }

    #[test]
    fn new_tab_pane_title_replaces_control_characters_with_spaces() {
        assert_eq!(
            new_tab_pane_title("first\nsecond\tthird\r\u{1b}[31m"),
            Some("first second third  [31m".into())
        );
    }

    #[test]
    fn new_tab_pane_title_rejects_every_blank_whitespace_form() {
        for query in ["", " ", "\t\n", "\u{3000}"] {
            assert_eq!(new_tab_pane_title(query), None, "query {query:?}");
        }
    }

    #[test]
    fn creating_a_tab_dispatches_the_host_calls_in_order() {
        let mut host = FakeTabCreationHost::successful();

        assert!(create_tab_with_pane_title(" build api ", &mut host));

        assert_eq!(
            host.calls,
            vec![
                FakeTabCreationCall::OpenTab,
                FakeTabCreationCall::QueryFocusedPane,
                FakeTabCreationCall::RenamePane(PaneId::Terminal(7), " build api ".into()),
                FakeTabCreationCall::ClosePlugin,
            ]
        );
    }

    #[test]
    fn blank_queries_do_not_call_the_host() {
        for query in ["", " ", "\t", "\n\r", "\u{3000}"] {
            let mut host = FakeTabCreationHost::successful();

            assert!(!create_tab_with_pane_title(query, &mut host), "{query:?}");
            assert!(host.calls.is_empty(), "{query:?}: {:?}", host.calls);
        }
    }

    #[test]
    fn creation_forwards_the_normalized_single_line_title() {
        let mut host = FakeTabCreationHost::successful();

        assert!(create_tab_with_pane_title(
            "first\nsecond\tthird",
            &mut host
        ));

        assert_eq!(
            host.calls[2],
            FakeTabCreationCall::RenamePane(PaneId::Terminal(7), "first second third".into())
        );
    }

    #[test]
    fn failed_tab_creation_stops_before_querying_or_closing() {
        let mut host = FakeTabCreationHost {
            open_result: None,
            ..FakeTabCreationHost::successful()
        };

        assert!(!create_tab_with_pane_title("build", &mut host));

        assert_eq!(host.calls, vec![FakeTabCreationCall::OpenTab]);
    }

    #[test]
    fn focused_pane_failure_still_closes_the_open_plugin() {
        let mut host = FakeTabCreationHost {
            focused_result: Err("pane unavailable".into()),
            ..FakeTabCreationHost::successful()
        };

        assert!(create_tab_with_pane_title("build", &mut host));

        assert_eq!(
            host.calls,
            vec![
                FakeTabCreationCall::OpenTab,
                FakeTabCreationCall::QueryFocusedPane,
                FakeTabCreationCall::ClosePlugin,
            ]
        );
    }

    #[test]
    fn unicode_query_is_forwarded_unchanged_to_the_pane_rename() {
        let mut host = FakeTabCreationHost::successful();

        assert!(create_tab_with_pane_title("构建 API 🚀", &mut host));

        assert_eq!(
            host.calls[2],
            FakeTabCreationCall::RenamePane(PaneId::Terminal(7), "构建 API 🚀".into())
        );
    }

    #[test]
    fn cursor_is_reported_only_when_its_position_changes() {
        let mut plugin = PaneSwitcherPlugin::default();

        assert!(plugin.should_report_cursor(None));
        assert!(!plugin.should_report_cursor(None));
        assert!(plugin.should_report_cursor(Some((4, 2))));
        assert!(!plugin.should_report_cursor(Some((4, 2))));
        assert!(plugin.should_report_cursor(Some((5, 2))));
    }

    #[test]
    fn duplicate_structural_events_do_not_request_another_render() {
        let mut plugin = PaneSwitcherPlugin::default();
        let session = SessionInfo {
            name: "work".into(),
            is_current_session: true,
            ..SessionInfo::default()
        };

        assert!(plugin.update(Event::SessionUpdate(vec![session.clone()], Vec::new())));
        plugin.should_report_cursor(Some((4, 2)));

        assert!(!plugin.update(Event::SessionUpdate(vec![session], Vec::new())));
        assert!(!plugin.update(Event::TabUpdate(Vec::new())));
        assert!(!plugin.update(Event::PaneUpdate(PaneManifest {
            panes: HashMap::new(),
        })));
    }

    #[test]
    fn failed_pane_queries_are_retried_until_both_values_are_cached() {
        let mut plugin = PaneSwitcherPlugin::default();

        assert!(!plugin.update_missing_pane_values(
            "work",
            vec![7],
            |_| -> Option<Vec<String>> { None },
            |_| -> Option<PathBuf> { None },
        ));
        assert!(plugin.update_missing_pane_values(
            "work",
            vec![7],
            |_| Some(vec!["cargo".into(), "test".into()]),
            |_| Some(PathBuf::from("/tmp/work")),
        ));
        assert!(!plugin.update_missing_pane_values(
            "work",
            vec![7],
            |_| -> Option<Vec<String>> { panic!("cached command should not be queried") },
            |_| -> Option<PathBuf> { panic!("cached cwd should not be queried") },
        ));
    }

    #[test]
    fn persistently_failing_pane_queries_stop_retrying() {
        let mut plugin = PaneSwitcherPlugin::default();
        let mut command_queries = 0;
        let mut cwd_queries = 0;

        for _ in 0..10 {
            plugin.update_missing_pane_values(
                "work",
                vec![7],
                |_| {
                    command_queries += 1;
                    None
                },
                |_| {
                    cwd_queries += 1;
                    None
                },
            );
        }

        assert_eq!(command_queries, 2);
        assert_eq!(cwd_queries, 2);
    }

    #[test]
    fn hidden_cwd_does_not_trigger_a_host_query() {
        let config = Config::from_map(&std::collections::BTreeMap::from([(
            "row_format".into(),
            "{title}".into(),
        )]));
        let mut plugin = PaneSwitcherPlugin {
            finder: Finder::new(config),
            ..PaneSwitcherPlugin::default()
        };
        let mut command_queries = 0;
        let mut cwd_queries = 0;

        plugin.update_missing_pane_values(
            "work",
            vec![7],
            |_| {
                command_queries += 1;
                Some(vec!["zsh".into()])
            },
            |_| {
                cwd_queries += 1;
                Some(PathBuf::from("/tmp/work"))
            },
        );

        assert_eq!(command_queries, 0);
        assert_eq!(cwd_queries, 0);
    }

    #[test]
    fn left_and_right_keys_do_not_request_render() {
        let mut plugin = PaneSwitcherPlugin::default();

        assert!(!plugin.on_key(KeyWithModifier::new(BareKey::Left)));
        assert!(!plugin.on_key(KeyWithModifier::new(BareKey::Right)));
    }

    #[test]
    fn mouse_hover_tracks_the_visual_row_without_moving_selection() {
        let mut plugin = PaneSwitcherPlugin::default();
        assert!(plugin.on_mouse(Mouse::Hover(3, 10)));
        assert_eq!(plugin.hovered_slot, Some(1));
        assert_eq!(plugin.finder.selection().index(), 0);
        assert!(!plugin.on_mouse(Mouse::Hover(3, 20)));

        assert!(plugin.on_mouse(Mouse::Hover(1, 20)));
        assert_eq!(plugin.hovered_slot, None);
        assert_eq!(plugin.finder.selection().index(), 0);
    }

    #[test]
    fn single_character_ime_commits_from_key_and_paste_are_preserved() {
        let mut plugin = PaneSwitcherPlugin::default();

        assert!(plugin.update(Event::Key(KeyWithModifier::new(BareKey::Char('的')))));
        assert_eq!(plugin.finder.query(), "的");

        assert!(plugin.update(Event::PastedText("的".into())));
        assert_eq!(plugin.finder.query(), "的的");
    }

    #[test]
    fn mixed_ime_input_sources_preserve_order_and_backspace_removes_one_cjk_character() {
        let mut plugin = PaneSwitcherPlugin::default();

        assert!(plugin.update(Event::PastedText("输".into())));
        assert!(plugin.update(Event::Key(KeyWithModifier::new(BareKey::Char('入')))));
        assert!(plugin.update(Event::PastedText("shuru".into())));
        assert_eq!(plugin.finder.query(), "输入shuru");

        assert!(plugin.update(Event::Key(KeyWithModifier::new(BareKey::Backspace))));
        assert_eq!(plugin.finder.query(), "输入shur");

        plugin.finder.rows();
        assert_eq!(plugin.finder.rebuild_count(), 1);
    }

    #[test]
    fn mode_update_applies_the_active_zellij_theme_palette() {
        let mut plugin = PaneSwitcherPlugin::default();
        let mut mode = ModeInfo::default();
        mode.style.colors.text_unselected.emphasis_2 = PaletteColor::Rgb((1, 2, 3));

        assert!(plugin.update(Event::ModeUpdate(mode)));
        assert_eq!(plugin.theme.green, ColorSpec::Rgb((1, 2, 3)));
    }
}
