//! The core's error type.
//!
//! # Why errors are values here
//!
//! Every fallible operation in this crate returns `Result` with a variant that
//! names what went wrong and carries what the caller needs to respond. Panicking
//! is not an option the constitution leaves open, and for good reason: each of
//! these failures is a user typing something into a field, which is ordinary
//! behaviour rather than a bug. A malformed hex prefix must leave the previous
//! filter running, and it can only do that if it arrives as a value.

use crate::domain::filter::PrefixMode;
use crate::domain::ids::SourceId;
use thiserror::Error;

/// Something the core refused to do, and why.
///
/// Each variant documents the condition that produces it and what the caller
/// should do about it, because a caller that cannot act on an error has been
/// handed a string with extra steps.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    /// A channel number outside 1–16 was supplied.
    ///
    /// Produced when the One Channel field is typed into directly. The caller
    /// should reject the entry and keep the previously selected channel, so the
    /// filter never applies a channel that cannot exist.
    #[error("channel {value} is outside the valid range 1-16")]
    ChannelOutOfRange {
        /// The rejected value, so the interface can explain the refusal at the
        /// control that caused it rather than showing a generic failure.
        value: u8,
    },

    /// A retention limit outside the supported range was supplied.
    ///
    /// Produced by zero, or by a value above the stated ceiling. The caller
    /// should keep the previous limit rather than accept a number the
    /// application will not honour.
    #[error("retention limit {value} is outside the supported range 1-100000")]
    RetentionOutOfRange {
        /// The rejected count.
        value: usize,
    },

    /// A hex prefix entry contained a character that is not a hex digit.
    ///
    /// The caller must report the entry as invalid and **leave the last valid
    /// filter in effect** — blanking the event list because of a typo would
    /// look like a crash.
    #[error("'{entry}' is not a valid hexadecimal prefix")]
    MalformedHexPrefix {
        /// The offending entry, echoed back so the message can quote it.
        entry: String,
    },

    /// A hex prefix entry held no hex digits at all.
    ///
    /// Distinct from [`Self::MalformedHexPrefix`] because the remedy differs:
    /// there is nothing to correct, the user simply has not finished typing.
    #[error("a hexadecimal prefix cannot be empty")]
    EmptyHexPrefix,

    /// A prefix rule was added whose prefix is already in the list.
    ///
    /// Produced under **either** kind, because the prefix is a rule's identity:
    /// two rules carrying the same prefix would make deletion ambiguous, and one
    /// of each kind would contradict outright. The caller should report the
    /// refusal and leave the list untouched; the remedy is to delete the listed
    /// rule or to enter a different prefix.
    #[error("a rule for '{prefix}' is already in the list")]
    DuplicatePrefixRule {
        /// The normalised prefix, for quoting back at the entry field.
        prefix: String,
        /// The kind the listed rule carries, so the message can name it.
        existing_kind: PrefixMode,
    },

    /// A prefix rule was added that contradicts one already in the list.
    ///
    /// Produced when the two prefixes overlap — one begins the other — and their
    /// kinds differ, which is the only way an event could match both a show-only
    /// rule and a hide rule. Refusing the pair here is what lets
    /// [`crate::domain::filter::DataPrefixFilter::admits`] need no precedence
    /// rule at all.
    ///
    /// Distinct from [`Self::DuplicatePrefixRule`] because the remedy differs:
    /// there is no duplicate to delete, one of the two prefixes has to be
    /// narrowed or one of the kinds changed. The caller should name both rules,
    /// since the conflicting one is not the one the user just typed.
    #[error(
        "'{prefix}' overlaps the existing rule for '{existing_prefix}', which has the opposite effect"
    )]
    ContradictoryPrefixRule {
        /// The normalised prefix being added.
        prefix: String,
        /// The kind being added.
        kind: PrefixMode,
        /// The listed prefix it overlaps.
        existing_prefix: String,
        /// That rule's kind.
        existing_kind: PrefixMode,
    },

    /// A prefix arrived that names no rule in the list.
    ///
    /// Signals an interface holding a rule list the core has since changed — the
    /// prefix-rule counterpart of [`Self::UnknownSource`]. The caller should
    /// refresh its view of the list rather than retry.
    #[error("no rule for '{prefix}' is in the list")]
    UnknownPrefixRule {
        /// The unrecognised prefix.
        prefix: String,
    },

    /// Hiding this column would leave the table with no columns at all.
    ///
    /// The caller should refuse the interaction. Enforced in the core rather
    /// than in the interface so the rule cannot be bypassed by a future caller.
    #[error("at least one column must remain visible")]
    LastColumnVisible,

    /// Settings could not be read from or written to storage.
    ///
    /// The caller should surface this but keep running: the change it
    /// accompanies has already been applied in memory, and failing to remember
    /// a preference is not a reason to reject it.
    #[error("settings storage failed: {detail}")]
    SettingsStorage {
        /// What the storage layer reported, for the message.
        detail: String,
    },

    /// A source id arrived that the catalogue does not contain.
    ///
    /// Signals a stale interface referring to a source that no longer exists.
    /// With real hardware this is ordinary rather than exceptional: a device can
    /// be unplugged between the webview rendering a row and the user clicking
    /// it. The caller should refresh its catalogue rather than retry.
    #[error("no source with id {}", id.get())]
    UnknownSource {
        /// The unrecognised id.
        id: SourceId,
    },

    /// A port exists but could not be opened for listening.
    ///
    /// Produced when another application holds the device exclusively, or when
    /// the operating system refuses the connection. The caller must **keep
    /// monitoring every port that did open** and surface this against the one
    /// that did not — one unavailable device is not a reason to stop watching
    /// the others.
    #[error("could not open '{name}': {detail}")]
    PortUnavailable {
        /// The port's display name, so the message can identify it to the user.
        name: String,
        /// What the operating system reported.
        detail: String,
    },

    /// The MIDI system itself could not be reached.
    ///
    /// Produced when the platform's MIDI service is unavailable or access was
    /// refused. The caller must present this as **distinct from having found no
    /// devices**: "there is nothing attached" and "I cannot see what is
    /// attached" call for entirely different responses from the user, and an
    /// empty list would conflate them.
    #[error("the MIDI system is unavailable: {detail}")]
    MidiSystemUnavailable {
        /// What the platform reported, for the message.
        detail: String,
    },

    /// A message was handed to the encoder that cannot be transmitted.
    ///
    /// Produced only by [`crate::domain::message::MidiMessage::Invalid`], which
    /// models data a *monitor* observed rather than anything a sender composed.
    /// No caller in this application can reach it: the composition API cannot
    /// construct that variant. It exists because the monitor's message type must
    /// keep modelling malformed input, and narrowing it to what can be sent would
    /// damage the older feature to serve the newer one.
    #[error("this message cannot be transmitted: {reason}")]
    UnsendableMessage {
        /// Why the message is not transmittable, for the message shown.
        reason: String,
    },

    /// Hand-typed bytes are not exactly one valid MIDI message.
    ///
    /// Produced when the entry decodes to nothing, to more than one message, to
    /// malformed data, or to an unterminated System Exclusive. The caller shows
    /// `detail` — the decoder's own reason — at the entry field and transmits
    /// nothing, leaving the previous composition in force.
    #[error("those bytes are not a single valid MIDI message: {detail}")]
    MalformedSendBytes {
        /// What the decoder found wrong, for the message shown.
        detail: String,
    },

    /// A composition value was set outside the range its message permits.
    ///
    /// The caller states the permitted range at the control being edited and
    /// leaves the composition as it was. An out-of-range value must never reach
    /// the encoder.
    #[error("{field} must be between {min} and {max}")]
    ValueOutOfRange {
        /// The field's label, for the message shown.
        field: String,
        /// Lowest accepted value.
        min: u16,
        /// Highest accepted value.
        max: u16,
    },

    /// A send was attempted with no target chosen.
    ///
    /// The caller explains that a target is needed and transmits nothing. This is
    /// the ordinary first-use state, not a fault, so the message reads as an
    /// instruction rather than as a complaint.
    #[error("choose where to send before sending")]
    NoSendTarget,

    /// The chosen target is no longer among those the machine reports.
    ///
    /// Typically the device was unplugged, or the program holding the port closed
    /// it. The caller reports it and refreshes the target list; the stored key is
    /// kept, so reattaching the device restores the choice.
    #[error("{name} is no longer available to send to")]
    UnknownTarget {
        /// The target's name as it was last known, for the message shown.
        name: String,
    },

    /// The platform accepted neither the bytes nor the target.
    ///
    /// The caller records a failed send carrying `detail` and leaves the
    /// composition untouched, so the user can retry without re-entering anything.
    #[error("could not send to {target}: {detail}")]
    TransmitFailed {
        /// The target's name, for the message shown and for the record.
        target: String,
        /// What the platform reported.
        detail: String,
    },

    /// This platform cannot publish a MIDI source at all.
    ///
    /// Reachable only from a stale view: the control is not operable where the
    /// platform reports the limitation, because a control that looks live and
    /// does nothing is worse than one that explains itself. `detail` is the
    /// adapter's own wording, naming what to do instead.
    #[error("{detail}")]
    PublicationUnsupported {
        /// What the platform cannot do and what the user can do instead.
        detail: String,
    },

    /// The platform can publish a source, but this attempt did not succeed.
    ///
    /// Distinct from [`Self::PublicationUnsupported`] because the responses
    /// differ: this one is worth retrying and the other never will be. The caller
    /// reports it and leaves everything else — including sending to destinations
    /// — working.
    #[error("the source could not be published: {detail}")]
    PublicationFailed {
        /// What the platform reported.
        detail: String,
    },

    /// The name for the published source is empty or too long.
    ///
    /// The caller shows this at the name field and keeps the name already in
    /// force. Other programs remember their settings against this string, so an
    /// unusable name is refused rather than silently trimmed into something the
    /// user did not choose.
    #[error("a name for the published source must be 1 to {max} characters")]
    MalformedPublicationName {
        /// The accepted ceiling, for the message shown.
        max: usize,
    },

    /// No request in the library carries that name.
    ///
    /// Produced when a view asks for a request that has since been deleted or
    /// renamed. The caller reports it and refreshes the list rather than acting
    /// on a stale name.
    #[error("there is no request called {name}")]
    UnknownRequest {
        /// The name that was asked for.
        name: String,
    },

    /// A saved request would take a name the library already holds.
    ///
    /// Checked against built-in names as well as saved ones: a saved request that
    /// shadowed a built-in would quietly falsify the promise that the built-in
    /// library is always available in full. The caller shows this at the save
    /// control and adds nothing.
    #[error("a request called {name} already exists")]
    DuplicateRequestName {
        /// The name that collided.
        name: String,
    },

    /// A built-in request cannot be renamed or deleted.
    ///
    /// The built-in library is code, not data, and is always available in full.
    /// The caller leaves the library untouched.
    #[error("{name} is a built-in request and cannot be changed")]
    BuiltInRequestImmutable {
        /// The built-in that was targeted.
        name: String,
    },

    /// The name for a saved request is empty or too long.
    ///
    /// The name is the request's identity, so an unusable one is refused at the
    /// save control rather than normalised into something the user did not type.
    #[error("a request name must be 1 to {max} characters")]
    MalformedRequestName {
        /// The accepted ceiling, for the message shown.
        max: usize,
    },

    /// The composition does not carry the field that was set.
    ///
    /// Reachable only from a view whose model is stale — a field belonging to a
    /// message type that is no longer composed, or any field at all on a
    /// hand-typed composition, which has none. The caller refreshes rather than
    /// blaming the user.
    #[error("this message has no {field} value")]
    UnknownField {
        /// The field identifier that was asked for.
        field: String,
    },

    /// No composable message carries that identifier.
    ///
    /// Reachable only from a stale view. The caller refreshes.
    #[error("there is no message type called {id}")]
    UnknownSendableKind {
        /// The identifier that was asked for.
        id: String,
    },

    /// No send record carries that identifier.
    ///
    /// Produced when a re-send names a record evicted past the retention ceiling.
    /// The caller reports it and refreshes the record list.
    #[error("that send is no longer in the record")]
    UnknownSendRecord,

    /// The platform reported a host-clock rate of zero ticks per second.
    ///
    /// Produced once, at startup, when `mach_timebase_info` or
    /// `QueryPerformanceFrequency` answers with a value that would make every
    /// tick-to-nanosecond conversion a division by zero. It is raised here, at
    /// construction, precisely so the renderer can take a rate that is already
    /// known good and have no error case of its own — a fallible path that runs
    /// once per table row is a path that will eventually reach for `unwrap`.
    ///
    /// The caller should report that host time is unavailable and leave the
    /// monitor running on `Clock time`; nothing else about the application
    /// depends on this reading.
    #[error("the platform reported an unusable host clock rate")]
    UnusableTickRate,
}
