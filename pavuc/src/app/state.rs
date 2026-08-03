//! Application state and input handling for the pavuc TUI.

use libpavuc::{PulseClient, PulseState, volume};
use ratatui::crossterm::event::KeyCode;

/// The five tabs, matching pavucontrol exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Playback streams (sink inputs).
    Playback,
    /// Recording streams (source outputs).
    Recording,
    /// Output devices (sinks).
    Output,
    /// Input devices (sources).
    Input,
    /// Cards and their profiles.
    Configuration,
}

impl Tab {
    /// All tabs in display order.
    pub const ALL: [Tab; 5] = [
        Tab::Playback,
        Tab::Recording,
        Tab::Output,
        Tab::Input,
        Tab::Configuration,
    ];

    /// Title shown in the tab bar.
    pub fn title(self) -> &'static str {
        match self {
            Tab::Playback => "Playback",
            Tab::Recording => "Recording",
            Tab::Output => "Output Devices",
            Tab::Input => "Input Devices",
            Tab::Configuration => "Configuration",
        }
    }
}

/// Volume step applied with the arrow / h-l keys.
const COARSE_STEP: i64 = 5;
/// Fine volume step applied with the `<` / `>` keys.
const FINE_STEP: i64 = 1;

/// A modal selection popup (device routing, port, or profile picker).
pub struct Popup {
    /// What the popup selects and which object it applies to.
    pub kind: PopupKind,
    /// Heading shown at the top of the popup.
    pub title: String,
    /// Selectable entries.
    pub items: Vec<PopupItem>,
    /// Currently highlighted entry.
    pub selected: usize,
}

/// One selectable entry in a [`Popup`].
pub struct PopupItem {
    /// Text shown to the user.
    pub label: String,
    /// Opaque value (port/profile name, or stringified device index) applied on confirm.
    pub value: String,
    /// Whether the entry is currently available (greyed out otherwise).
    pub available: bool,
}

/// What a [`Popup`] configures.
pub enum PopupKind {
    /// Move a playback stream to the sink with the chosen index.
    MoveSinkInput(u32),
    /// Move a recording stream to the source with the chosen index.
    MoveSourceOutput(u32),
    /// Set the active port of the sink with the given index.
    SinkPort(u32),
    /// Set the active port of the source with the given index.
    SourcePort(u32),
    /// Set the active profile of the card with the given index.
    CardProfile(u32),
}

/// The full application state.
#[derive(Default)]
pub struct App {
    /// Latest snapshot of the server.
    pub state: PulseState,
    /// Index into [`Tab::ALL`].
    pub tab: usize,
    /// Selected item per tab.
    pub selected: [usize; 5],
    /// Active modal popup, if any.
    pub popup: Option<Popup>,
    /// Whether to quit on the next loop.
    pub should_quit: bool,
    /// Transient status / hint message.
    pub status: String,
}

impl App {
    /// The currently active tab.
    pub fn current_tab(&self) -> Tab {
        Tab::ALL[self.tab]
    }

    /// Replaces the cached snapshot and clamps selections to it.
    pub fn update_state(&mut self, state: PulseState) {
        self.state = state;
        for (i, tab) in Tab::ALL.iter().enumerate() {
            let count = self.item_count(*tab);
            if count == 0 {
                self.selected[i] = 0;
            } else if self.selected[i] >= count {
                self.selected[i] = count - 1;
            }
        }
        if let Some(popup) = &mut self.popup
            && popup.selected >= popup.items.len()
        {
            popup.selected = popup.items.len().saturating_sub(1);
        }
    }

    /// Number of rows shown in the given tab.
    pub fn item_count(&self, tab: Tab) -> usize {
        match tab {
            Tab::Playback => self.state.sink_inputs.len(),
            Tab::Recording => self.state.source_outputs.len(),
            Tab::Output => self.state.sinks.len(),
            Tab::Input => self.state.sources.len(),
            Tab::Configuration => self.state.cards.len(),
        }
    }

    /// Selection index for the current tab.
    pub fn selection(&self) -> usize {
        self.selected[self.tab]
    }

    fn set_selection(&mut self, value: usize) {
        self.selected[self.tab] = value;
    }

