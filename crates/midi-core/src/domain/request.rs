//! Ready-made messages, and the ones the user saves beside them.
//!
//! # Why a request's name is its identity
//!
//! Names are unique across the whole library, so nothing is gained by minting an
//! id beside them — and a good deal is lost. Keying on the name means deletion
//! needs no index and no positional contract, and a view asking to delete
//! something already gone gets [`CoreError::UnknownRequest`] rather than deleting
//! whatever now sits in that position. This is the reasoning the data prefix
//! rules already use for their prefix, applied to the same shape of problem.
//!
//! Uniqueness is checked against built-in names as well as saved ones. A saved
//! request that shadowed a built-in would quietly falsify the promise that the
//! built-in library is always available in full.
//!
//! # Why the built-ins are code rather than data
//!
//! They are part of what this application *is*, not part of what a user has
//! configured. Storing them would let a stale copy on disk contradict the
//! promise above, and would make "the library is always complete" a thing to
//! repair rather than a thing that cannot break.

use super::composition::Composition;
use super::ids::{DataByte, RequestName};
use super::message::MidiMessage;
use super::sendable::SendableKind;
use crate::application::error::CoreError;

/// The controller number that releases every note on a channel.
///
/// Named because `123` on its own says nothing, and this is the one built-in
/// whose bytes are not simply a message type's defaults.
const CONTROLLER_ALL_NOTES_OFF: u8 = 123;

/// Where a request came from, and therefore what may be done to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOrigin {
    /// Shipped with the application. Always present, never editable.
    BuiltIn,
    /// Saved by the user. Renameable, deletable, persisted.
    Saved,
}

/// One named, sendable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The name, which is also the identity.
    pub name: RequestName,
    /// What it does, in plain language rather than in protocol terms.
    ///
    /// Carried rather than derived, because "what this does to a synthesiser" is
    /// not something the bytes can be asked. It is the whole reason someone can
    /// use this screen without knowing MIDI.
    pub description: String,
    /// The message, and where its authority lies.
    pub composition: Composition,
    /// Whether the user may rename or delete it.
    pub origin: RequestOrigin,
}

/// The built-in requests and the user's own, together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestLibrary {
    built_in: Vec<Request>,
    saved: Vec<Request>,
}

impl Default for RequestLibrary {
    fn default() -> Self {
        Self {
            built_in: built_in_requests(),
            saved: Vec::new(),
        }
    }
}

impl RequestLibrary {
    /// An empty library — built-ins only.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every request, built-ins first and then the user's own.
    ///
    /// Built-ins lead because they are what a first-time user needs, and their
    /// order is fixed by the code rather than by anything that could reorder
    /// itself between launches.
    pub fn all(&self) -> impl Iterator<Item = &Request> {
        self.built_in.iter().chain(self.saved.iter())
    }

    /// One request, by name.
    ///
    /// # Errors
    ///
    /// [`CoreError::UnknownRequest`] when nothing carries that name — a view
    /// acting on a request that has since been deleted or renamed.
    pub fn get(&self, name: &RequestName) -> Result<&Request, CoreError> {
        self.all()
            .find(|request| request.name == *name)
            .ok_or_else(|| CoreError::UnknownRequest {
                name: name.as_str().to_owned(),
            })
    }

    /// The user's own requests, for persistence.
    #[must_use]
    pub fn saved(&self) -> &[Request] {
        &self.saved
    }

    /// Replaces the user's own requests, as restored from storage.
    ///
    /// Takes what it is given: entries that could not be rebuilt were already
    /// dropped by the caller, one at a time, rather than costing the whole
    /// document.
    pub fn replace_saved(&mut self, saved: Vec<Request>) {
        self.saved = saved;
    }
}

