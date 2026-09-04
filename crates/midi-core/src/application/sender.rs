//! The send screen's state: what is being composed, where it goes, and what went.
//!
//! # Why one service rather than several
//!
//! The same reason [`super::monitor::Monitor`] is one service: the state is
//! entangled. Sending needs the composition *and* the target; recording a send
//! needs the target's name *and* the outcome; withdrawing the published source
//! has to clear the chosen target when that is what it was. Splitting these
//! across services would push the coordination into the Tauri layer, which is
//! forbidden to hold it.
//!
//! # Why this does not hold the `Transmitter`
//!
//! Exactly as [`super::monitor::Monitor`] does not hold the
//! [`super::ports::EventSource`]. This service decides **what** should be sent
//! and **where**; the composition root performs the transmission through the port
//! and hands the outcome back. The shape is already established by
//! `AppState::sync_ports`, which reads the monitor's state, calls the adapter,
//! and returns the adapter's failures to the monitor.
//!
//! The alternative — injecting the port here — would put an `Arc<Mutex<dyn ..>>`
//! into the domain-facing layer and make every method that touches state
//! potentially blocking on a device. The two-step costs one extra call at one
//! site and keeps this crate free of the outside world.

use std::collections::VecDeque;

use super::error::CoreError;
use super::ports::{PlatformCapabilities, PublicationSupport};
use super::settings::{PersistedSettings, SavedRequest, SendSettings};
use crate::constants::SEND_RECORD_LIMIT;
use crate::domain::composition::{Composition, PersistedComposition};
use crate::domain::ids::{
    PublishedName, RequestName, SendRecordId, TargetId, TargetKey, Timestamp,
};
use crate::domain::message::MidiMessage;
use crate::domain::publication::Publication;
use crate::domain::request::{Request, RequestLibrary, RequestOrigin};
use crate::domain::send_record::{SendOutcome, SendRecord};
use crate::domain::sendable::{FieldId, SendableKind};
use crate::domain::target::Target;

/// The target the user has chosen, and the name it carried when they chose it.
///
/// # Why the name is remembered beside the key
///
/// A device can vanish between being chosen and being sent to, and the failure
/// has to name it — "MIDI Monitor is no longer available to send to" is
/// actionable, and "something is no longer available" is not. The key alone
/// cannot produce that name once the device is gone from the catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ChosenTarget {
    /// The identity that outlives the session, and is persisted.
    key: TargetKey,
    /// The name as last seen, or [`None`] when the choice was restored from disk
    /// and the device has not been seen this session.
    name: Option<String>,
}

/// What the composition root should transmit, and where.
///
/// # Why this exists rather than the sender exposing its parts
///
/// The three values have to be taken together and at one instant: the bytes, the
/// target they go to, and the name to record against them. Reading them one at a
/// time would let a device change between two of the reads and produce a record
/// that describes a send that did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outgoing {
    /// Where to send it.
    pub target: TargetId,
    /// The target's name, for the record and for any failure message.
    pub target_name: String,
    /// The interpretation, for the record's description.
    pub message: MidiMessage,
    /// The bytes to transmit, exactly as they must go out.
    pub bytes: Vec<u8>,
}

/// Everything the send screen shows and changes.
pub struct Sender {
    /// Where this machine can currently be sent to.
    targets: Vec<Target>,
    /// What the user chose, if anything.
    chosen: Option<ChosenTarget>,
    /// The published source's name, and whether it is published.
    publication: Publication,
    /// The message being built.
    composition: Composition,
    /// The built-in requests and the user's own.
    library: RequestLibrary,
    /// This session's sends, newest last, capped at [`SEND_RECORD_LIMIT`].
    records: VecDeque<SendRecord>,
    /// The next record identifier to mint. Monotonic across the session.
    next_record_id: u32,
    /// Why the last publication attempt failed, if it did.
    ///
    /// # Why this is remembered rather than only returned
    ///
    /// Publishing is attempted once at startup, restoring what the user left on.
    /// There is nobody to return an error to at that moment, and silently coming
    /// up unpublished would leave the user to discover it in the receiving
    /// program. Held here, the screen can say what happened the first time they
    /// look at it.
    publication_error: Option<String>,
    /// What this platform's MIDI access can and cannot do.
    capabilities: PlatformCapabilities,
}

