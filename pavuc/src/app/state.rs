//! Application state and input handling for the pavuc TUI.

use libpavuc::{Command, PulseState, volume};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
    /// Whether the connection to the server was lost (and is being retried).
    pub disconnected: bool,
}

impl App {
    /// The currently active tab.
    pub fn current_tab(&self) -> Tab {
        Tab::ALL[self.tab]
    }

    /// Replaces the cached snapshot, keeping each tab's selection on the same
    /// object when it moved, or clamping it when the object went away.
    pub fn update_state(&mut self, state: PulseState) {
        let selected_ids = Tab::ALL.map(|tab| self.item_id(tab, self.selected[tab as usize]));
        self.state = state;
        for (tab, id) in Tab::ALL.into_iter().zip(selected_ids) {
            let i = tab as usize;
            let count = self.item_count(tab);
            if let Some(position) = id.and_then(|id| self.position_of(tab, id)) {
                self.selected[i] = position;
            } else if count == 0 {
                self.selected[i] = 0;
            } else if self.selected[i] >= count {
                self.selected[i] = count - 1;
            }
        }
        if let Some(popup) = &self.popup
            && !self.popup_target_exists(&popup.kind)
        {
            self.popup = None;
            self.status = "Closed the menu: its stream or device went away".to_string();
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

    /// Marks the server as gone: drops the stale snapshot and any open popup.
    pub fn set_disconnected(&mut self) {
        self.disconnected = true;
        self.popup = None;
        self.update_state(PulseState::default());
        self.status = "Connection to the audio server lost, reconnecting...".to_string();
    }

    /// Marks the server as reachable again.
    pub fn set_connected(&mut self) {
        self.disconnected = false;
        self.status = "Reconnected to the audio server".to_string();
    }

    /// Shows the most recent refused command in the status line.
    pub fn report_failures(&mut self, failures: Vec<String>) {
        if let Some(last) = failures.into_iter().last() {
            let mut chars = last.chars();
            self.status = chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect())
                .unwrap_or_default();
        }
    }

    /// Whether the object a popup applies to is still in the snapshot.
    fn popup_target_exists(&self, kind: &PopupKind) -> bool {
        match *kind {
            PopupKind::MoveSinkInput(id) => self.position_of(Tab::Playback, id).is_some(),
            PopupKind::MoveSourceOutput(id) => self.position_of(Tab::Recording, id).is_some(),
            PopupKind::SinkPort(id) => self.position_of(Tab::Output, id).is_some(),
            PopupKind::SourcePort(id) => self.position_of(Tab::Input, id).is_some(),
            PopupKind::CardProfile(id) => self.position_of(Tab::Configuration, id).is_some(),
        }
    }

    /// Server index of the object at `position` in `tab`.
    fn item_id(&self, tab: Tab, position: usize) -> Option<u32> {
        match tab {
            Tab::Playback => self.state.sink_inputs.get(position).map(|s| s.index),
            Tab::Recording => self.state.source_outputs.get(position).map(|s| s.index),
            Tab::Output => self.state.sinks.get(position).map(|d| d.index),
            Tab::Input => self.state.sources.get(position).map(|d| d.index),
            Tab::Configuration => self.state.cards.get(position).map(|c| c.index),
        }
    }

    /// Position of the object with server index `id` in `tab`.
    fn position_of(&self, tab: Tab, id: u32) -> Option<usize> {
        match tab {
            Tab::Playback => self.state.sink_inputs.iter().position(|s| s.index == id),
            Tab::Recording => self.state.source_outputs.iter().position(|s| s.index == id),
            Tab::Output => self.state.sinks.iter().position(|d| d.index == id),
            Tab::Input => self.state.sources.iter().position(|d| d.index == id),
            Tab::Configuration => self.state.cards.iter().position(|c| c.index == id),
        }
    }

    /// Selection index for the current tab.
    pub fn selection(&self) -> usize {
        self.selected[self.tab]
    }

    fn set_selection(&mut self, value: usize) {
        self.selected[self.tab] = value;
    }

    /// Handles a key event (with modifiers) and returns the command to send.
    ///
    /// Ctrl+C quits from anywhere. Other Ctrl/Alt chords are ignored so they
    /// don't trigger the plain key's action (Ctrl+X must not kill a stream).
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<Command> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return None;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return None;
        }
        self.handle_key(key.code)
    }

    /// Handles a key press and returns the command to send, if any.
    pub fn handle_key(&mut self, code: KeyCode) -> Option<Command> {
        if self.popup.is_some() {
            return self.handle_popup_key(code);
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
            KeyCode::Left | KeyCode::Char('h') => return self.adjust_volume(-COARSE_STEP),
            KeyCode::Right | KeyCode::Char('l') => return self.adjust_volume(COARSE_STEP),
            KeyCode::Char('<' | ',') => return self.adjust_volume(-FINE_STEP),
            KeyCode::Char('>' | '.') => return self.adjust_volume(FINE_STEP),
            KeyCode::Char('m') => return self.toggle_mute(),
            KeyCode::Char('d') => return self.set_default(),
            KeyCode::Char('x') => return self.kill_stream(),
            KeyCode::Enter => self.open_popup(),
            _ => {}
        }
        None
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

    fn adjust_volume(&mut self, delta: i64) -> Option<Command> {
        let sel = self.selection();
        match self.current_tab() {
            Tab::Playback => {
                let s = self.state.sink_inputs.get(sel)?;
                if !s.has_volume {
                    self.status = "This stream has no volume control".to_string();
                    return None;
                }
                if !s.volume_writable {
                    self.status = "This stream's volume is not adjustable".to_string();
                    return None;
                }
                let target = volume::step_percent(s.volume_percent(), delta);
                Some(Command::SetSinkInputVolume {
                    index: s.index,
                    volume: volume::scale(&s.volume, target),
                })
            }
            Tab::Recording => {
                let s = self.state.source_outputs.get(sel)?;
                if !s.has_volume {
                    self.status = "This stream has no volume control".to_string();
                    return None;
                }
                if !s.volume_writable {
                    self.status = "This stream's volume is not adjustable".to_string();
                    return None;
                }
                let target = volume::step_percent(s.volume_percent(), delta);
                Some(Command::SetSourceOutputVolume {
                    index: s.index,
                    volume: volume::scale(&s.volume, target),
                })
            }
            Tab::Output => {
                let d = self.state.sinks.get(sel)?;
                let target = volume::step_percent(d.volume_percent(), delta);
                Some(Command::SetSinkVolume {
                    index: d.index,
                    volume: volume::scale(&d.volume, target),
                })
            }
            Tab::Input => {
                let d = self.state.sources.get(sel)?;
                let target = volume::step_percent(d.volume_percent(), delta);
                Some(Command::SetSourceVolume {
                    index: d.index,
                    volume: volume::scale(&d.volume, target),
                })
            }
            Tab::Configuration => None,
        }
    }

    fn toggle_mute(&mut self) -> Option<Command> {
        let sel = self.selection();
        match self.current_tab() {
            Tab::Playback => self
                .state
                .sink_inputs
                .get(sel)
                .map(|s| Command::SetSinkInputMute {
                    index: s.index,
                    mute: !s.mute,
                }),
            Tab::Recording => {
                self.state
                    .source_outputs
                    .get(sel)
                    .map(|s| Command::SetSourceOutputMute {
                        index: s.index,
                        mute: !s.mute,
                    })
            }
            Tab::Output => self.state.sinks.get(sel).map(|d| Command::SetSinkMute {
                index: d.index,
                mute: !d.mute,
            }),
            Tab::Input => self.state.sources.get(sel).map(|d| Command::SetSourceMute {
                index: d.index,
                mute: !d.mute,
            }),
            Tab::Configuration => {
                self.status = "Cards have no volume; mute a device instead".to_string();
                None
            }
        }
    }

    fn set_default(&mut self) -> Option<Command> {
        let sel = self.selection();
        match self.current_tab() {
            Tab::Output => {
                let d = self.state.sinks.get(sel)?;
                self.status = format!("Set {} as default output", d.description);
                Some(Command::SetDefaultSink(d.name.clone()))
            }
            Tab::Input => {
                let d = self.state.sources.get(sel)?;
                self.status = format!("Set {} as default input", d.description);
                Some(Command::SetDefaultSource(d.name.clone()))
            }
            _ => {
                self.status = "Set-as-default only applies to devices".to_string();
                None
            }
        }
    }

    fn kill_stream(&mut self) -> Option<Command> {
        let sel = self.selection();
        let (name, command) = match self.current_tab() {
            Tab::Playback => self
                .state
                .sink_inputs
                .get(sel)
                .map(|s| (s.name.clone(), Command::KillSinkInput(s.index)))?,
            Tab::Recording => self
                .state
                .source_outputs
                .get(sel)
                .map(|s| (s.name.clone(), Command::KillSourceOutput(s.index)))?,
            _ => {
                self.status = "Terminate only applies to streams".to_string();
                return None;
            }
        };
        self.status = format!("Terminated {name}");
        Some(command)
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
                    .map(|d| PopupItem {
                        label: d.description.clone(),
                        value: d.index.to_string(),
                        available: true,
                    })
                    .collect();
                Popup {
                    kind: PopupKind::MoveSourceOutput(s.index),
                    title: format!("Move \"{}\" to input:", s.name),
                    selected: self
                        .state
                        .sources
                        .iter()
                        .position(|d| d.index == s.device)
                        .unwrap_or(0),
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

    fn handle_popup_key(&mut self, code: KeyCode) -> Option<Command> {
        let popup = self.popup.as_mut()?;
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
            KeyCode::Enter => return self.popup.take().and_then(|p| p.confirm()),
            _ => {}
        }
        None
    }
}

impl Popup {
    /// The command for the highlighted entry.
    fn confirm(self) -> Option<Command> {
        let item = self.items.into_iter().nth(self.selected)?;
        match self.kind {
            PopupKind::MoveSinkInput(index) => item
                .value
                .parse()
                .ok()
                .map(|sink| Command::MoveSinkInput { index, sink }),
            PopupKind::MoveSourceOutput(index) => item
                .value
                .parse()
                .ok()
                .map(|source| Command::MoveSourceOutput { index, source }),
            PopupKind::SinkPort(index) => Some(Command::SetSinkPort {
                index,
                port: item.value,
            }),
            PopupKind::SourcePort(index) => Some(Command::SetSourcePort {
                index,
                port: item.value,
            }),
            PopupKind::CardProfile(card) => Some(Command::SetCardProfile {
                card,
                profile: item.value,
            }),
        }
    }
}
