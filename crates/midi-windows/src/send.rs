//! Sending bytes out through a WinMM output device.
//!
//! # Why the handle is held rather than opened per send
//!
//! `midiOutOpen` talks to a driver. It is slow enough to be visible on a keypress,
//! and it can fail outright when another program holds the device — so opening
//! once per send would turn a working target into an intermittent one, and would
//! do it under exactly the load a user is most likely to generate while testing.
//! The handle is opened on first use for the chosen device and kept until the
//! choice changes or the adapter shuts down.
//!
//! # Why System Exclusive takes a different call
//!
//! WinMM splits output in two. `midiOutShortMsg` takes a message packed into a
//! single `u32`, which covers every message of three bytes or fewer — that is,
//! everything except System Exclusive. Anything longer goes through
//! `midiOutLongMsg`, which takes a `MIDIHDR` pointing at a buffer that must be
//! prepared before the call and unprepared after it, and which must stay put in
//! memory for the whole of that. The `Box` below is what pins it.
//!
//! This is the same division `sysex` already handles on the receive side, and the
//! two are kept apart here for the same reason: they are different Windows
//! contracts, not two shapes of one.

use std::mem::size_of;

use midi_core::application::error::CoreError;
use windows::Win32::Media::Audio::{
    midiOutClose, midiOutLongMsg, midiOutOpen, midiOutPrepareHeader, midiOutReset, midiOutShortMsg,
    midiOutUnprepareHeader, CALLBACK_NULL, HMIDIOUT, MIDIHDR,
};
use windows::Win32::Media::MMSYSERR_NOERROR;

/// The most bytes a message can have and still go through `midiOutShortMsg`.
///
/// Three: a status byte and at most two data bytes. Windows' own boundary, not a
/// choice this application makes.
const MAX_SHORT_MESSAGE_BYTES: usize = 3;

/// An open WinMM output device, and which device it is.
///
/// # Why the device index is carried alongside the handle
///
/// The index is positional and shifts as devices come and go, so the adapter has
/// to be able to notice that the device it wants is no longer the device this
/// handle points at. Holding the two together is what makes that check possible
/// without a second lookup.
pub struct OutputHandle {
    // The handle is a raw pointer, so the compiler will not move it across
    // threads on its own account — see the `Send` impl below.
    handle: HMIDIOUT,
    device_index: u32,
}

// SAFETY: WinMM handles are process-wide and thread-agnostic, exactly as the
// input handles in `receive` are. Windows documents them as usable from any
// thread, so the constraint is the compiler's conservatism about pointers rather
// than the platform's. Nothing here dereferences it, and this type owns the
// handle exclusively, so no two threads can close it.
unsafe impl Send for OutputHandle {}

impl OutputHandle {
    /// Opens the output device at `device_index`.
    ///
    /// # Errors
    ///
    /// [`CoreError::TransmitFailed`] when Windows will not open the device —
    /// most often because another program holds it. `target` names the device in
    /// the message, because "a device could not be opened" is not actionable and
    /// "loopMIDI Port could not be opened" is.
    pub fn open(device_index: u32, target: &str) -> Result<Self, CoreError> {
        let mut handle = HMIDIOUT::default();
        // SAFETY: `handle` is a live `HMIDIOUT` owned by this frame. No callback
        // is registered, so the two callback parameters are `None` and Windows
        // has nothing to call back into; the open flag says as much.
        let result =
            unsafe { midiOutOpen(&raw mut handle, device_index, None, None, CALLBACK_NULL) };
        if result != MMSYSERR_NOERROR {
            return Err(CoreError::TransmitFailed {
                target: target.to_owned(),
                detail: format!("Windows could not open the device (error {result})"),
            });
        }
        Ok(Self {
            handle,
            device_index,
        })
    }

    /// Which device this handle refers to.
    #[must_use]
    pub const fn device_index(&self) -> u32 {
        self.device_index
    }

    /// Sends one message, whole.
    ///
    /// # Errors
    ///
    /// [`CoreError::TransmitFailed`] when Windows refuses the message. Nothing
    /// partial is ever reported as success: the long path unprepares its buffer
    /// and reports the failure rather than leaving a half-sent transfer behind.
    pub fn send(&self, bytes: &[u8], target: &str) -> Result<(), CoreError> {
        if bytes.is_empty() {
            return Err(CoreError::TransmitFailed {
                target: target.to_owned(),
                detail: "there was nothing to send".to_owned(),
            });
        }
        if bytes.len() <= MAX_SHORT_MESSAGE_BYTES {
            self.send_short(bytes, target)
        } else {
            self.send_long(bytes, target)
        }
    }

