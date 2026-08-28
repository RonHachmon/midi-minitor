//! Hearing about devices being plugged in and unplugged.
//!
//! # Why `CM_Register_Notification` and not `RegisterDeviceNotification`
//!
//! The obvious route needs a window handle, which would mean owning a
//! message-only window and a message-pump thread purely to hear about cables —
//! and it would mean this adapter borrowing something from the application's
//! window, which is exactly the coupling the macOS adapter is careful not to
//! have. Microsoft's own documentation points away from it: the newer call
//! requires no window handle, and this application's supported Windows floor is
//! comfortably above where it became available.
//!
//! See `specs/003-windows-support/research.md`, decision D4.
//!
//! # Why every notification is treated the same way
//!
//! Arrival, removal, and every other kind all do one thing: re-read the truth
//! from the system. Reacting to specific kinds would mean trusting the
//! notification to describe the change completely, and a rescan is cheap enough
//! that trusting it buys nothing. This mirrors the macOS adapter exactly, which
//! is the point — the two platforms report changes differently and respond to
//! them identically.
//!
//! # Why the debounce is shared with the macOS adapter rather than reimplemented
//!
//! One physical connection produces a burst of notifications on both platforms,
//! and rebuilding the Sources panel per notification would make it flicker on
//! both. The policy is platform-free, so it lives in the core and both adapters
//! use the same one — see [`midi_core::support::debounce`].

use std::mem::size_of;
use std::sync::Arc;

use midi_core::application::error::CoreError;
use midi_core::support::debounce::Debouncer;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Register_Notification, CM_Unregister_Notification, CM_NOTIFY_ACTION, CM_NOTIFY_EVENT_DATA,
    CM_NOTIFY_FILTER, CM_NOTIFY_FILTER_FLAG_ALL_INTERFACE_CLASSES,
    CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE, CR_SUCCESS, HCMNOTIFICATION,
};

/// What a notification callback returns to say it handled the event.
///
/// Named rather than written as a bare zero because the value is an operating
/// system contract, not an arbitrary success code of ours.
const NOTIFICATION_HANDLED: u32 = 0;

/// A live registration for device arrival and removal, and the rescan it drives.
///
/// # Why the callback state is leaked and reclaimed rather than borrowed
///
/// Windows holds the context pointer until the registration is cancelled and
/// calls into it from its own thread. A borrow could not outlive this function,
/// and anything movable could be moved while Windows still pointed at it. The
/// context is therefore pinned for exactly the lifetime of the registration and
/// freed in [`Drop`], after the registration is cancelled and the callback can no
/// longer run.
pub struct DeviceWatcher {
    handle: HCMNOTIFICATION,
    context: *mut RescanContext,
}

// SAFETY: the handle and the context pointer are only ever used to cancel the
// registration and then free the context, both on whichever thread drops this.
// Windows itself is what touches them concurrently, and it stops doing so before
// `CM_Unregister_Notification` returns.
unsafe impl Send for DeviceWatcher {}

/// What the device-change callback is allowed to touch.
///
/// # Why the rescan is shared rather than owned outright
///
/// The debounce deliberately runs the rescan *later*, on a thread of its own,
/// after the callback has returned. Cancelling the registration waits for
/// in-flight callbacks — it does not wait for a rescan the debounce has already
/// scheduled. A closure reaching back into this context would therefore be able
/// to outlive it. Sharing ownership means the scheduled rescan keeps alive
/// exactly what it needs, and the timing question disappears rather than being
/// reasoned about.
struct RescanContext {
    debouncer: Debouncer,
    rescan: Arc<dyn Fn() + Send + Sync>,
}

impl DeviceWatcher {
    /// Registers for device-interface changes and calls `rescan` when they settle.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::MidiSystemUnavailable`] when the registration is
    /// refused. That is the one failure meaning this adapter cannot do its job:
    /// without it the Sources list would silently stop following cables, which is
    /// worse than saying so.
    pub fn register<F>(rescan: F) -> Result<Self, CoreError>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let context = Box::into_raw(Box::new(RescanContext {
            debouncer: Debouncer::new(),
            rescan: Arc::new(rescan),
        }));

        let filter = CM_NOTIFY_FILTER {
            cbSize: size_of::<CM_NOTIFY_FILTER>() as u32,
            // Every interface class rather than the MIDI one specifically. The
            // response is a rescan either way, and asking for everything cannot
            // miss a device whose class this code guessed wrong. If the rescan
            // rate ever proves noisy, narrowing to a class GUID is the change —
            // it would make this quieter, never more correct.
            Flags: CM_NOTIFY_FILTER_FLAG_ALL_INTERFACE_CLASSES,
            FilterType: CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
            ..CM_NOTIFY_FILTER::default()
        };

        let mut handle = HCMNOTIFICATION::default();
        // SAFETY: `filter` is a live, fully initialised structure whose `cbSize`
        // matches its type; the callback is a real `extern "system"` function of
        // the required signature; `context` is pinned and outlives the
        // registration; and `handle` is a live local the call writes once.
        let result = unsafe {
            CM_Register_Notification(
                &raw const filter,
                Some(context.cast::<core::ffi::c_void>()),
                Some(on_device_change),
                &raw mut handle,
            )
        };
        if result != CR_SUCCESS {
            // SAFETY: registration failed, so Windows kept no pointer to the
            // context and it is ours to reclaim.
            drop(unsafe { Box::from_raw(context) });
            return Err(CoreError::MidiSystemUnavailable {
                detail: format!(
                    "Windows would not report device changes (error {}).",
                    result.0
                ),
            });
        }

        Ok(Self { handle, context })
    }
}

impl Drop for DeviceWatcher {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful registration. This call is
        // documented to wait for any in-flight callback to finish, which is what
        // makes freeing the context on the next line sound.
        unsafe {
            let _ = CM_Unregister_Notification(self.handle);
        }
        // SAFETY: the registration is cancelled and no callback can be running or
        // start, so the context is ours again.
        drop(unsafe { Box::from_raw(self.context) });
    }
}

/// Windows' device-change callback.
///
/// # Safety
///
/// Windows calls this on one of its own threads with `context` set to the pointer
/// given to `CM_Register_Notification`, which stays live until the registration
/// is cancelled.
unsafe extern "system" fn on_device_change(
    _notification: HCMNOTIFICATION,
    context: *const core::ffi::c_void,
    _action: CM_NOTIFY_ACTION,
    _event: *const CM_NOTIFY_EVENT_DATA,
    _event_size: u32,
) -> u32 {
    if context.is_null() {
        return NOTIFICATION_HANDLED;
    }
    // SAFETY: the pointer is the one handed to the registration, and the
    // registration outlives every call to this function.
    let rescan_context = unsafe { &*(context as *const RescanContext) };

    // The rescan runs on a thread of its own once the burst settles. Doing it
    // here would block a system notification thread, which would delay every
    // other device notification this process receives.
    //
    // The closure takes a share of the rescan rather than a reference to this
    // context, so it stays valid even if the registration is cancelled while a
    // debounced rescan is still pending.
    let rescan = Arc::clone(&rescan_context.rescan);
    rescan_context.debouncer.schedule(move || rescan());

    NOTIFICATION_HANDLED
}
