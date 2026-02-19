# vol

Native macOS volume control CLI. Sets system output volume using Core Audio directly - no AppleScript, no shell overhead.

## Installation

Build the binary:

```bash
cargo build --release
```

The compiled binary will be at:
```
target/release/vol
```

### System Installation (Optional)

To use `vol` from anywhere:

```bash
# Copy to a directory in your PATH
cp target/release/vol /usr/local/bin/

# Or use cargo install
cargo install --path .
```

## Usage

```bash
vol [VOLUME]
```

- **VOLUME**: Integer from 0 to 100 (percentage)
- **No arguments**: Does nothing (no-op)

### Examples

```bash
vol 0      # Mute (sets volume to 0% and mutes)
vol 50     # Set volume to 50%
vol        # No-op (does nothing)
```

## Features

- Direct Core Audio FFI calls (no osascript overhead)
- ~4ms execution time (5-6x faster than AppleScript)
- Auto-mute on volume 0 (complete silence)
- Auto-unmute when setting volume > 0
- M3-optimized binary (~312KB)
- Proper error handling with exit codes

## Limitations

- **macOS only**: Uses Core Audio APIs (no Windows/Linux support)
- **Default output only**: Controls system default output device, not specific devices
- **No input control**: Output volume only, not microphone/input
- **No GUI**: Command-line interface only
- **User permissions**: Requires standard user permissions (no root needed)

## FAQ

**Q: Why not just use `osascript`?**

A: `osascript` is 5-6x slower due to interpreter and process-spawning overhead. This tool calls Core Audio directly in ~4ms.

**Q: Can I use this in shell scripts?**

A: Yes. It returns proper exit codes (0 for success, 1 for errors) and is silent on success.

**Q: Does it work with Bluetooth headphones?**

A: Yes. It controls the system default output, which includes Bluetooth, USB, and built-in audio.

**Q: Why does `vol 0` mute the system?**

A: Setting volume to 0 in Core Audio can still have faint audio leakage. We also set the mute property for complete silence.

**Q: Can I get the current volume level?**

A: No. This tool is for setting volume only. Reading current volume is not implemented.

**Q: Does it work on Intel Macs?**

A: By default, no. It's optimized for M3. To support Intel Macs, you need to build a universal binary (see DEVELOPERS.md).

## License

MIT License - see LICENSE file for details.

---

**Note**: This is a personal tool written with AI assistance. While functional, it should not be considered production-ready software. Use at your own risk.
