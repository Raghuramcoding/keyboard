# macOS T.C.K App

Native macOS application for T.C.K (TalentCloud Keyboard) built with SwiftUI and the TCKClient Swift package.

## Features

- 🖥️ **3-Column Layout** — sidebar, terminal, chat panel
- 💻 **Full Terminal** — real-time shell with monospace font
- 💬 **Side Chat Panel** — AI assistant while coding
- 🔧 **Instance Management** — quick access sidebar
- ⌨️ **Keyboard Shortcuts** — CMD+N new instance, CMD+W close window
- 🔌 **Resilient Connection** — auto-reconnect with exponential backoff
- 📊 **Status Bar** — server health at a glance

## Requirements

- **macOS** 12.0+
- **Xcode** 15.0+
- **Swift** 5.9+
- **T.C.K Server** running on the network

## Installation & Setup

### 1. Open the Project

```bash
cd macos/TCK
open TCK.xcodeproj
```

### 2. Add TCKClient Dependency

In Xcode:
- **File → Add Packages**
- Enter: `https://github.com/yourusername/tck-swift.git`
- Select version/branch
- Add to target `TCK`

Or edit `Package.swift` to add local dependency:
```swift
.package(path: "../../swift/TCKClient")
```

### 3. Build & Run

```bash
# macOS development
Product → Run (⌘R)

# Build for distribution
Product → Build For → Running (or Archive)
```

## Architecture

```
TCKMacApp (entry point)
├─ AppViewModel (state management)
│  ├─ TCKClient (service layer)
│  └─ instances, selectedInstanceId, etc.
├─ ContentView (3-column layout)
│  ├─ InstancesSidebarView (left column)
│  ├─ TerminalViewMac (middle column)
│  ├─ ChatViewMac (right column)
│  └─ SettingsWindowMac (sheet)
└─ ViewModels
   ├─ TerminalViewModel
   ├─ ChatViewModel
   └─ SettingsViewModel
```

## Interface Layout

```
┌──────────────────────────────────────────────────────┐
│ File Edit View Window Help              ⚙️           │
├────────────────┬──────────────────────────────────────┤
│  Instances     │ Terminal (shell)  │ Chat Panel       │
│  ────────────  │ ──────────────    │ ──────────      │
│  + dev         │ $ ls -la          │ You:            │
│    ✓ prod      │ file1.txt         │ Hello!          │
│    test        │ file2.txt         │                 │
│    [online]    │ $ █               │ Assistant:      │
│    [offline]   │                   │ Hi there!       │
│    [↻]         │                   │                 │
│                │                   │ Type here...    │
│                │                   │ ────────────    │
└────────────────┴──────────────────────────────────────┘
```

### Left Sidebar (Instances)
- List of terminal sessions with status indicators
- Click to select instance
- Right-click for context menu (delete)
- + button to create new instance
- Server health indicator (green = online, red = offline)
- Refresh button (⟳)

### Center Column (Terminal)
- Real-time shell output with monospace font
- Command input at bottom
- Connection status (green circle = connected)
- Auto-scroll toggle
- Send button (or press Enter)

### Right Column (Chat)
- AI conversation thread
- User messages (right, blue)
- Assistant messages (left, gray)
- Text input with send button
- Typing indicator during generation

### Settings Window
- **General** — server URL, AI provider, model
- **Generation** — temperature slider, max tokens
- **Prompt** — system prompt text editor
- **API Keys** — secure Keychain editor for each provider

## Views & Components

### **ContentView**
3-column layout:
- Instances sidebar (250pt wide)
- Terminal in center (flexible)
- Chat panel right (300pt wide)

Switch between tabs on iOS, but on macOS all three are visible simultaneously.

### **InstancesSidebarView**
Left column with instance list:
- Status indicator (green = running, gray = idle, yellow = paused, red = error)
- Instance name and agent role
- Click to select
- Right-click to delete
- + button to create new

### **TerminalViewMac**
Center column with terminal:
- Monospace output (green on black)
- Command input field
- Connection status badge
- Auto-scroll toggle
- Toolbar with instance ID

### **ChatViewMac**
Right column with AI chat:
- Message bubbles with timestamps
- User/assistant role differentiation
- Typing indicator
- Input field with send button

### **SettingsWindowMac**
Modal window with TabView:
- Tab 1: Server, Provider, Model
- Tab 2: Temperature, Max Tokens
- Tab 3: System Prompt text editor
- Tab 4: API Key configuration
- Cancel / Save buttons (keyboard shortcuts)

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| ⌘N | New instance (in sidebar context) |
| ⌘, | Open Settings |
| ⌘W | Close window |
| ⌘Q | Quit app |
| Enter | Send command (terminal) or message (chat) |
| ⌘↻ | Refresh instances |
| ⌘⌘⌘ | Focus terminal input field |

## Usage Examples

### Creating & Running a Terminal Session

```
1. Sidebar: Tap + button
2. Enter instance name: "dev"
3. Tap Create
4. Click instance to select
5. Terminal: Type command: ls -la
6. Press Enter
7. See output in real-time
```

### Multi-Instance Workflow

```
1. Create 3 instances: "server", "client", "db"
2. Click "server" in sidebar → Terminal shows server output
3. Click "client" → Terminal switches to client shell
4. Click "db" → Terminal switches to db shell
5. Quick switching without losing connection state
```

### AI-Assisted Development

```
1. Type command in terminal: git status
2. See output, but unsure what to do next
3. Click in Chat panel (right side)
4. Type: "Should I commit these changes?"
5. AI responds with advice
6. Implement suggestions in terminal
7. Ask AI follow-up questions without losing terminal context
```

### Configuring AI Provider

