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

use crate::command::Command;
use crate::error::{Error, Result};
use crate::model::{Card, Device, PulseState, Stream};

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
        Self::open(app_name, None)
    }

    /// Connects to the server at `server` (e.g. `unix:/run/user/1000/pulse/native`
    /// or `tcp:host:4713`), blocking until the connection is ready or fails.
    ///
    /// # Errors
    ///
    /// As for [`Self::connect`].
    pub fn connect_to(app_name: &str, server: &str) -> Result<Self> {
        Self::open(app_name, Some(server))
    }

    fn open(app_name: &str, server: Option<&str>) -> Result<Self> {
        let mainloop = Rc::new(RefCell::new(Mainloop::new().ok_or(Error::Mainloop)?));
        let context = Rc::new(RefCell::new(
            Context::new(&*mainloop.borrow(), app_name).ok_or(Error::Context)?,
        ));

        // `pa_context_connect` only returns -1; the reason is in the context's errno.
        if context
            .borrow_mut()
            .connect(server, ContextFlagSet::NOFLAGS, None)
            .is_err()
        {
            return Err(Error::Connect(last_error(&context.borrow())));
        }

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
                    return Err(Error::Connect(last_error(&context.borrow())));
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

    /// Sends `command` to the server.
    ///
    /// The command is queued and runs asynchronously; its effect shows up in a
    /// later [`Self::snapshot`] once the server reports the change.
    pub fn execute(&mut self, command: Command) {
        let mut introspect = self.context.borrow().introspect();
        let op: Box<dyn PendingOp> = match command {
            Command::SetSinkVolume { index, volume } => Box::new(
                introspect.set_sink_volume_by_index(index, &channel_volumes(&volume), None),
            ),
            Command::SetSinkMute { index, mute } => {
                Box::new(introspect.set_sink_mute_by_index(index, mute, None))
            }
            Command::SetSourceVolume { index, volume } => Box::new(
                introspect.set_source_volume_by_index(index, &channel_volumes(&volume), None),
            ),
            Command::SetSourceMute { index, mute } => {
                Box::new(introspect.set_source_mute_by_index(index, mute, None))
            }
            Command::SetSinkInputVolume { index, volume } => {
                Box::new(introspect.set_sink_input_volume(index, &channel_volumes(&volume), None))
            }
            Command::SetSinkInputMute { index, mute } => {
                Box::new(introspect.set_sink_input_mute(index, mute, None))
            }
            Command::SetSourceOutputVolume { index, volume } => Box::new(
                introspect.set_source_output_volume(index, &channel_volumes(&volume), None),
            ),
            Command::SetSourceOutputMute { index, mute } => {
                Box::new(introspect.set_source_output_mute(index, mute, None))
            }
            Command::SetDefaultSink(name) => {
                Box::new(self.context.borrow_mut().set_default_sink(&name, |_| {}))
            }
            Command::SetDefaultSource(name) => {
                Box::new(self.context.borrow_mut().set_default_source(&name, |_| {}))
            }
            Command::MoveSinkInput { index, sink } => {
                Box::new(introspect.move_sink_input_by_index(index, sink, None))
            }
            Command::MoveSourceOutput { index, source } => {
                Box::new(introspect.move_source_output_by_index(index, source, None))
            }
            Command::SetCardProfile { card, profile } => {
                Box::new(introspect.set_card_profile_by_index(card, &profile, None))
            }
            Command::SetSinkPort { index, port } => {
                Box::new(introspect.set_sink_port_by_index(index, &port, None))
            }
            Command::SetSourcePort { index, port } => {
                Box::new(introspect.set_source_port_by_index(index, &port, None))
            }
            Command::KillSinkInput(index) => Box::new(introspect.kill_sink_input(index, |_| {})),
            Command::KillSourceOutput(index) => {
                Box::new(introspect.kill_source_output(index, |_| {}))
            }
        };
        self.pending.push(op);
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

/// The context's last error as text, e.g. "Connection refused".
fn last_error(context: &Context) -> String {
    context
        .errno()
        .to_string()
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| "unknown error".to_string())
}

/// Builds a [`ChannelVolumes`] from raw per-channel values.
///
/// An empty slice becomes one muted channel; values past
/// [`ChannelVolumes::CHANNELS_MAX`] are dropped.
fn channel_volumes(raw: &[u32]) -> ChannelVolumes {
    let mut cv = ChannelVolumes::default();
    let channels = raw
        .len()
        .clamp(1, usize::from(ChannelVolumes::CHANNELS_MAX));
    #[allow(clippy::cast_possible_truncation)] // clamped to CHANNELS_MAX (32) above
    cv.set(channels as u8, Volume::MUTED);
    for (slot, &value) in cv.get_mut().iter_mut().zip(raw) {
        *slot = Volume(value);
    }
    cv
}

#[cfg(test)]
mod tests;
