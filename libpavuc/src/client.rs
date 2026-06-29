//! The PulseAudio/PipeWire client.
//!
//! [`PulseClient`] owns a non-threaded `libpulse` main loop that is pumped
//! cooperatively from the UI's event loop via [`PulseClient::iterate`]. All
//! callbacks therefore run on the same thread as the UI, which lets us keep the
//! shared state behind a plain [`Rc<RefCell<_>>`] instead of locks.
//!
//! On a PipeWire system this transparently connects to `pipewire-pulse`, the
//! PulseAudio-compatible server, exactly like pavucontrol does.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::subscribe::InterestMaskSet;
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
use libpulse_binding::operation::{Operation, State as OperationState};
use libpulse_binding::volume::{ChannelVolumes, Volume};

use crate::error::{Error, Result};
use crate::model::{Card, Device, PulseState, Stream};
use crate::volume;

/// Type-erased handle that keeps an in-flight operation (and, crucially, its
/// boxed callback) alive until the server reports it finished.
trait PendingOp {
    fn is_finished(&self) -> bool;
}

impl<T: ?Sized> PendingOp for Operation<T> {
    fn is_finished(&self) -> bool {
        !matches!(self.get_state(), OperationState::Running)
    }
}

/// A live connection to the audio server.
pub struct PulseClient {
    mainloop: Rc<RefCell<Mainloop>>,
    context: Rc<RefCell<Context>>,
    state: Rc<RefCell<PulseState>>,
    dirty: Rc<Cell<bool>>,
    pending: Vec<Box<dyn PendingOp>>,
}