```
1. Click ⚙️ gear in top-right
2. Go to "General" tab
3. Provider: Select "OpenAI"
4. Model: Enter "gpt-4"
5. Go to "API Keys" tab
6. Click "Configure" next to "OpenAI"
7. Paste API key, click Save
8. Close settings (⌘W)
9. Start using GPT-4 in chat
```

### Tuning Generation

```
1. Click ⚙️ gear
2. "Generation" tab
3. Temperature: drag slider to 1.5 (more creative)
4. Max Tokens: increase to 8192 (longer responses)
5. "Prompt" tab: customize system prompt
6. Click Save
7. New messages will use these settings
```

## Configuration

### Server URL
**Settings → General → Server URL**

- Local: `http://127.0.0.1:3000` (default)
- LAN: `http://192.168.1.100:3000`
- Remote: `http://tck.example.com:3000` (with SSL: `https://...`)

### API Keys
**Settings → API Keys**

1. Click provider (Anthropic, OpenAI, Google, Ollama)
2. Paste API key
3. Toggle "Show" to verify
4. Click Save (Keychain-encrypted)

### Generation Parameters
**Settings → Generation**

- **Temperature** — slider 0.0–2.0
  - 0.0 = deterministic (best for code)
  - 0.7 = balanced (default)
  - 1.5+ = creative (best for brainstorming)
- **Max Tokens** — 256–16384 (response length)

### System Prompt
**Settings → Prompt**

Custom instructions for AI:

```
You are an expert software engineer.
Focus on clean, efficient code.
Explain your reasoning.
```

## Troubleshooting

### App won't launch
- Ensure Xcode is updated to 15.0+
- Delete DerivedData: `rm -rf ~/Library/Developer/Xcode/DerivedData/TCK*`
- Rebuild: Product → Clean Build Folder (⇧⌘K), then Product → Build (⌘B)

### "Server connection failed"
- Verify tck-server is running: `cargo run -p tck-server`
- Check server URL in Settings matches actual IP/port
- Ping server: `ping 192.168.1.100` (replace with your IP)
- Check macOS firewall (System Preferences → Security & Privacy)

### Terminal not receiving input
- Ensure instance is selected (highlighted in sidebar)
- Click terminal input field to focus
- Check connection status (green circle in terminal toolbar)
- Check tck-server logs for WebSocket errors

### Chat not responding
- Verify API key is set (Settings → API Keys)
- Check Selected model matches provider (e.g., "gpt-4" for OpenAI)
- Ensure tck-ai service is running: `cd tck-ai && npm run dev`
- Check API key quota on provider dashboard (OpenAI, Anthropic, etc.)

### Settings not saving
- API keys: stored in Keychain (more secure)
- Other settings: stored in UserDefaults (`defaults write com.tck.app ...`)
- To reset: `defaults delete com.tck.app` and restart app

## Performance Tips

### Large Terminal Output
- Very large outputs (100MB+) may slow scrolling
- Use "Clear" button in terminal toolbar to reset
- Redirect output to file instead: `command > log.txt`

### Chat History
- In-memory only; restart app to clear
- Consider exporting chat for archival

### Network Optimization
- Use local server on LAN (lower latency)
- Enable auto-scroll only if needed (saves CPU)
- Auto-reconnect respects exponential backoff (1s, 2s, 4s, 8s, 16s, 32s max)

## Development

### SwiftUI Preview
Most views have `#Preview` blocks for live editing:

```swift
#Preview {
    ContentView()
        .environment(AppViewModel())
}
```

Select preview, click "Resume" after edits.

### Hot Reload
- Edit Swift file
- Preview updates automatically
- No need to rebuild entire project

### Debugging
- **Console output** — Xcode Debug Navigator (⌘⇧2)
- **Network requests** — check tck-server logs
- **WebSocket events** — Terminal class uses print() statements
- **Breakpoints** — Xcode line number gutter

### Logging
Add to any file:
```swift
import os
let logger = Logger(subsystem: "com.tck.app", category: "terminal")
logger.debug("WebSocket connected")
```

View logs in Console.app:
```bash
log stream --predicate 'process == "TCK"'
```

## File Structure

```
macos/TCK/TCK/
├── TCKMacApp.swift           # Entry point
├── Views/
│   └── ContentView.swift     # 3-column layout + modals
├── ViewModels/
│   ├── AppViewModel.swift    # Main state
│   ├── TerminalViewModel.swift
│   ├── ChatViewModel.swift
│   └── SettingsViewModel.swift
├── Assets.xcassets/          # App icon, colors, images
├── Localizable.strings       # i18n (future)
└── Info.plist                # App metadata
```

## Distribution

### Build for Release

```bash
# Build archive
Product → Build For → Archiving

# Export for distribution
Window → Organizer → Select latest archive → Export
```

### Code Signing
Set in Xcode:
1. Select project → Targets → TCK
2. Signing & Capabilities tab
3. Team: select your Apple Developer account
4. Bundle ID: `com.yourcompany.tck`

### Notarization (macOS 10.15+)
Required for App Store or direct distribution:

1. Archive the app
2. Submit for notarization: Xcode → Window → Organizer → Notarize
3. Wait for approval email
4. Staple ticket: `xcrun stapler staple TCK.app`

## Contributing

Bug reports and PRs welcome! See [CONTRIBUTING.md](../../CONTRIBUTING.md).

## License

MIT — see [LICENSE](../../LICENSE)

---

**Server Setup:** [CLAUDE.md](../../CLAUDE.md)  
**Swift Package Docs:** [swift/TCKClient/README.md](../../swift/TCKClient/README.md)  
**iOS App:** [ios/TCK/README.md](../../ios/TCK/README.md)
