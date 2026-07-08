# TCKClient — Swift Package for T.C.K

A comprehensive Swift package providing type-safe access to the **T.C.K (TalentCloud Keyboard)** server APIs. Enables iOS, macOS, and other Apple platform apps to interact with terminal sessions, AI services, and GitHub integration.

## Features

- ✅ **HTTP API Client** — all REST endpoints with type-safe Codable models
- ✅ **WebSocket Management** — persistent connections with automatic reconnection for terminal I/O and events
- ✅ **Terminal Service** — high-level PTY abstraction for shell interaction
- ✅ **Settings Manager** — secure Keychain storage for API keys, UserDefaults for preferences
- ✅ **Full T.C.K Coverage** — instances, AI generation, GitHub integration, desktop APIs
- ✅ **Async/Await** — modern Swift concurrency throughout
- ✅ **Error Handling** — descriptive `TCKClientError` enum

## Requirements

- **Swift** 5.9+
- **iOS** 15+ / **macOS** 12+
- **T.C.K Server** running on the network (default: `http://127.0.0.1:3000`)

## Installation

### Swift Package Manager

Add to your `Package.swift`:

```swift
.package(url: "https://github.com/yourusername/tck-swift.git", branch: "main")
```

Or in Xcode: **File → Add Packages** → paste the repo URL.

## Quick Start

### 1. Initialize the Client

```swift
import TCKClient

let client = TCKClient(serverURL: URL(string: "http://127.0.0.1:3000")!)
```

### 2. Check Server Health

```swift
do {
    try await client.checkServerHealth()
    print("✓ Server is online")
} catch {
    print("✗ Server error: \(error)")
}
```

### 3. List Terminal Instances

```swift
let instances = try await client.getInstances()
for instance in instances {
    print("Instance: \(instance.name) (\(instance.state))")
}
```

### 4. Connect to a Terminal

```swift
let terminal = client.terminalManager.terminal(for: instanceId)

// Observe output
terminal.onOutput = { text in
    print("Terminal: \(text)")
}

// Observe errors
terminal.onError = { error in
    print("Error: \(error)")
}

// Connect
terminal.connect()

// Send input
try await terminal.sendInput("ls -la\n")

// Resize terminal
try await terminal.resize(rows: 24, cols: 80)
```

### 5. Generate AI Text

```swift
let response = try await client.generateAIText(
    prompt: "Explain Swift concurrency",
    system: "You are a helpful assistant"
)
print("AI: \(response ?? "No response")")
```

### 6. Fetch AI Models

```swift
let models = try await client.getAIModels()
for model in models {
    print("\(model.provider) / \(model.name)")
}
```

### 7. Store API Keys Securely

```swift
let settings = SettingsManager.shared

// Store API key
try settings.setAPIKey("sk-1234567890", for: "openai")

// Retrieve API key
let apiKey = try settings.getAPIKey(for: "openai")
```

## Module Breakdown

### `APIClient`
Low-level HTTP wrapper for all REST endpoints:
- Instance management: `listInstances()`, `createInstance()`, `getInstance()`, `updateInstance()`, `deleteInstance()`
- Terminal I/O: `sendTerminalInput()`, `resizeTerminal()`
- AI generation: `generateText()`, `fetchAIModels()`, `fetchAIProviders()`, `checkAIHealth()`
- GitHub: `getGitHubStatus()`, `listGitHubRepos()`, `getGitHubRepoContents()`

### `WebSocketManager`
Persistent WebSocket connection with auto-reconnection:
- States: `.disconnected`, `.connecting`, `.connected`, `.reconnecting`, `.failed`
- Events: `.connected`, `.disconnected`, `.message(String)`, `.error(Error)`
- Auto-reconnect with exponential backoff (max 5 attempts)
- Send string, data, or Codable objects

### `TerminalService`
High-level terminal abstraction:
- `connect()` — open PTY WebSocket
- `disconnect()` — close PTY
- `sendInput(String)` — send shell command
- `resize(rows:cols:)` — terminal window size
- Callbacks: `onOutput`, `onError`, `onStateChange`

### `TerminalManager`
Manages multiple concurrent terminal sessions:
- `terminal(for:)` — get or create service
- `connect/disconnect` — manage lifecycle
- `disconnectAll()` — cleanup

