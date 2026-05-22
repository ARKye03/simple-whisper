## [0.9.0] - 2026-05-22

### 🚀 Features

- Structured error handling with provider-aware classification

### 🚜 Refactor

- Consolidate provider and error classification logic

### 📚 Documentation

- Refresh CLAUDE.md for bilingual i18n and history sidebar
## [0.8.1] - 2026-05-22

### ⚙️ Miscellaneous Tasks

- Automate changelog generation with git-cliff
- Clean up cliff config and improve changelog generation
## [0.8.0] - 2026-05-22

### 🚀 Features

- Add transcription history sidebar with persistent storage
## [0.7.0] - 2026-05-18

### 🚀 Features

- Add multi-language UI support with i18n system

### 🐛 Bug Fixes

- Improve i18n language detection fallback
## [0.6.0] - 2026-05-18

### 🚀 Features

- Replace Keychain with encrypted secrets storage

### 🚜 Refactor

- Improve encrypted secrets migration robustness
## [0.5.0] - 2026-05-18

### 🚀 Features

- Remove output format selection feature

### 🚜 Refactor

- Improve TranscriptPanel action buttons UI and accessibility
## [0.4.0] - 2026-05-18

### 🚀 Features

- Implement auto-update functionality
- Finalize auto-update configuration with signing key
## [0.3.0] - 2026-05-18

### 🚀 Features

- Add Gemini transcription support with provider abstraction
- Add gemini-3-flash-preview model support

### 🐛 Bug Fixes

- Add MIME type detection for audio formats and prevent API key race condition
- Use POSIX-compatible sed approach in bump script

### 💼 Other

- Replace pnpm version with inline Node.js for package.json updates

### 📚 Documentation

- Update README for Google Gemini provider support

### ⚙️ Miscellaneous Tasks

- Add version bump script and source version from package.json
## [0.2.1] - 2026-05-17

### 🐛 Bug Fixes

- Enable keyring platform backends, bump to 0.2.1

### 📚 Documentation

- Add installation instructions for release builds
## [0.2.0] - 2026-05-17

### ⚙️ Miscellaneous Tasks

- Improve icon
- New app icon
- Bump version to 0.2.0
## [0.1.0] - 2026-05-17

### 🚀 Features

- Implement complete transcription workflow with Groq API
- Implement multi-file transcription queue with drag-drop UI
- Add keychain integration for secure API key storage

### 🐛 Bug Fixes

- Improve error handling in settings and transcript download

### 🚜 Refactor

- Replace conditional rendering with class-based drawer states
- Migrate settings to reactive store pattern
- Replace tauri-plugin-shell with tokio subprocess execution

### 📚 Documentation

- Add comprehensive implementation guide for simple-whisper
- Add comprehensive README and screenshot for Simple Whisper

### ⚙️ Miscellaneous Tasks

- Update app icons and branding assets for all platforms
- Add GitHub Actions release workflow for tauri builds
- Bump action versions and use pnpm 11
