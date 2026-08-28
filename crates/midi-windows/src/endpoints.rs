//! Reading the machine's real MIDI input ports and naming them as Windows does.
//!
//! # Why names are taken verbatim, truncation and all
//!
//! Every name here comes from Windows and is passed through untouched — no
//! trimming, no title-casing, no deduplicating, no appending an index to tell two
//! identical names apart. Someone reading this list is trying to match it against
//! another MIDI application or against the label printed on a box, and a "tidier"
//! name is a name that no longer matches anything.
//!
//! Windows itself truncates: the field it reports a port's name in holds at most
//! [`MAX_PORT_NAME_CHARS`] characters, so a long device name arrives clipped.
//! That clipping is Windows' and is left as Windows made it. Recovering the full
//! name would mean a device-property-store lookup keyed on the interface string
//! below; it is a possible later refinement and deliberately not done here,
//! because a name this application repaired would differ from the name every
//! other program on the machine shows.
//!
//! Two ports genuinely may share a display name. They stay separate rows,
//! distinguished by identity rather than by text, which is exactly why identity
//! and name are different fields on a source.
//!
//! # Why identity is a device-interface string
//!
//! Windows offers no integer equivalent of the unique id macOS assigns an
//! endpoint. The enumeration index is *positional*: it shifts as devices come and
//! go, so persisting it would restore yesterday's choices onto today's numbering.
//! The device-interface string is the documented stable handle, survives replug
//! and reboot, and is the same string the newer Windows MIDI APIs expose — so
//! choosing it does not paint this adapter into a corner. See
//! `specs/003-windows-support/research.md`, decision D3.

use std::mem::size_of;

use midi_core::domain::ids::{SourceGroupId, SourceId, SourceKey};
use midi_core::domain::source::{Availability, Source};
use windows::Win32::Media::Audio::{
    midiInGetDevCapsW, midiInGetNumDevs, midiInMessage, HMIDIIN, MIDIINCAPSW,
};
use windows::Win32::Media::Multimedia::DRV_RESERVED;
use windows::Win32::Media::MMSYSERR_NOERROR;

/// The most characters Windows will report of a port's name.
///
/// Named because the number is Windows' and explains a visible behaviour — long
/// device names arrive clipped — rather than being an arbitrary buffer size of
/// ours.
pub const MAX_PORT_NAME_CHARS: usize = 32;

/// Asks the system for the byte size of a port's device-interface name.
///
/// The system intercepts this message and answers it without involving the
/// device's driver, which is why it can be asked of a device that is not open.
const DRV_QUERY_DEVICE_INTERFACE_SIZE: u32 = DRV_RESERVED + 13;

/// Asks the system to write a port's device-interface name into a buffer.
const DRV_QUERY_DEVICE_INTERFACE: u32 = DRV_RESERVED + 12;

/// One MIDI input port as Windows describes it.
///
/// # Why this exists rather than mapping inline
///
/// Reading a port's properties can fail one property at a time, and the
/// fallbacks differ: a missing name is cosmetic, a missing identity is not.
/// Naming the intermediate step gives those decisions one place to live instead
/// of scattering them through an enumeration loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortSnapshot {
    /// Where this port sits in Windows' enumeration **right now**.
    ///
    /// Positional and therefore short-lived: it is what every WinMM call takes,
    /// and it is emphatically not an identity. It is re-read on every scan and
    /// never persisted.
    pub device_index: u32,
    /// Windows' own display name, used verbatim.
    pub display_name: String,
    /// The device-interface string, or [`None`] when Windows reports none.
    ///
    /// Documented as possible: a device with no device interface yields a
    /// zero-length string. The name-based fallback that covers it lives in
    /// [`Self::key`].
    pub interface_id: Option<String>,
}

impl PortSnapshot {
    /// The identity a saved selection is stored against.
    ///
    /// Prefers the device-interface string, which survives replug and reboot.
    /// Falls back to the port's name when Windows reports no interface, which is
    /// the most stable thing left — and which two ports can share, a case the
    /// caller must handle rather than this method guessing at.
    #[must_use]
    pub fn key(&self) -> SourceKey {
        self.interface_id.as_ref().map_or_else(
            || SourceKey::PortName(self.display_name.clone()),
            |interface| SourceKey::DeviceInterface(interface.clone()),
        )
    }

    /// Turns this port into the domain's view of a source.
    #[must_use]
    pub fn to_source(&self, id: SourceId, availability: Availability) -> Source {
        Source {
            id,
            key: self.key(),
            name: self.display_name.clone(),
            group: Some(SourceGroupId::MidiSources),
            selected: false,
            availability,
        }
    }
}