### `Models.swift`
Codable types matching T.C.K backend:
- **Instance management** — `Instance`, `InstanceState`, `CreateInstanceRequest`, `UpdateInstanceRequest`
- **AI** — `GenerateRequest`, `GenerateResponse`, `CatalogModel`, `Agent`, `TokenUsage`
- **Chat** — `ChatMessage`, `ChatRole`
- **Settings** — `AppSettings`, `ProviderConfig`
- **GitHub** — `GitHubStatus`, `RepoEntry`, `RepoFile`
- **WebSocket** — `EventMessage`, `TerminalInput`, `TerminalOutput`

### `SettingsManager`
Secure settings management with Keychain integration:
- `loadSettings()` / `saveSettings()` — app-wide settings (UserDefaults)
- `setAPIKey() / getAPIKey() / deleteAPIKey()` — secure Keychain storage
- `serverURL`, `selectedInstanceId`, `terminalFontSize`, `chatPanelVisible` — UI state

### `TCKClient`
Facade coordinating all services:
- `apiClient`, `terminalManager`, `settings` — access sub-services
- Convenience methods: `getInstances()`, `createInstance()`, `generateAIText()`, `getAIModels()`, etc.

## Architecture

```
TCKClient (facade)
├─ APIClient (REST requests)
├─ TerminalManager
│  └─ TerminalService (manages WebSocketManager per instance)
│     └─ WebSocketManager (persistent WebSocket)
└─ SettingsManager (Keychain + UserDefaults)
```

## Error Handling

```swift
do {
    try await client.generateAIText(prompt: "Hello")
} catch let error as TCKClientError {
    switch error {
    case .networkError(let err):
        print("Network: \(err)")
    case .serverError(let code, let msg):
        print("Server \(code): \(msg)")
    case .decodingError(let err):
        print("Decode: \(err)")
    default:
        print("Error: \(error.localizedDescription)")
    }
} catch {
    print("Unknown: \(error)")
}
```

## Testing

Run unit tests:

```bash
swift test
```

Or in Xcode: **Product → Test** (⌘U).

Test files cover:
- HTTP client mock/real requests
- WebSocket lifecycle
- Model encoding/decoding
- Settings persistence

## Usage Examples

### Example: Chat UI (SwiftUI)

```swift
@Observable
class ChatViewModel {
    let client: TCKClient
    var messages: [ChatMessage] = []
    var loading = false
    
    init(client: TCKClient) {
        self.client = client
    }
    
    func sendMessage(_ text: String) async {
        loading = true
        messages.append(ChatMessage(role: .user, text: text))
        
        do {
            if let response = try await client.generateAIText(prompt: text) {
                messages.append(ChatMessage(role: .assistant, text: response))
            }
        } catch {
            messages.append(ChatMessage(role: .error, text: "Error: \(error.localizedDescription)"))
        }
        
        loading = false
    }
}
```

### Example: Terminal View (SwiftUI)

```swift
@Observable
class TerminalViewModel {
    let terminal: TerminalService
    var output: String = ""
    var isConnected = false
    
    init(terminal: TerminalService) {
        self.terminal = terminal
        
        terminal.onOutput = { [weak self] text in
            self?.output.append(text)
        }
        
        terminal.onStateChange = { [weak self] state in
            self?.isConnected = (state == .connected)
        }
    }
    
    func connect() {
        terminal.connect()
    }
    
    func sendCommand(_ cmd: String) async throws {
        try await terminal.sendInput(cmd + "\n")
    }
}
```

## Troubleshooting

### "Cannot reach tck-server"
- Verify server is running: `cargo run -p tck-server`
- Check `serverURL` — defaults to `http://127.0.0.1:3000`
- On LAN: use server IP and ensure firewall allows port 3000

### "WebSocket disconnected"
- Network issue — check internet connection
- Auto-reconnect should activate (up to 5 attempts)
- Check server logs for errors

### "Keychain error"
- Simulators share Keychain — test on device if issues persist
- Ensure app has Keychain entitlements in **Build Settings**

## Contributing

Bug reports and PRs welcome! Please:
1. Test with a running T.C.K server
2. Add unit tests for new features
3. Follow Swift naming conventions (camelCase methods, PascalCase types)

## License

MIT — see [LICENSE](LICENSE) for details

---

**Next:** Build iOS and macOS apps using this library. See [ios/README.md](../ios/README.md) and [macos/README.md](../macos/README.md).
