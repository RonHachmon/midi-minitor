//! The System Exclusive buffer pool WinMM requires the application to own.
//!
//! # Why this module exists at all, when the macOS adapter has no counterpart
//!
//! CoreMIDI hands over System Exclusive bytes in the same callback as everything
//! else, in memory it owns. WinMM does not: it will only deliver a transfer into
//! a buffer the *application* supplied in advance, and it hands that buffer back
//! full and expects it to be returned before the next transfer can use it. A port
//! opened with no buffers queued receives no System Exclusive at all — silently,
//! with every other message still arriving, which is the worst possible failure
//! for a monitor.
//!
//! So each open port owns a small pool. Buffers are prepared and queued when the
//! port opens, returned to the driver after each transfer, and unprepared when
//! the port closes.
//!
//! # Why the buffers are leaked into raw pointers rather than held in a `Vec`
//!
//! The driver keeps a pointer to each header for as long as it is queued, and
//! writes through it from its own thread. Anything that could move the header
//! while it is queued — a vector reallocating, a struct being returned by value —
//! would leave the driver writing into freed memory. Boxing each header and
//! keeping the raw pointer is what pins it, and [`BufferPool::release`] is what
//! reclaims it once the driver has been told to give it back.
//!
//! # Why nothing here runs on the callback thread
//!
//! `midiInAddBuffer` is a multimedia function, and the input callback may not
//! call one. Every method here therefore runs on the adapter's worker thread or
//! on the thread that opens and closes ports.

use std::mem::size_of;

use windows::Win32::Media::Audio::{
    midiInAddBuffer, midiInPrepareHeader, midiInUnprepareHeader, HMIDIIN, MIDIHDR,
};
use windows::Win32::Media::MMSYSERR_NOERROR;

/// How many bytes one System Exclusive buffer holds.
///
/// Large enough for the dumps real instruments send — a full synthesiser patch
/// bank is the usual worst case — without reserving a megabyte per port. A
/// transfer longer than this is not lost: WinMM fills this buffer, hands it back,
/// and continues into the next one, and the decoder joins the pieces into one
/// message.
const BUFFER_BYTES: usize = 16 * 1024;

/// How many buffers each open port keeps queued.
///
/// More than one so that a transfer arriving while the previous buffer is still
/// being processed has somewhere to go. Four is enough to cover the round trip on
/// any machine this application targets, and is cheap at this buffer size.
const BUFFER_COUNT: usize = 4;

/// The System Exclusive buffers one open port has queued with the driver.
///
/// # Why this owns raw pointers rather than boxes
///
/// Each header is handed to the driver, which holds it until the port is reset.
/// A `Box` here would be dropped on unwind or on an early return while the driver
/// still had the pointer. Keeping raw pointers makes reclaiming them a deliberate
/// step — [`Self::release`] — that happens only after the driver has been told to
/// let go.
pub struct BufferPool {
    headers: Vec<*mut MIDIHDR>,
}

// SAFETY: the pool is only ever touched by one thread at a time — the thread that
// opens the port, and afterwards the adapter's worker thread. The driver writes
// through the queued headers, but never through this structure. What is not safe,
// and is not done, is touching a header while it is queued.
unsafe impl Send for BufferPool {}

impl BufferPool {
    /// Prepares and queues this port's buffers.
    ///
    /// A buffer that cannot be prepared or queued is dropped rather than
    /// retained: a half-registered header is one the driver may or may not hold,
    /// and guessing wrong either leaks it or frees it underneath the driver. The
    /// port still works with the buffers that did register.
    #[must_use]
    pub fn attach(port: HMIDIIN) -> Self {
        let mut headers = Vec::with_capacity(BUFFER_COUNT);

        for _ in 0..BUFFER_COUNT {
            // The data buffer is leaked deliberately: the header points into it
            // and the driver writes into it, so it must not move or be freed
            // while queued. `release` reclaims both together.
            let data = vec![0u8; BUFFER_BYTES].into_boxed_slice();
            let data = Box::into_raw(data);

            let header = Box::into_raw(Box::new(MIDIHDR {
                // `PSTR` is the crate's spelling for a byte pointer here; the
                // buffer holds arbitrary MIDI bytes, not text.
                lpData: windows::core::PSTR(data.cast::<u8>()),
                dwBufferLength: BUFFER_BYTES as u32,
                ..MIDIHDR::default()
            }));

            // SAFETY: `header` points to a live, pinned `MIDIHDR` whose `lpData`
            // points to a live buffer of exactly `dwBufferLength` bytes, which is
            // what both calls require. `port` is open.
            let prepared = unsafe {
                midiInPrepareHeader(port, header, size_of::<MIDIHDR>() as u32) == MMSYSERR_NOERROR
                    && midiInAddBuffer(port, header, size_of::<MIDIHDR>() as u32)
                        == MMSYSERR_NOERROR
            };

            if prepared {
                headers.push(header);
            } else {
                // SAFETY: the driver rejected the header, so it holds no pointer
                // to either allocation and both are ours to reclaim.
                unsafe {
                    drop(Box::from_raw(header));
                    drop(Box::from_raw(data));
                }
            }
        }

        Self { headers }
    }

    /// Returns one filled buffer to the driver so it can be used again.
    ///
    /// Called from the worker thread once the transfer's bytes have been copied
    /// out — never from the input callback, which may not call this.
    ///
    /// # Safety
    ///
    /// `header` must be a header [`Self::attach`] prepared for `port`, which the
    /// driver has just handed back and which is not currently queued, and `port`
    /// must still be open. Re-queueing a header the driver already holds, or one
    /// belonging to a closed port, hands the driver a pointer it may write
    /// through after this process has freed it.
    pub unsafe fn requeue(port: HMIDIIN, header: *mut MIDIHDR) {
        // SAFETY: guaranteed by this function's own contract, which the worker
        // upholds by only re-queueing a header the callback just received.
        let _ = unsafe { midiInAddBuffer(port, header, size_of::<MIDIHDR>() as u32) };
    }

    /// Unprepares and frees every buffer.
    ///
    /// The caller **must** have reset the port first. `midiInReset` is what makes
    /// the driver hand every queued buffer back; unpreparing one the driver still
    /// holds fails, and freeing it would leave the driver writing into memory
    /// that no longer belongs to this process.
    pub fn release(&mut self, port: HMIDIIN) {
        for header in self.headers.drain(..) {
            // SAFETY: the port has been reset, so the driver has released this
            // header and both allocations are ours again. `lpData` is the pointer
            // `attach` leaked, reconstructed with the same length.
            unsafe {
                let _ = midiInUnprepareHeader(port, header, size_of::<MIDIHDR>() as u32);
                let owned = Box::from_raw(header);
                let data =
                    std::slice::from_raw_parts_mut(owned.lpData.0, owned.dwBufferLength as usize);
                drop(Box::from_raw(data as *mut [u8]));
                drop(owned);
            }
        }
    }
}