    /// Handles a key press, issuing commands to `client` as needed.
    pub fn handle_key(&mut self, code: KeyCode, client: &mut PulseClient) {
        if self.popup.is_some() {
            self.handle_popup_key(code, client);
            return;
        }

        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('1') => self.tab = 0,
            KeyCode::Char('2') => self.tab = 1,
            KeyCode::Char('3') => self.tab = 2,
            KeyCode::Char('4') => self.tab = 3,
            KeyCode::Char('5') => self.tab = 4,
            KeyCode::Tab => self.tab = (self.tab + 1) % Tab::ALL.len(),
            KeyCode::BackTab => self.tab = (self.tab + Tab::ALL.len() - 1) % Tab::ALL.len(),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Left | KeyCode::Char('h') => self.adjust_volume(-COARSE_STEP, client),
            KeyCode::Right | KeyCode::Char('l') => self.adjust_volume(COARSE_STEP, client),
            KeyCode::Char('<') | KeyCode::Char(',') => self.adjust_volume(-FINE_STEP, client),
            KeyCode::Char('>') | KeyCode::Char('.') => self.adjust_volume(FINE_STEP, client),
            KeyCode::Char('m') => self.toggle_mute(client),
            KeyCode::Char('d') => self.set_default(client),
            KeyCode::Char('x') => self.kill_stream(client),
            KeyCode::Enter => self.open_popup(),
            _ => {}
        }
    }

    fn move_selection(&mut self, delta: i64) {
        let count = self.item_count(self.current_tab());
        if count == 0 {
            return;
        }
        let current = self.selection() as i64;
        let next = (current + delta).rem_euclid(count as i64) as usize;
        self.set_selection(next);
    }

    fn adjust_volume(&mut self, delta: i64, client: &mut PulseClient) {
        let sel = self.selection();
        match self.current_tab() {
            Tab::Playback => {
                if let Some(s) = self.state.sink_inputs.get(sel) {
                    if !s.volume_writable {
                        self.status = "This stream's volume is not adjustable".to_string();
                        return;
                    }
                    let target = volume::clamp_percent(i64::from(s.volume_percent()) + delta);
                    client.set_sink_input_volume(s.index, s.channels, target);
                }
            }
            Tab::Recording => {
                if let Some(s) = self.state.source_outputs.get(sel) {
                    if !s.volume_writable {
                        self.status = "This stream's volume is not adjustable".to_string();
                        return;
                    }
                    let target = volume::clamp_percent(i64::from(s.volume_percent()) + delta);
                    client.set_source_output_volume(s.index, s.channels, target);
                }
            }
            Tab::Output => {
                if let Some(d) = self.state.sinks.get(sel) {
                    let target = volume::clamp_percent(i64::from(d.volume_percent()) + delta);
                    client.set_sink_volume(d.index, d.channels, target);
                }
            }
            Tab::Input => {
                if let Some(d) = self.state.sources.get(sel) {
                    let target = volume::clamp_percent(i64::from(d.volume_percent()) + delta);
                    client.set_source_volume(d.index, d.channels, target);
                }
            }
            Tab::Configuration => {}
        }
    }

    fn toggle_mute(&mut self, client: &mut PulseClient) {
        let sel = self.selection();
        match self.current_tab() {
            Tab::Playback => {
                if let Some(s) = self.state.sink_inputs.get(sel) {
                    client.set_sink_input_mute(s.index, !s.mute);
                }
            }
            Tab::Recording => {
                if let Some(s) = self.state.source_outputs.get(sel) {
                    client.set_source_output_mute(s.index, !s.mute);
                }
            }
            Tab::Output => {
                if let Some(d) = self.state.sinks.get(sel) {
                    client.set_sink_mute(d.index, !d.mute);
                }
            }
            Tab::Input => {
                if let Some(d) = self.state.sources.get(sel) {
                    client.set_source_mute(d.index, !d.mute);
                }
            }
            Tab::Configuration => {}
        }
    }

    fn set_default(&mut self, client: &mut PulseClient) {
        let sel = self.selection();
        match self.current_tab() {
            Tab::Output => {
                if let Some(d) = self.state.sinks.get(sel) {
                    client.set_default_sink(&d.name);
                    self.status = format!("Set {} as default output", d.description);
                }
            }
            Tab::Input => {
                if let Some(d) = self.state.sources.get(sel) {
                    client.set_default_source(&d.name);
                    self.status = format!("Set {} as default input", d.description);
                }
            }
            _ => self.status = "Set-as-default only applies to devices".to_string(),
        }
    }

    fn kill_stream(&mut self, client: &mut PulseClient) {
        let sel = self.selection();
        match self.current_tab() {
            Tab::Playback => {
                if let Some(s) = self.state.sink_inputs.get(sel) {
                    client.kill_sink_input(s.index);
                }
            }
            Tab::Recording => {
                if let Some(s) = self.state.source_outputs.get(sel) {
                    client.kill_source_output(s.index);
                }
            }
            _ => {}
        }
    }

    fn open_popup(&mut self) {
        let sel = self.selection();
        let popup = match self.current_tab() {
            Tab::Playback => self.state.sink_inputs.get(sel).map(|s| {
                let items = self
                    .state
                    .sinks
                    .iter()
                    .map(|d| PopupItem {
                        label: d.description.clone(),
                        value: d.index.to_string(),
                        available: true,
                    })
                    .collect();
                Popup {
                    kind: PopupKind::MoveSinkInput(s.index),
                    title: format!("Move \"{}\" to output:", s.name),
                    selected: self
                        .state
                        .sinks
                        .iter()
                        .position(|d| d.index == s.device)
                        .unwrap_or(0),
                    items,
                }
            }),
            Tab::Recording => self.state.source_outputs.get(sel).map(|s| {
                let items = self
                    .state
                    .sources
                    .iter()
                    .filter(|d| !d.monitor)
                    .map(|d| PopupItem {
                        label: d.description.clone(),
                        value: d.index.to_string(),
                        available: true,
                    })
                    .collect();
                Popup {
                    kind: PopupKind::MoveSourceOutput(s.index),
                    title: format!("Move \"{}\" to input:", s.name),
                    selected: 0,
                    items,
                }
            }),
            Tab::Output => self.state.sinks.get(sel).map(|d| {
                let items = d
                    .ports
                    .iter()
                    .map(|p| PopupItem {
                        label: p.description.clone(),
                        value: p.name.clone(),
                        available: p.available,
                    })
                    .collect();
                Popup {
                    kind: PopupKind::SinkPort(d.index),
                    title: format!("Port for \"{}\":", d.description),
                    selected: d
                        .ports
                        .iter()
                        .position(|p| Some(&p.name) == d.active_port.as_ref())
                        .unwrap_or(0),
                    items,
                }
            }),
            Tab::Input => self.state.sources.get(sel).map(|d| {
                let items = d
                    .ports
                    .iter()
                    .map(|p| PopupItem {
                        label: p.description.clone(),
                        value: p.name.clone(),
                        available: p.available,
                    })
                    .collect();
                Popup {
                    kind: PopupKind::SourcePort(d.index),
                    title: format!("Port for \"{}\":", d.description),
                    selected: d
                        .ports
                        .iter()
                        .position(|p| Some(&p.name) == d.active_port.as_ref())
                        .unwrap_or(0),
                    items,
                }
            }),
            Tab::Configuration => self.state.cards.get(sel).map(|c| {
                let items = c
                    .profiles
                    .iter()
                    .map(|p| PopupItem {
                        label: p.description.clone(),
                        value: p.name.clone(),
                        available: p.available,
                    })
                    .collect();
                Popup {
                    kind: PopupKind::CardProfile(c.index),
                    title: format!("Profile for \"{}\":", c.description),
                    selected: c
                        .profiles
                        .iter()
                        .position(|p| Some(&p.name) == c.active_profile.as_ref())
                        .unwrap_or(0),
                    items,
                }
            }),
        };

        if let Some(popup) = popup {
            if popup.items.is_empty() {
                self.status = "Nothing to choose here".to_string();
            } else {
                self.popup = Some(popup);
            }
        }
    }

    fn handle_popup_key(&mut self, code: KeyCode, client: &mut PulseClient) {
        let Some(popup) = &mut self.popup else {
            return;
        };
        match code {
            KeyCode::Esc | KeyCode::Char('q') => self.popup = None,
            KeyCode::Up | KeyCode::Char('k') => {
                if !popup.items.is_empty() {
                    popup.selected = (popup.selected + popup.items.len() - 1) % popup.items.len();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !popup.items.is_empty() {
                    popup.selected = (popup.selected + 1) % popup.items.len();
                }
            }
            KeyCode::Enter => {
                self.confirm_popup(client);
                self.popup = None;
            }
            _ => {}
        }
    }

    fn confirm_popup(&mut self, client: &mut PulseClient) {
        let Some(popup) = &self.popup else {
            return;
        };
        let Some(item) = popup.items.get(popup.selected) else {
            return;
        };
        match &popup.kind {
            PopupKind::MoveSinkInput(index) => {
                if let Ok(sink) = item.value.parse::<u32>() {
                    client.move_sink_input(*index, sink);
                }
            }
            PopupKind::MoveSourceOutput(index) => {
                if let Ok(source) = item.value.parse::<u32>() {
                    client.move_source_output(*index, source);
                }
            }
            PopupKind::SinkPort(index) => client.set_sink_port(*index, &item.value),
            PopupKind::SourcePort(index) => client.set_source_port(*index, &item.value),
            PopupKind::CardProfile(index) => client.set_card_profile(*index, &item.value),
        }
    }
}