impl PulseClient {
    /// Connects to the default audio server, blocking until the connection is
    /// ready or fails.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the main loop or context cannot be created, or
    /// if the connection to the server fails.
    pub fn connect(app_name: &str) -> Result<Self> {
        let mainloop = Rc::new(RefCell::new(Mainloop::new().ok_or(Error::Mainloop)?));
        let context = Rc::new(RefCell::new(
            Context::new(&*mainloop.borrow(), app_name).ok_or(Error::Context)?,
        ));

        context
            .borrow_mut()
            .connect(None, ContextFlagSet::NOFLAGS, None)
            .map_err(|e| Error::Connect(format!("{e}")))?;

        // Pump the loop until the context is ready or fails.
        loop {
            match mainloop.borrow_mut().iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    return Err(Error::Connect("main loop stopped".to_string()));
                }
                IterateResult::Success(_) => {}
            }
            match context.borrow().get_state() {
                ContextState::Ready => break,
                ContextState::Failed | ContextState::Terminated => {
                    return Err(Error::Connect("server refused the connection".to_string()));
                }
                _ => {}
            }
        }

        let state = Rc::new(RefCell::new(PulseState::default()));
        let dirty = Rc::new(Cell::new(true));

        // Whenever anything changes, mark the snapshot dirty so the next
        // `iterate` re-introspects the server.
        {
            let dirty_cb = dirty.clone();
            context.borrow_mut().set_subscribe_callback(Some(Box::new(
                move |_facility, _operation, _index| {
                    dirty_cb.set(true);
                },
            )));
        }

        let mut client = Self {
            mainloop,
            context,
            state,
            dirty,
            pending: Vec::new(),
        };

        let op = client
            .context
            .borrow_mut()
            .subscribe(InterestMaskSet::ALL, |_success| {});
        client.pending.push(Box::new(op));

        Ok(client)
    }

    /// Drives the main loop, dispatching any pending server events and
    /// refreshing the cached state when it has changed.
    ///
    /// Call this once per UI tick.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Disconnected`] if the connection to the server is lost.
    pub fn iterate(&mut self) -> Result<()> {
        loop {
            let result = self.mainloop.borrow_mut().iterate(false);
            match result {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    return Err(Error::Disconnected);
                }
                IterateResult::Success(dispatched) => {
                    if dispatched == 0 {
                        break;
                    }
                }
            }
        }

        match self.context.borrow().get_state() {
            ContextState::Failed | ContextState::Terminated => return Err(Error::Disconnected),
            _ => {}
        }

        if self.dirty.get() {
            self.dirty.set(false);
            self.refresh_all();
        }

        self.pending.retain(|op| !op.is_finished());
        Ok(())
    }

    /// Returns an owned snapshot of the current server state.
    #[must_use]
    pub fn snapshot(&self) -> PulseState {
        self.state.borrow().clone()
    }

    // --- Volume / mute -----------------------------------------------------

    /// Sets the volume of a sink (output device) to `percent` on all channels.
    pub fn set_sink_volume(&mut self, index: u32, channels: u8, percent: u32) {
        let cv = uniform_volume(channels, volume::percent_to_raw(percent));
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_sink_volume_by_index(index, &cv, None);
        self.pending.push(Box::new(op));
    }

    /// Mutes or unmutes a sink (output device).
    pub fn set_sink_mute(&mut self, index: u32, mute: bool) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_sink_mute_by_index(index, mute, None);
        self.pending.push(Box::new(op));
    }

    /// Sets the volume of a source (input device) to `percent` on all channels.
    pub fn set_source_volume(&mut self, index: u32, channels: u8, percent: u32) {
        let cv = uniform_volume(channels, volume::percent_to_raw(percent));
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_source_volume_by_index(index, &cv, None);
        self.pending.push(Box::new(op));
    }

    /// Mutes or unmutes a source (input device).
    pub fn set_source_mute(&mut self, index: u32, mute: bool) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_source_mute_by_index(index, mute, None);
        self.pending.push(Box::new(op));
    }

    /// Sets the volume of a playback stream (sink input).
    pub fn set_sink_input_volume(&mut self, index: u32, channels: u8, percent: u32) {
        let cv = uniform_volume(channels, volume::percent_to_raw(percent));
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_sink_input_volume(index, &cv, None);
        self.pending.push(Box::new(op));
    }

    /// Mutes or unmutes a playback stream (sink input).
    pub fn set_sink_input_mute(&mut self, index: u32, mute: bool) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_sink_input_mute(index, mute, None);
        self.pending.push(Box::new(op));
    }

    /// Sets the volume of a recording stream (source output).
    pub fn set_source_output_volume(&mut self, index: u32, channels: u8, percent: u32) {
        let cv = uniform_volume(channels, volume::percent_to_raw(percent));
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_source_output_volume(index, &cv, None);
        self.pending.push(Box::new(op));
    }

    /// Mutes or unmutes a recording stream (source output).
    pub fn set_source_output_mute(&mut self, index: u32, mute: bool) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_source_output_mute(index, mute, None);
        self.pending.push(Box::new(op));
    }

    // --- Defaults / routing ------------------------------------------------

    /// Sets the default sink (fallback output device) by name.
    pub fn set_default_sink(&mut self, name: &str) {
        let op = self.context.borrow_mut().set_default_sink(name, |_| {});
        self.pending.push(Box::new(op));
    }

    /// Sets the default source (fallback input device) by name.
    pub fn set_default_source(&mut self, name: &str) {
        let op = self.context.borrow_mut().set_default_source(name, |_| {});
        self.pending.push(Box::new(op));
    }

    /// Moves a playback stream to a different sink.
    pub fn move_sink_input(&mut self, index: u32, sink_index: u32) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.move_sink_input_by_index(index, sink_index, None);
        self.pending.push(Box::new(op));
    }

    /// Moves a recording stream to a different source.
    pub fn move_source_output(&mut self, index: u32, source_index: u32) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.move_source_output_by_index(index, source_index, None);
        self.pending.push(Box::new(op));
    }

    /// Selects the active profile of a card.
    pub fn set_card_profile(&mut self, card_index: u32, profile: &str) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_card_profile_by_index(card_index, profile, None);
        self.pending.push(Box::new(op));
    }

    /// Selects the active port of a sink (output device).
    pub fn set_sink_port(&mut self, index: u32, port: &str) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_sink_port_by_index(index, port, None);
        self.pending.push(Box::new(op));
    }

    /// Selects the active port of a source (input device).
    pub fn set_source_port(&mut self, index: u32, port: &str) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.set_source_port_by_index(index, port, None);
        self.pending.push(Box::new(op));
    }

    /// Terminates a playback stream.
    pub fn kill_sink_input(&mut self, index: u32) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.kill_sink_input(index, |_| {});
        self.pending.push(Box::new(op));
    }

    /// Terminates a recording stream.
    pub fn kill_source_output(&mut self, index: u32) {
        let mut introspect = self.context.borrow().introspect();
        let op = introspect.kill_source_output(index, |_| {});
        self.pending.push(Box::new(op));
    }

    // --- Introspection -----------------------------------------------------

    fn refresh_all(&mut self) {
        self.refresh_server();
        self.refresh_sinks();
        self.refresh_sources();
        self.refresh_sink_inputs();
        self.refresh_source_outputs();
        self.refresh_cards();
        self.refresh_clients();
    }

    fn refresh_server(&mut self) {
        let state = self.state.clone();
        let op = self
            .context
            .borrow()
            .introspect()
            .get_server_info(move |info| {
                let mut state = state.borrow_mut();
                state.default_sink = info.default_sink_name.as_ref().map(ToString::to_string);
                state.default_source = info.default_source_name.as_ref().map(ToString::to_string);
            });
        self.pending.push(Box::new(op));
    }

    fn refresh_sinks(&mut self) {
        let state = self.state.clone();
        let acc = Rc::new(RefCell::new(Vec::new()));
        let op = self
            .context
            .borrow()
            .introspect()
            .get_sink_info_list(move |res| match res {
                ListResult::Item(info) => acc.borrow_mut().push(Device::from_sink(info)),
                ListResult::End => state.borrow_mut().sinks = std::mem::take(&mut acc.borrow_mut()),
                ListResult::Error => {}
            });
        self.pending.push(Box::new(op));
    }

    fn refresh_sources(&mut self) {
        let state = self.state.clone();
        let acc = Rc::new(RefCell::new(Vec::new()));
        let op = self
            .context
            .borrow()
            .introspect()
            .get_source_info_list(move |res| match res {
                ListResult::Item(info) => acc.borrow_mut().push(Device::from_source(info)),
                ListResult::End => {
                    state.borrow_mut().sources = std::mem::take(&mut acc.borrow_mut());
                }
                ListResult::Error => {}
            });
        self.pending.push(Box::new(op));
    }

    fn refresh_sink_inputs(&mut self) {
        let state = self.state.clone();
        let acc = Rc::new(RefCell::new(Vec::new()));
        let op =
            self.context
                .borrow()
                .introspect()
                .get_sink_input_info_list(move |res| match res {
                    ListResult::Item(info) => acc.borrow_mut().push(Stream::from_sink_input(info)),
                    ListResult::End => {
                        state.borrow_mut().sink_inputs = std::mem::take(&mut acc.borrow_mut());
                    }
                    ListResult::Error => {}
                });
        self.pending.push(Box::new(op));
    }

    fn refresh_source_outputs(&mut self) {
        let state = self.state.clone();
        let acc = Rc::new(RefCell::new(Vec::new()));
        let op = self
            .context
            .borrow()
            .introspect()
            .get_source_output_info_list(move |res| match res {
                ListResult::Item(info) => {
                    acc.borrow_mut().push(Stream::from_source_output(info));
                }
                ListResult::End => {
                    state.borrow_mut().source_outputs = std::mem::take(&mut acc.borrow_mut());
                }
                ListResult::Error => {}
            });
        self.pending.push(Box::new(op));
    }

    fn refresh_cards(&mut self) {
        let state = self.state.clone();
        let acc = Rc::new(RefCell::new(Vec::new()));
        let op = self
            .context
            .borrow()
            .introspect()
            .get_card_info_list(move |res| match res {
                ListResult::Item(info) => acc.borrow_mut().push(Card::from_info(info)),
                ListResult::End => state.borrow_mut().cards = std::mem::take(&mut acc.borrow_mut()),
                ListResult::Error => {}
            });
        self.pending.push(Box::new(op));
    }

    fn refresh_clients(&mut self) {
        let state = self.state.clone();
        let acc = Rc::new(RefCell::new(std::collections::HashMap::new()));
        let op = self
            .context
            .borrow()
            .introspect()
            .get_client_info_list(move |res| match res {
                ListResult::Item(info) => {
                    let name = info
                        .name
                        .as_ref()
                        .map_or_else(String::new, ToString::to_string);
                    acc.borrow_mut().insert(info.index, name);
                }
                ListResult::End => {
                    state.borrow_mut().clients = std::mem::take(&mut acc.borrow_mut());
                }
                ListResult::Error => {}
            });
        self.pending.push(Box::new(op));
    }
}

impl Drop for PulseClient {
    fn drop(&mut self) {
        self.context.borrow_mut().disconnect();
    }
}

/// Builds a [`ChannelVolumes`] with `channels` channels all set to `raw`.
fn uniform_volume(channels: u8, raw: u32) -> ChannelVolumes {
    let mut cv = ChannelVolumes::default();
    cv.set(channels.max(1), Volume(raw));
    cv
}
