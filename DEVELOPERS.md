# Developer Documentation

## Overview

`vol` is a native macOS volume control CLI that uses Core Audio FFI directly. This document explains the codebase structure, testing approach, and development practices.

## Running Tests

### Unit Tests

Run all tests:
```bash
cargo test
```

Run tests with output:
```bash
cargo test -- --nocapture
```

Run specific test:
```bash
cargo test test_parse_volume_valid
```

### Test Structure

Tests are embedded in `src/main.rs` using `#[cfg(test)]` modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_volume_valid() { ... }
}
```

### Test Categories

1. **Input Validation Tests**
   - `test_parse_volume_valid`: Tests valid numeric inputs (0, 50, 100)
   - `test_parse_volume_invalid_input`: Tests non-numeric rejection
   - `test_parse_volume_out_of_range`: Tests out-of-bounds rejection

2. **Note on Audio Tests**
   - We do NOT test volume levels > 10 in automated tests
   - This prevents unexpected loud audio during development
   - Manual testing required for: `vol 50`, `vol 100`
   - Always test with headphones removed or at low volume first

## Code Architecture

### Core Components

```
main()
├── parse_volume(input) → Result<f32, VolumeError>
├── get_default_device() → Result<u32, VolumeError>
├── set_volume(device_id, volume) → Result<(), VolumeError>
└── set_mute(device_id, muted) → Result<(), VolumeError>
```

### FFI Safety

All Core Audio FFI calls are wrapped in `unsafe` blocks with SAFETY comments:

```rust
// SAFETY: AudioObjectGetPropertyData is thread-safe for read-only
// device queries. Valid pointers provided with correct sizes.
unsafe {
    AudioObjectGetPropertyData(...)
}
```

**Key Invariants:**
- Stack-allocated data with proper lifetimes
- Pointer casts use `&raw const` syntax (avoids reference semantics)
- All return values (OSStatus) checked
- `NonNull` wrapper prevents null pointer dereferences

### FFI Safety Deep Dive

**Understanding the `unsafe` blocks:**

All Core Audio calls use C APIs requiring `unsafe`. Here's why each is safe:

#### 1. `&raw const` vs Regular References

```rust
&raw const VOLUME_ADDRESS as *mut _
```

**Why not just `&VOLUME_ADDRESS`?**
- `&raw const` creates a raw pointer directly without creating an intermediate reference
- Avoids Rust's reference validity rules (noaliasing, lifetime constraints)
- For FFI, we need raw pointers anyway, so this skips the intermediate step
- Prevents undefined behavior from reference-to-pointer-to-reference round trips

#### 2. `NonNull::new_unchecked()`

```rust
NonNull::new_unchecked(&raw const VOLUME_ADDRESS as *mut _)
```

**Why is this safe?**
- We control the static address (it's always valid)
- Never null (static allocation)
- Core Audio API expects `NonNull` for type safety
- The `unchecked` is fine because we guarantee non-null at compile time

#### 3. Mutable Pointers from Const Data

```rust
&raw const mute_value as *mut u32
```

**Casting `const` to `*mut`:**
- Looks suspicious but is safe for FFI
- Core Audio writes to this memory (out-parameter pattern)
- Stack variable lives long enough (synchronous call)
- No data races (single-threaded CLI tool)

#### 4. `mElement: 0` (Master Channel)

```rust
mElement: 0,  // Master channel
```

**Why 0?**
- `0` = master/mono channel (affects all speakers equally)
- `1` = left channel only
- `2` = right channel only
- For system volume, we always want master (0)

#### 5. OSStatus Error Codes

Core Audio returns `i32` status codes:
- `0` = success (`kAudioHardwareNoError`)
- Negative = various errors (device not found, permission denied, etc.)
- We check `if status != 0` and propagate errors

**Common error codes:**
- `-1` (`kAudioHardwareNotRunningError`)
- `-2` (`kAudioHardwareUnspecifiedError`)
- `-3` (`kAudioHardwareUnknownPropertyError`)

### Error Handling

Custom `VolumeError` enum:
```rust
pub enum VolumeError {
    InvalidInput(String),
    DeviceError(i32),
    SetError(i32),
}
```

- Implements `std::error::Error` and `std::fmt::Display`
- Main returns `Result<(), Box<dyn std::error::Error>>`
- Exit codes: 0 (success), 1 (any error)

### Key Design Decisions

1. **Auto-Mute Behavior**
   - Volume 0 automatically mutes the device
   - Volume > 0 automatically unmutes
   - Ensures complete silence at 0 (not just minimum volume)

2. **No-Op Default**
   - Running `vol` without arguments does nothing
   - Safer than assuming a default volume
   - Users must explicitly specify desired volume

3. **Input Validation**
   - Strict numeric parsing
   - Range check: 0.0 - 100.0
   - Early exit with descriptive error messages

4. **Performance**
   - Direct Core Audio FFI (no osascript)
   - 5-6x faster than AppleScript alternatives
   - ~4ms execution time

## Development Workflow

### Building

Development build:
```bash
cargo build
```

Release build (optimized):
```bash
cargo build --release
```

### Code Quality

Format code:
```bash
cargo fmt
```

Check for warnings:
```bash
cargo clippy
```

Build and verify:
```bash
cargo build --release && cargo clippy && cargo test
```

### M3 Optimization

The project is configured for M3 MacBooks:
- `.cargo/config.toml` sets `target-cpu=apple-m3`
- Not compatible with Intel Macs (by design)
- Optimized for Apple Silicon performance

## Adding Features

### Adding a New Test

1. Add test function in `#[cfg(test)]` module
2. Follow naming convention: `test_<function_name>_<scenario>`
3. Test both success and error cases
4. Run `cargo test` to verify