impl Sender {
    /// Builds the send state from what the machine reports and what was saved.
    ///
    /// The chosen target is restored by key rather than resolved here: the device
    /// may be attached later in the session, and a key that matches nothing today
    /// is kept rather than cleared so that reattaching restores the choice.
    #[must_use]
    pub fn new(
        targets: Vec<Target>,
        saved: Option<&PersistedSettings>,
        capabilities: PlatformCapabilities,
    ) -> Self {
        let send = saved
            .map(|settings| settings.send.clone())
            .unwrap_or_default();
        Self {
            targets,
            chosen: send.target.map(|key| ChosenTarget { key, name: None }),
            publication: send.publication,
            // Saved requests are rebuilt here rather than in a later call, so a
            // freshly built sender is never briefly missing them.
            //
            // An entry that cannot be rebuilt is **dropped**, not refused: a
            // request written by a different version, or hand-edited into
            // something this build cannot read, costs the user that one request.
            // Refusing the document would cost them every setting in it, which is
            // a far worse trade for a far rarer cause.
            library: {
                let mut library = RequestLibrary::new();
                let mut seen: Vec<RequestName> = Vec::new();
                let restored = send
                    .saved_requests
                    .iter()
                    .filter(|entry| {
                        // A duplicate name cannot be produced by this application
                        // — saving and renaming both refuse one — so reaching here
                        // means a hand-edited document or a different version. The
                        // first occurrence is kept and the rest dropped, because
                        // losing one saved request is a far better outcome than
                        // refusing the document and losing every setting in it.
                        if seen.contains(&entry.name) {
                            return false;
                        }
                        seen.push(entry.name.clone());
                        true
                    })
                    .filter_map(|entry| {
                        entry.composition.rebuild().ok().map(|composition| Request {
                            name: entry.name.clone(),
                            description: entry.description.clone(),
                            composition,
                            origin: RequestOrigin::Saved,
                        })
                    })
                    .collect();
                library.replace_saved(restored);
                library
            },
            // Note On is where a user starts: it is the message that proves the
            // other end is listening, and the one the library's first entry is.
            composition: Composition::from_kind(SendableKind::NoteOn),
            records: VecDeque::new(),
            next_record_id: 0,
            publication_error: None,
            capabilities,
        }
    }

    /// Replaces the target list after a device change.
    ///
    /// The chosen target is deliberately **not** cleared when it disappears from
    /// the list. The user's choice survives an unplugged cable; what changes is
    /// only whether it can be resolved right now.
    pub fn replace_targets(&mut self, targets: Vec<Target>) {
        self.targets = targets;
        // Refresh the remembered name while the device is still in view, so a
        // later failure can name it after it has gone.
        if let Some(chosen) = self.chosen.as_mut() {
            if let Some(target) = self.targets.iter().find(|target| target.key == chosen.key) {
                chosen.name = Some(target.name.clone());
            }
        }
    }

    /// Where this machine can currently be sent to.
    #[must_use]
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    /// The chosen target, if one is chosen and currently present.
    #[must_use]
    pub fn chosen_target(&self) -> Option<&Target> {
        let chosen = self.chosen.as_ref()?;
        self.targets.iter().find(|target| target.key == chosen.key)
    }

    /// The published source's name and state.
    #[must_use]
    pub const fn publication(&self) -> &Publication {
        &self.publication
    }

