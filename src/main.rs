//! Native macOS volume control CLI
//!
//! Fast system volume control using direct Core Audio FFI calls.
//! Optimized for M3 MacBooks with no external process spawning.

use objc2_core_audio::{
    kAudioDevicePropertyMute, kAudioDevicePropertyVolumeScalar,
    kAudioHardwarePropertyDefaultOutputDevice, kAudioObjectPropertyScopeOutput,
    AudioObjectGetPropertyData, AudioObjectPropertyAddress, AudioObjectSetPropertyData,
};
use std::fmt;

/// System object ID for default audio device queries
const SYSTEM_OBJECT: u32 = 1;

/// Property address for retrieving the default output device
static DEVICE_ADDRESS: AudioObjectPropertyAddress = AudioObjectPropertyAddress {
    mSelector: kAudioHardwarePropertyDefaultOutputDevice,
    mScope: kAudioObjectPropertyScopeOutput,
    mElement: 0,
};

/// Property address for setting master volume on a device
static VOLUME_ADDRESS: AudioObjectPropertyAddress = AudioObjectPropertyAddress {
    mSelector: kAudioDevicePropertyVolumeScalar,
    mScope: kAudioObjectPropertyScopeOutput,
    mElement: 0,
};

/// Property address for mute control
static MUTE_ADDRESS: AudioObjectPropertyAddress = AudioObjectPropertyAddress {
    mSelector: kAudioDevicePropertyMute,
    mScope: kAudioObjectPropertyScopeOutput,
    mElement: 0,
};

/// Errors that can occur during volume operations
#[derive(Debug)]
enum VolumeError {
    InvalidInput(&'static str),
    DeviceError(i32),
    SetError(i32),
}

impl fmt::Display for VolumeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VolumeError::InvalidInput(msg) => write!(f, "{}", msg),
            VolumeError::DeviceError(code) => write!(f, "Failed to get audio device: {}", code),
            VolumeError::SetError(code) => write!(f, "Failed to set volume: {}", code),
        }
    }
}

impl std::error::Error for VolumeError {}

/// Parses volume percentage from string input.
///
/// Accepts values 0-100. Returns scalar float (0.0-1.0) on success.
fn parse_volume(input: &str) -> Result<f32, VolumeError> {
    let percent: f32 = input
        .parse()
        .map_err(|_| VolumeError::InvalidInput("Invalid number"))?;

    if !(0.0..=100.0).contains(&percent) {
        return Err(VolumeError::InvalidInput("Volume must be 0-100"));
    }

    Ok(percent / 100.0)
}

/// Retrieves the default audio output device ID.
///
/// SAFETY: This function makes FFI calls to Core Audio. It is safe because:
/// - All pointers are valid references to properly aligned stack variables
/// - The system object (ID 1) always exists on macOS
/// - The operation is read-only and thread-safe
fn get_default_device() -> Result<u32, VolumeError> {
    let mut device_id: u32 = 0;
    let mut data_size: u32 = std::mem::size_of::<u32>() as u32;

    let status = unsafe {
        AudioObjectGetPropertyData(
            SYSTEM_OBJECT,
            std::ptr::NonNull::new_unchecked(&raw const DEVICE_ADDRESS as *mut _),
            0,
            std::ptr::null(),
            std::ptr::NonNull::new_unchecked(&raw mut data_size),
            std::ptr::NonNull::new_unchecked(&raw mut device_id as *mut _),
        )
    };

    if status != 0 {
        Err(VolumeError::DeviceError(status))
    } else {
        Ok(device_id)
    }
}

/// Sets the mute state on the specified audio device.
///
/// SAFETY: This function makes FFI calls to Core Audio. It is safe because:
/// - The device_id is validated by a successful call to get_default_device()
/// - The mute_value pointer points to valid, aligned stack memory
/// - The size matches exactly what Core Audio expects (u32 for boolean)
fn set_mute(device_id: u32, muted: bool) -> Result<(), VolumeError> {
    let mute_value: u32 = if muted { 1 } else { 0 };

    let status = unsafe {
        AudioObjectSetPropertyData(
            device_id,
            std::ptr::NonNull::new_unchecked(&raw const MUTE_ADDRESS as *mut _),
            0,
            std::ptr::null(),
            std::mem::size_of::<u32>() as u32,
            std::ptr::NonNull::new_unchecked(&raw const mute_value as *mut _),
        )
    };

    if status != 0 {
        Err(VolumeError::SetError(status))
    } else {
        Ok(())
    }
}

/// Sets the master volume on the specified audio device.
///
/// SAFETY: This function makes FFI calls to Core Audio. It is safe because:
/// - The device_id is validated by a successful call to get_default_device()
/// - The volume pointer points to valid, aligned stack memory
/// - The size matches exactly what Core Audio expects
///
/// Auto-mute behavior:
/// - When volume is 0, the device is muted for complete silence
/// - When volume > 0, the device is unmuted to ensure audio plays
fn set_volume(device_id: u32, volume: f32) -> Result<(), VolumeError> {
    let status = unsafe {
        AudioObjectSetPropertyData(
            device_id,
            std::ptr::NonNull::new_unchecked(&raw const VOLUME_ADDRESS as *mut _),
            0,
            std::ptr::null(),
            std::mem::size_of::<f32>() as u32,
            std::ptr::NonNull::new_unchecked(&raw const volume as *mut _),
        )
    };

    if status != 0 {
        return Err(VolumeError::SetError(status));
    }

    // Auto-mute when volume is 0 for complete silence
    // Auto-unmute when volume > 0 to ensure audio plays
    set_mute(device_id, volume == 0.0)?;

    Ok(())
}

fn main() {
    let input = match std::env::args().nth(1) {
        Some(arg) => arg,
        None => return, // No-op if no arguments provided
    };

    let volume = match parse_volume(&input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let device_id = match get_default_device() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = set_volume(device_id, volume) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_volume_valid() {
        assert_eq!(parse_volume("0").unwrap(), 0.0);
        assert_eq!(parse_volume("50").unwrap(), 0.5);
        assert_eq!(parse_volume("100").unwrap(), 1.0);
    }

    #[test]
    fn test_parse_volume_invalid_input() {
        assert!(matches!(
            parse_volume("abc"),
            Err(VolumeError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_parse_volume_out_of_range() {
        assert!(matches!(
            parse_volume("-10"),
            Err(VolumeError::InvalidInput(_))
        ));
        assert!(matches!(
            parse_volume("200"),
            Err(VolumeError::InvalidInput(_))
        ));
    }
}
