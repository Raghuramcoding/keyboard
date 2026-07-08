# iOS T.C.K App

Native iOS application for T.C.K (TalentCloud Keyboard) built with SwiftUI and the TCKClient Swift package.

## Features

- 📱 **Responsive Design** — optimized for iPhone and iPad
- 💻 **Terminal View** — real-time shell access over WebSocket
- 💬 **Chat Panel** — AI assistant conversation
- 🔧 **Instance Management** — create, list, delete terminal sessions
- ⚙️ **Settings** — API keys, provider configuration, generation parameters
- 🔌 **Offline Support** — graceful handling of disconnections with auto-reconnect

## Requirements

- **iOS** 15.0+
- **Xcode** 15.0+
- **Swift** 5.9+
- **T.C.K Server** running on the network

## Installation & Setup

### 1. Open the Project

```bash
cd ios/TCK
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
.package(path: "../swift/TCKClient")
```

### 3. Build & Run

```bash
# Select iPhone simulator or device
Product → Run (⌘R)
```

## Architecture

```
TCKApp (entry point)
├─ AppViewModel (state management)
│  ├─ TCKClient (service layer)
│  └─ instances, selectedInstanceId, etc.
├─ ContentView (tab navigation)
│  ├─ InstancesView (SwiftUI List)
│  ├─ TerminalView (PTY terminal)
│  ├─ ChatView (AI messages)
│  └─ SettingsView (configuration)
└─ ViewModels
   ├─ TerminalViewModel
   ├─ ChatViewModel
   └─ SettingsViewModel
```

## Views

### **ContentView**
Tab-based navigation between main features:
- **Instances** — list and create terminal sessions
- **Terminal** — shell access (if instance selected)
- **Chat** — AI conversation (if instance selected)
- **Settings** — app configuration

### **InstancesView**
- List all terminal instances
- Create new instance (sheet)
- Select instance to focus
- Delete instance (swipe action)
- Pull-to-refresh

**Interactions:**
```swift
// Tap to select
// Swipe left to delete
// Pull down to refresh
```

### **TerminalView**
- Real-time terminal output (scrollable)
- Command input field
- Connection status indicator
- Auto-scroll toggle
- Send on Enter key

**Usage:**
```swift
// Type command
// Press Enter or tap send
// Output appears live
```

### **ChatView**
- Message thread with timestamps
- User messages (right-aligned, blue)
- Assistant messages (left-aligned, gray)
- Typing indicator during generation
- Auto-scroll to latest message

**Usage:**
```swift
// Type message
// Tap send or press Enter
// Wait for AI response
```

### **SettingsView**
- **Server** — hostname/port
- **AI Provider** — anthropic, openai, google, ollama
- **Model** — model identifier
- **Temperature** — 0.0 to 2.0 slider
- **Max Tokens** — 256 to 16384 stepper
- **System Prompt** — custom instructions
- **API Keys** — secure Keychain storage

## ViewModels

### **AppViewModel**
Main state container:
- `instances: [Instance]` — list of terminal sessions
- `selectedInstanceId: String?` — active instance
- `serverHealth: Bool` — server connectivity
- `errorMessage: String?` — last error

**Methods:**
```swift
func initialize() async — load initial state
func refreshInstances() async — fetch latest instances
func createNewInstance(name:agentRole:) async throws
func deleteInstance(_:) async throws
func selectInstance(_:) — set active instance
```

### **TerminalViewModel**
PTY terminal management:
- `output: String` — accumulated terminal output
- `isConnected: Bool` — WebSocket connection state
- `errorMessage: String?` — terminal errors

**Methods:**
```swift
func connect() — open PTY WebSocket
func disconnect() — close PTY
func sendCommand(_:) async throws — execute shell command
func resize(rows:cols:) async throws — terminal window size
```

### **ChatViewModel**
AI conversation:
- `messages: [ChatMessage]` — chat history
- `isLoading: Bool` — generation in progress
- `availableModels: [CatalogModel]` — AI models list

**Methods:**
```swift
func sendMessage(_:) async — send user message
func fetchModels() async — load available models
```

### **SettingsViewModel**
Configuration management:
- `serverURL: String`
- `selectedProvider: String`
- `selectedModel: String`
- `temperature, maxTokens, systemPrompt: ...`