    /// Sends a message of three bytes or fewer.
    ///
    /// The bytes are packed low-first into a `u32` — status in the lowest byte,
    /// then the data bytes in order. That layout is Windows', not a choice made
    /// here, and it is why the shift positions are literal rather than named.
    fn send_short(&self, bytes: &[u8], target: &str) -> Result<(), CoreError> {
        let mut packed: u32 = 0;
        for (position, byte) in bytes.iter().enumerate() {
            // `position` is below three and `u32::from` widens, so neither the
            // shift nor the or can lose information.
            let shift = u32::try_from(position * 8).unwrap_or(0);
            packed |= u32::from(*byte) << shift;
        }

        // SAFETY: the handle is open and owned by this value, and the message is
        // a plain integer with no memory for Windows to reach into.
        let result = unsafe { midiOutShortMsg(self.handle, packed) };
        if result != MMSYSERR_NOERROR {
            return Err(CoreError::TransmitFailed {
                target: target.to_owned(),
                detail: format!("Windows refused the message (error {result})"),
            });
        }
        Ok(())
    }

    /// Sends a System Exclusive transfer.
    ///
    /// # Why the buffer is boxed
    ///
    /// Windows reads the buffer through the pointer inside the `MIDIHDR` for the
    /// duration of the call. A `Vec` that moved between `midiOutPrepareHeader`
    /// and `midiOutLongMsg` would leave that pointer dangling, so the bytes are
    /// pinned in a `Box` that outlives every call that touches them.
    ///
    /// # Why this blocks until the transfer is done
    ///
    /// The device is opened with no callback, so `midiOutLongMsg` returns only
    /// once Windows has taken the data. That is what lets this report whole-or-
    /// nothing honestly: there is no window in which the call has returned and
    /// the transfer is still partly outstanding.
    fn send_long(&self, bytes: &[u8], target: &str) -> Result<(), CoreError> {
        let mut buffer = bytes.to_vec().into_boxed_slice();
        let length = u32::try_from(buffer.len()).map_err(|_| CoreError::TransmitFailed {
            target: target.to_owned(),
            detail: "that message is too long for Windows to send".to_owned(),
        })?;

        let mut header = MIDIHDR {
            lpData: windows::core::PSTR(buffer.as_mut_ptr()),
            dwBufferLength: length,
            dwBytesRecorded: length,
            ..MIDIHDR::default()
        };
        let header_size = u32::try_from(size_of::<MIDIHDR>()).unwrap_or(u32::MAX);

        let failure = |detail: String| CoreError::TransmitFailed {
            target: target.to_owned(),
            detail,
        };

        // SAFETY: `header` is a live `MIDIHDR` owned by this frame and points at
        // `buffer`, which is boxed and outlives every call below.
        let prepared = unsafe { midiOutPrepareHeader(self.handle, &raw mut header, header_size) };
        if prepared != MMSYSERR_NOERROR {
            return Err(failure(format!(
                "Windows could not prepare the transfer (error {prepared})"
            )));
        }

        // SAFETY: the header was prepared immediately above and its buffer is
        // still alive and still where the header says it is.
        let sent = unsafe { midiOutLongMsg(self.handle, &raw const header, header_size) };

        // Unprepared whether or not the send worked. Leaving a prepared header
        // behind would leak the lock Windows takes on the buffer, and a later
        // send through the same handle would then fail for a reason that has
        // nothing to do with it.
        // SAFETY: same header, still alive, and no transfer is outstanding —
        // `midiOutLongMsg` has returned by this point.
        let released = unsafe { midiOutUnprepareHeader(self.handle, &raw mut header, header_size) };

        if sent != MMSYSERR_NOERROR {
            return Err(failure(format!(
                "Windows refused the transfer (error {sent})"
            )));
        }
        if released != MMSYSERR_NOERROR {
            return Err(failure(format!(
                "Windows could not release the transfer buffer (error {released})"
            )));
        }
        Ok(())
    }
}

impl Drop for OutputHandle {
    /// Silences and closes the device.
    ///
    /// `midiOutReset` first, because closing a device with notes still sounding
    /// leaves them sounding: Windows turns off every note on every channel here,
    /// which is the courtesy a user expects from something that just sent them.
    fn drop(&mut self) {
        // SAFETY: the handle is open and owned by this value, and nothing else
        // can be using it — `drop` has exclusive access by definition.
        unsafe {
            // Both return a status this code cannot act on: the value is being
            // dropped and there is nowhere left to report a failure to.
            let _ = midiOutReset(self.handle);
            let _ = midiOutClose(self.handle);
        }
    }
}