### Adding New FFI Calls

1. Import required constants from `objc2_core_audio`
2. Create static `AudioObjectPropertyAddress` if reusable
3. Add SAFETY comment explaining invariants
4. Check OSStatus return value
5. Wrap in proper error handling

Example:
```rust
// SAFETY: Explain why this call is safe
let status = unsafe {
    AudioObjectSetPropertyData(
        device_id,
        NonNull::new_unchecked(&raw const PROPERTY as *mut _),
        0,
        null(),
        size_of::<Type>() as u32,
        NonNull::new_unchecked(&raw const value as *mut _),
    )
};
```

## Common Pitfalls

1. **Volume vs Mute**: These are separate properties in Core Audio
   - Volume scalar (0.0-1.0) controls loudness
   - Mute property (0/1) controls complete silence
   - Setting volume to 0 ≠ muting (hence auto-mute feature)

2. **Pointer Types**: Core Audio expects specific pointer types
   - Use `NonNull<T>` wrapper for non-null guarantees
   - Cast `&raw const T` to `*mut T` for FFI compatibility
   - Double-check size parameters match data type

3. **Element Scope**: `mElement: 0` means master channel
   - Individual channels would use 1, 2, etc.
   - Always use master (0) for system volume control

4. **Error Messages**: Keep them concise
   - Current: "Invalid number", "Volume must be 0-100"
   - Avoid verbose explanations in error output
   - Users want quick actionable feedback

## Performance Considerations

- **Binary Size**: Optimized release is ~312KB
- **Execution Time**: ~4ms per call
- **Memory**: Minimal heap allocation (none in hot path)
- **Syscalls**: 2-3 Core Audio API calls per invocation

## Testing on Real Hardware

Since this interfaces with system audio:

1. **Safe Testing** (always):
   ```bash
   cargo test  # Unit tests only, safe volumes
   ./target/release/vol 0    # Mute
   ./target/release/vol 5    # Low volume
   ./target/release/vol 10   # Still safe
   ```

2. **Manual Testing** (with caution):
   ```bash
   ./target/release/vol 50   # Remove headphones first
   ./target/release/vol 100  # Careful - very loud
   ```

3. **Continuous Testing**:
   ```bash
   # Safe loop
   for i in 0 5 10; do ./target/release/vol $i; done
   ```

## Troubleshooting

### Build Failures

- **Missing macOS SDK**: Install Xcode Command Line Tools
  ```bash
  xcode-select --install
  ```

- **Architecture mismatch**: Ensure running on M3 Mac or adjust `.cargo/config.toml`

### Runtime Issues

- **Permission denied**: Core Audio requires user permissions (normal)
- **Device not found**: Check default output device in System Preferences
- **Volume not changing**: Verify not muted at system level first

## Release Checklist

Before releasing:

- [ ] `cargo test` passes
- [ ] `cargo clippy` shows no warnings
- [ ] `cargo fmt` shows no changes needed
- [ ] `cargo build --release` succeeds
- [ ] Binary size ~312KB
- [ ] Manual test with: 0, 5, 10, 50, 100
- [ ] Update CHANGELOG.md
- [ ] Tag version in git

## Workarounds

### M-Series Compatibility

The project is configured for **M3 MacBooks only** via `.cargo/config.toml`:
```toml
[build]
rustflags = ["-C", "target-cpu=apple-m3"]
```

**To support all M-series MacBooks (M1, M2, M3, M4):**

Change the target CPU from `apple-m3` to `apple-m1`:
```toml
[build]
rustflags = ["-C", "target-cpu=apple-m1"]
```

**Why this works:**
- M1 instructions are baseline for Apple Silicon
- All newer M-series chips maintain backward compatibility with M1
- M3-specific optimizations won't be used, but the code runs correctly
- Binary size may increase slightly (5-10KB typically)

**Trade-offs:**
- ✅ Supports all M-series Macs with one binary
- ⚠️ Slightly less optimized for M3 specifically
- ⚠️ Cannot use M3-specific CPU features

**Alternative:** Build separate binaries for each architecture:
```bash
# For M1/M2 users
cargo build --release --target aarch64-apple-darwin

# For M3 users (with config.toml)
cargo build --release
```

### Universal Binary

To create a universal binary supporting both Intel and Apple Silicon:

```bash
# Build for both architectures
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Combine with lipo
lipo -create -output target/release/vol-universal \
    target/x86_64-apple-darwin/release/vol \
    target/aarch64-apple-darwin/release/vol
```

**Note:** Core Audio APIs are identical across architectures, so this works seamlessly.

## Resources

- **Core Audio Documentation**: [Apple Developer](https://developer.apple.com/documentation/coreaudio)
- **objc2-core-audio Crate**: [GitHub](https://github.com/madsmtm/objc2)
- **Rust FFI Guide**: [The Rust Book](https://doc.rust-lang.org/nomicon/ffi.html)

## Contact

This is a personal tool. For issues or improvements, check the project repository.