    /// What this platform's MIDI access can and cannot do.
    #[must_use]
    pub const fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }

    /// The message currently being built.
    #[must_use]
    pub const fn composition(&self) -> &Composition {
        &self.composition
    }

    /// Replaces the message being built.
    pub fn set_composition(&mut self, composition: Composition) {
        self.composition = composition;
    }

    /// Sets one value on the message being built.
    ///
    /// # Errors
    ///
    /// Propagates [`Composition::set_field`]: [`CoreError::UnknownField`] for a
    /// field this message does not carry, [`CoreError::ValueOutOfRange`] for a
    /// number outside what it permits, and [`CoreError::MalformedSendBytes`] for
    /// an unreadable System Exclusive payload. The composition is unchanged
    /// whenever this fails.
    pub fn set_composition_field(&mut self, field: FieldId, entry: &str) -> Result<(), CoreError> {
        self.composition.set_field(field, entry)
    }

    /// This session's sends, oldest first.
    pub fn records(&self) -> impl Iterator<Item = &SendRecord> {
        self.records.iter()
    }

    /// What to transmit, and where.
    ///
    /// # Errors
    ///
    /// - [`CoreError::NoSendTarget`] when nothing is chosen, or when a choice
    ///   restored from disk names a device that has not been seen this session.
    ///   Both are the same thing from the user's side: the screen shows no target
    ///   chosen, so the message has to be an instruction rather than a complaint.
    /// - [`CoreError::UnknownTarget`] when a target that *was* present this
    ///   session has since gone. This one can name the device, which is why it is
    ///   worth distinguishing from the case above.
    /// - [`CoreError::UnsendableMessage`] from the encoder, which no composition
    ///   this crate can build will produce.
    pub fn outgoing(&self) -> Result<Outgoing, CoreError> {
        let Some(chosen) = self.chosen.as_ref() else {
            return Err(CoreError::NoSendTarget);
        };
        let Some(target) = self.targets.iter().find(|target| target.key == chosen.key) else {
            return match chosen.name.as_ref() {
                Some(name) => Err(CoreError::UnknownTarget { name: name.clone() }),
                None => Err(CoreError::NoSendTarget),
            };
        };

        Ok(Outgoing {
            target: target.id,
            target_name: target.name.clone(),
            message: self.composition.message().clone(),
            bytes: self.composition.bytes()?,
        })
    }

    /// Records the outcome of a send.
    ///
    /// Called for failures as well as successes: a send that failed is a send
    /// that happened, and leaving it out of the record would recreate exactly the
    /// confusion the record exists to remove.
    pub fn record(&mut self, outgoing: &Outgoing, outcome: SendOutcome, at: Timestamp) {
        let id = SendRecordId::new(self.next_record_id);
        self.next_record_id = self.next_record_id.wrapping_add(1);

        self.records.push_back(SendRecord {
            id,
            time: at,
            target: outgoing.target_name.clone(),
            message: outgoing.message.clone(),
            bytes: outgoing.bytes.clone(),
            outcome,
        });
        while self.records.len() > SEND_RECORD_LIMIT {
            self.records.pop_front();
        }
    }

    /// Chooses where traffic goes.
    ///
    /// # Errors
    ///
    /// [`CoreError::UnknownTarget`] when the identifier is not in the current
    /// list, which only a stale view produces. The choice is left as it was.
    pub fn set_target(&mut self, id: TargetId) -> Result<(), CoreError> {
        let target = self
            .targets
            .iter()
            .find(|target| target.id == id)
            .ok_or_else(|| CoreError::UnknownTarget {
                name: String::new(),
            })?;
        self.chosen = Some(ChosenTarget {
            key: target.key.clone(),
            name: Some(target.name.clone()),
        });
        Ok(())
    }

    /// Sets the name the published source carries.
    ///
    /// The caller republishes if the source is currently published; this method
    /// records the intent and nothing more, because whether an endpoint can be
    /// renamed is the platform's answer to give.
    pub fn set_publication_name(&mut self, name: PublishedName) {
        self.publication.name = name;
    }

    /// Records whether the source is published.
    ///
    /// Withdrawing it also clears the chosen target when the published source
    /// *was* that target — otherwise the screen would show a target that no
    /// longer exists, and the next send would fail for a reason the user could
    /// not see.
    ///
    /// # Errors
    ///
    /// [`CoreError::PublicationUnsupported`] when this platform cannot publish at
    /// all. Reachable only from a stale view, because the control is not operable
    /// where the platform reports the limitation.
    pub fn set_published(&mut self, published: bool) -> Result<(), CoreError> {
        if let PublicationSupport::Unsupported { detail } = &self.capabilities.publication {
            return Err(CoreError::PublicationUnsupported {
                detail: detail.clone(),
            });
        }
        self.publication.published = published;
        if !published
            && self
                .chosen
                .as_ref()
                .is_some_and(|chosen| chosen.key == TargetKey::PublishedSource)
        {
            self.chosen = None;
        }
        Ok(())
    }

    /// The half of the persisted settings this service owns.
    #[must_use]
    pub fn persisted_settings(&self) -> SendSettings {
        SendSettings {
            target: self.chosen.as_ref().map(|chosen| chosen.key.clone()),
            publication: self.publication.clone(),
            saved_requests: self
                .library
                .saved()
                .iter()
                .map(|request| SavedRequest {
                    name: request.name.clone(),
                    description: request.description.clone(),
                    composition: PersistedComposition::capture(&request.composition),
                })
                .collect(),
        }
    }
}

