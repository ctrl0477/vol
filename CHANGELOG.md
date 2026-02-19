# Changelog

## 0.1.0 - 2026-02-19

### Features

- Direct Core Audio FFI calls using `objc2-core-audio` crate
- Set system volume from 0-100% via command line
- Auto-mute when volume is set to 0 for complete silence
- Auto-unmute when volume is set above 0
- No-op behavior when run without arguments
- Fast execution (~4ms per call, 5-6x faster than osascript)
- M3-optimized binary (312KB)
- Custom error handling with descriptive messages
- Input validation for numeric values and range (0-100)

### Technical

- Zero external runtime dependencies
- Single binary deployment
- M3 MacBook specific optimizations (`target-cpu=apple-m3`)
- Release profile optimized for size and speed
- Unit tests for input parsing