/// The fifteen requests the application ships with.
///
/// Chosen for testing utility: between them they cover every everyday reason to
/// send something at a device — play a note, release it, silence it, move a
/// controller, change a sound, bend, press, drive transport, and ask the device
/// who it is.
fn built_in_requests() -> Vec<Request> {
    let from_kind = |name: &'static str, description: &'static str, kind: SendableKind| Request {
        name: RequestName::built_in(name),
        description: description.to_owned(),
        composition: Composition::from_kind(kind),
        origin: RequestOrigin::BuiltIn,
    };

    vec![
        from_kind(
            "Note On",
            "Play a note. Middle C at a firm velocity, on channel 1.",
            SendableKind::NoteOn,
        ),
        from_kind(
            "Note Off",
            "Release a note that is sounding.",
            SendableKind::NoteOff,
        ),
        Request {
            name: RequestName::built_in("All Notes Off"),
            description: "Release every note sounding on the channel. Use this when a note is \
                          stuck on."
                .to_owned(),
            composition: Composition::from_message(MidiMessage::Control {
                channel: all_notes_off_channel(),
                controller: DataByte::from_masked(CONTROLLER_ALL_NOTES_OFF),
                value: DataByte::ZERO,
            }),
            origin: RequestOrigin::BuiltIn,
        },
        from_kind(
            "Control Change",
            "Move a controller. Channel volume by default — turn it down and up to check the \
             other end is listening.",
            SendableKind::Control,
        ),
        from_kind(
            "Program Change",
            "Switch the instrument or patch the receiving device is using.",
            SendableKind::Program,
        ),
        from_kind(
            "Pitch Bend",
            "Bend the pitch. Starts centred, which bends nothing.",
            SendableKind::PitchWheel,
        ),
        from_kind(
            "Channel Pressure",
            "Press harder on every note at once, on devices that respond to it.",
            SendableKind::ChannelPressure,
        ),
        from_kind(
            "Aftertouch (Poly)",
            "Press harder on one note, on devices that respond to it.",
            SendableKind::AftertouchPoly,
        ),
        from_kind(
            "Start",
            "Tell the receiving device to start playing from the beginning.",
            SendableKind::Start,
        ),
        from_kind(
            "Stop",
            "Tell the receiving device to stop playing.",
            SendableKind::Stop,
        ),
        from_kind(
            "Continue",
            "Tell the receiving device to resume from where it stopped.",
            SendableKind::Continue,
        ),
        from_kind(
            "Clock",
            "One timing pulse. Devices expect twenty-four of these per quarter note, so a single \
             one proves the connection rather than driving anything.",
            SendableKind::Clock,
        ),
        from_kind(
            "Active Sense",
            "The keep-alive heartbeat some devices send to say they are still connected.",
            SendableKind::ActiveSense,
        ),
        from_kind(
            "Reset",
            "Tell the receiving device to return to its power-on state.",
            SendableKind::Reset,
        ),
        from_kind(
            "Identity Request",
            "Ask the device to say what it is. The reply arrives in the monitor like any other \
             traffic — this screen does not wait for it.",
            SendableKind::SystemExclusive,
        ),
    ]
}

/// Channel 1, for the one built-in that is not a message type's defaults.
///
/// Built from the wire nibble because that path cannot fail, so no error branch
/// is invented for a case that cannot occur.
fn all_notes_off_channel() -> super::ids::ChannelNumber {
    super::ids::ChannelNumber::from_wire_nibble(0)
}

impl RequestLibrary {
    /// Saves a composition under a name of the user's choosing.
    ///
    /// # Errors
    ///
    /// [`CoreError::DuplicateRequestName`] when the name is already taken — by a
    /// saved request **or** by a built-in. The library is left exactly as it was:
    /// there is no partially added request.
    pub fn save(&mut self, name: RequestName, composition: Composition) -> Result<(), CoreError> {
        self.refuse_duplicate(&name)?;
        self.saved.push(Request {
            name,
            description: String::new(),
            composition,
            origin: RequestOrigin::Saved,
        });
        Ok(())
    }

    /// Renames one of the user's own requests.
    ///
    /// # Errors
    ///
    /// - [`CoreError::UnknownRequest`] when `from` names nothing.
    /// - [`CoreError::BuiltInRequestImmutable`] when `from` names a built-in.
    /// - [`CoreError::DuplicateRequestName`] when `to` is already taken.
    ///
    /// Nothing is changed when any of these is returned.
    pub fn rename(&mut self, from: &RequestName, to: RequestName) -> Result<(), CoreError> {
        self.refuse_built_in(from)?;
        // Checked before the lookup below mutates anything, so a refused rename
        // cannot leave the list half-renamed.
        if *from != to {
            self.refuse_duplicate(&to)?;
        }
        let request = self
            .saved
            .iter_mut()
            .find(|request| request.name == *from)
            .ok_or_else(|| CoreError::UnknownRequest {
                name: from.as_str().to_owned(),
            })?;
        request.name = to;
        Ok(())
    }

    /// Deletes one of the user's own requests.
    ///
    /// # Errors
    ///
    /// - [`CoreError::UnknownRequest`] when nothing carries that name.
    /// - [`CoreError::BuiltInRequestImmutable`] when it names a built-in.
    pub fn delete(&mut self, name: &RequestName) -> Result<(), CoreError> {
        self.refuse_built_in(name)?;
        let before = self.saved.len();
        self.saved.retain(|request| request.name != *name);
        if self.saved.len() == before {
            return Err(CoreError::UnknownRequest {
                name: name.as_str().to_owned(),
            });
        }
        Ok(())
    }

    /// Refuses a name any request already carries.
    fn refuse_duplicate(&self, name: &RequestName) -> Result<(), CoreError> {
        if self.all().any(|request| request.name == *name) {
            return Err(CoreError::DuplicateRequestName {
                name: name.as_str().to_owned(),
            });
        }
        Ok(())
    }

    /// Refuses an attempt to change a built-in.
    fn refuse_built_in(&self, name: &RequestName) -> Result<(), CoreError> {
        if self.built_in.iter().any(|request| request.name == *name) {
            return Err(CoreError::BuiltInRequestImmutable {
                name: name.as_str().to_owned(),
            });
        }
        Ok(())
    }
}