impl Sender {
    /// One recorded send, by identity.
    ///
    /// # Errors
    ///
    /// [`CoreError::UnknownSendRecord`] when the entry has been evicted past
    /// [`SEND_RECORD_LIMIT`]. The caller reports it and refreshes rather than
    /// re-sending whatever now sits in that position.
    pub fn recorded(&self, id: SendRecordId) -> Result<&SendRecord, CoreError> {
        self.records
            .iter()
            .find(|record| record.id == id)
            .ok_or(CoreError::UnknownSendRecord)
    }
}

impl Sender {
    /// Records that publishing failed, and reflects that the source is not up.
    ///
    /// Called when the platform refuses at startup, where there is no command to
    /// return an error from. The state is corrected rather than left claiming a
    /// source is published when it is not, because the screen showing the truth
    /// matters more than the screen showing what was asked for.
    pub fn report_publication_failure(&mut self, detail: &str) {
        self.publication.published = false;
        self.publication_error = Some(detail.to_owned());
    }

    /// Why the last publication attempt failed, if it did.
    #[must_use]
    pub fn publication_error(&self) -> Option<&str> {
        self.publication_error.as_deref()
    }

    /// Forgets a publication failure, once the user has acted on it.
    pub fn clear_publication_error(&mut self) {
        self.publication_error = None;
    }
}

impl Sender {
    /// The built-in requests and the user's own.
    #[must_use]
    pub const fn library(&self) -> &RequestLibrary {
        &self.library
    }

    /// Loads a request into the composition, ready to send or adjust.
    ///
    /// The request itself is untouched: a user who adjusts a built-in's values is
    /// changing what they are about to send, not the library entry.
    ///
    /// # Errors
    ///
    /// [`CoreError::UnknownRequest`] when nothing carries that name.
    pub fn select_request(&mut self, name: &RequestName) -> Result<(), CoreError> {
        let request = self.library.get(name)?;
        self.composition = request.composition.clone();
        Ok(())
    }
}

impl Sender {
    /// Replaces the composition with a different message type, at its defaults.
    pub fn set_composition_kind(&mut self, kind: SendableKind) {
        self.composition = Composition::from_kind(kind);
    }

    /// Replaces the composition with hand-typed bytes.
    ///
    /// # Errors
    ///
    /// [`CoreError::MalformedSendBytes`] when the entry is not exactly one valid
    /// MIDI message. The previous composition stays in force, so a mistyped entry
    /// does not cost the user what they had built.
    pub fn compose_raw(&mut self, entry: &str) -> Result<(), CoreError> {
        self.composition = Composition::parse_raw(entry)?;
        Ok(())
    }

    /// Saves the current composition as a named request.
    ///
    /// # Errors
    ///
    /// [`CoreError::DuplicateRequestName`] when the name is already taken.
    pub fn save_request(&mut self, name: RequestName) -> Result<(), CoreError> {
        self.library.save(name, self.composition.clone())
    }

    /// The library, for renaming and deleting the user's own requests.
    pub fn library_mut(&mut self) -> &mut RequestLibrary {
        &mut self.library
    }
}