/// Every MIDI input port Windows currently reports.
///
/// One entry per *port*, not per physical device: an interface exposing four
/// ports appears as four selectable rows, which is how Windows presents them and
/// how anyone routing MIDI thinks about them.
///
/// A port whose capabilities cannot be read is skipped rather than listed under
/// a made-up name: a row this application invented would be a row the user
/// cannot act on.
#[must_use]
pub fn enumerate() -> Vec<PortSnapshot> {
    // SAFETY: takes no arguments, touches no memory this code owns, and is
    // documented to be callable at any time. It cannot fail.
    let count = unsafe { midiInGetNumDevs() };

    (0..count).filter_map(read_port).collect()
}

/// Reads one port's capabilities, or [`None`] if Windows will not describe it.
fn read_port(device_index: u32) -> Option<PortSnapshot> {
    let mut caps = MIDIINCAPSW::default();

    // SAFETY: `caps` is a live, correctly aligned `MIDIINCAPSW` owned by this
    // frame, and the size passed is exactly its size, so the system cannot write
    // past it. `device_index` is below the count `midiInGetNumDevs` just
    // reported; a device removed since then makes this fail, which is handled.
    let result = unsafe {
        midiInGetDevCapsW(
            device_index as usize,
            &raw mut caps,
            u32::try_from(size_of::<MIDIINCAPSW>()).unwrap_or(u32::MAX),
        )
    };
    if result != MMSYSERR_NOERROR {
        return None;
    }

    Some(PortSnapshot {
        device_index,
        display_name: name_of(&caps),
        interface_id: interface_of(device_index),
    })
}

/// The port's display name, as Windows spelled it.
///
/// The field is a fixed-size array padded with nulls rather than a string, so the
/// name ends at the first null — reading the whole array would append the padding
/// to every name.
///
/// The array is copied out of the structure before being read. Windows declares
/// the structure byte-packed, so borrowing a field of it could produce a
/// misaligned reference; copying sixty-four bytes once per port per scan is not a
/// cost worth reasoning about.
fn name_of(caps: &MIDIINCAPSW) -> String {
    let units: [u16; MAX_PORT_NAME_CHARS] = caps.szPname;
    let end = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(MAX_PORT_NAME_CHARS);
    String::from_utf16_lossy(&units[..end])
}

/// The port's device-interface string, or [`None`] when it has none.
///
/// Asked of the device *id* rather than an open handle, which is what makes
/// identity readable during a plain enumeration — the system answers this
/// message itself instead of routing it to the driver, so nothing needs opening
/// and no port is taken away from another program to ask.
fn interface_of(device_index: u32) -> Option<String> {
    // The device id stands in for a handle here. That is the documented calling
    // convention for these two messages, not a cast this code invented.
    let handle = HMIDIIN(device_index as *mut core::ffi::c_void);

    let mut size_in_bytes: u32 = 0;
    // SAFETY: the size message writes one `u32` through the pointer given as
    // `dw1` and reads nothing else; `dw2` is documented as unused and must be
    // zero. The pointer is to a live local.
    let result = unsafe {
        midiInMessage(
            Some(handle),
            DRV_QUERY_DEVICE_INTERFACE_SIZE,
            Some(&raw mut size_in_bytes as usize),
            None,
        )
    };
    if result != MMSYSERR_NOERROR || size_in_bytes == 0 {
        // Zero is documented, not exceptional: it means this device has no
        // device interface. The caller falls back to the name.
        return None;
    }

    // The size is a *byte* count including the terminating null, and the string
    // is UTF-16, so the buffer is measured in units half that size. Rounding up
    // costs one unused unit and cannot under-allocate.
    let units = (size_in_bytes as usize).div_ceil(2);
    let mut buffer: Vec<u16> = vec![0; units];

    // SAFETY: the buffer is `units` UTF-16 units long and `size_in_bytes` is the
    // size the system just asked for, so the system writes within it. The buffer
    // outlives the call.
    let result = unsafe {
        midiInMessage(
            Some(handle),
            DRV_QUERY_DEVICE_INTERFACE,
            Some(buffer.as_mut_ptr() as usize),
            Some(size_in_bytes as usize),
        )
    };
    if result != MMSYSERR_NOERROR {
        return None;
    }

    let end = buffer.iter().position(|&unit| unit == 0).unwrap_or(units);
    let interface = String::from_utf16_lossy(&buffer[..end]);
    // An empty string is the same statement as a zero size, and the caller
    // treats it the same way.
    (!interface.is_empty()).then_some(interface)
}