**Methods:**
```swift
func save() — persist to UserDefaults
func setAPIKey(_:for:) throws — Keychain storage
func getAPIKey(for:) throws -> String? — Keychain retrieval
```

## Configuration

### Server URL
**Settings → Server URL**

Default: `http://127.0.0.1:3000`

For remote server:
- Enter: `http://192.168.1.100:3000` (replace with your IP)
- Ensure firewall allows port 3000

### API Keys
**Settings → API Keys**

Store securely in Keychain:
1. Tap "Configure" next to provider (Anthropic, OpenAI, etc.)
2. Paste API key
3. Tap "Save"

Keys are encrypted on-device and never logged.

### AI Generation
**Settings → Generation Parameters**

- **Temperature** — 0.0 = deterministic, 1.0 = creative, 2.0 = wild
- **Max Tokens** — response length (256–16384)
- **System Prompt** — role/instructions for the AI

## Usage Examples

### Creating a Terminal Instance

```
1. Tap "Instances" tab
2. Tap + button (top-right)
3. Enter name: "dev"
4. Tap "Create"
5. Tap instance to select
6. Go to "Terminal" tab
7. Start typing commands
```

### Asking AI for Help

```
1. Select instance (if needed for context)
2. Tap "Chat" tab
3. Type question: "How do I list files?"
4. Send
5. Read AI response
6. Continue conversation
```

### Changing AI Provider

```
1. Tap "Settings" tab
2. Choose Provider: "OpenAI"
3. Choose Model: "gpt-4"
4. Add API key: Settings → API Keys → Configure → paste key
5. Adjust Temperature/Max Tokens if needed
6. Tap Save
7. Go back to Chat and send message
```

## Troubleshooting

### "Cannot reach tck-server"
- Verify server is running: `cargo run -p tck-server`
- Check server URL in Settings matches actual server address
- On LAN: use server IP (e.g., `http://192.168.1.100:3000`)
- Ensure iPhone/simulator is on same network

### Terminal disconnects frequently
- Check network stability
- Auto-reconnect will activate (max 5 attempts)
- Check tck-server logs for errors

### AI not responding
- Verify API key is set in Settings → API Keys
- Check selected Model matches provider (e.g., "gpt-4" for OpenAI)
- Ensure tck-ai service is running: `cd tck-ai && npm run dev`
- Check remaining API quota on provider dashboard

### Settings not persisting
- App uses UserDefaults (not persisted between deletes)
- API keys stored in Keychain (persists)
- Delete and reinstall app to reset

## Performance Tips

- **Terminal Output** — very large outputs may slow scrolling. Clear output with "Clear" button if needed.
- **Chat History** — message history is in-memory. Restart app to clear.
- **Connection** — WebSocket connects automatically; manual disconnect in settings saves battery.

## Development

### Hot Reload
Xcode Preview uses live SwiftUI previews:
```swift
#Preview {
    InstancesView()
        .environment(AppViewModel())
}
```

Tap "Resume" in preview after code changes.

### Debugging
- **Terminal output** — Xcode console (⌘⇧Y)
- **Network requests** — check tck-server logs
- **WebSocket events** — Terminal class logs to console

### Adding New Features
1. Add new View in `Views/`
2. Create ViewModel in `ViewModels/`
3. Integrate into `ContentView` tab
4. Update `AppViewModel` if state is needed
5. Test with Preview or simulator

## File Structure

```
ios/TCK/TCK/
├── TCKApp.swift              # Entry point
├── Views/
│   ├── ContentView.swift     # Tab navigation
│   ├── InstancesView.swift   # Instance list
│   ├── TerminalView.swift    # PTY terminal
│   ├── ChatView.swift        # AI chat
│   └── SettingsView.swift    # Configuration
├── ViewModels/
│   ├── AppViewModel.swift    # Main state
│   ├── TerminalViewModel.swift
│   ├── ChatViewModel.swift
│   └── SettingsViewModel.swift
└── Assets.xcassets/          # Images, colors, etc.
```

## Next Steps

- Add SSH terminal mode (vs. local PTY)
- Export chat history
- Dark mode support (system-integrated)
- Keyboard shortcuts (iPad)
- Siri integration
- Push notifications for instance events

## License

MIT — see [LICENSE](../../LICENSE)

---

For server setup, see [CLAUDE.md](../../CLAUDE.md).
For Swift package docs, see [swift/TCKClient/README.md](../../swift/TCKClient/README.md).
