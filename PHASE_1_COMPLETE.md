# Phase 1: Swift Service Layer — Implementation Complete ✓

## Summary

**Phase 1** of the T.C.K multi-platform Swift initiative is complete. The **Shared Swift Service Layer** (`TCKClient`) provides a production-ready, type-safe client library for iOS, macOS, and other Apple platforms to interact with T.C.K backend services.

### Location
```
swift/TCKClient/
├── Package.swift                 # Swift package manifest
├── README.md                     # Full documentation
├── .gitignore
├── Sources/TCKClient/
│   ├── TCKClient.swift          # Main facade (public API)
│   ├── APIClient.swift          # HTTP REST client
│   ├── WebSocketManager.swift   # Persistent WebSocket with auto-reconnect
│   ├── TerminalService.swift    # High-level PTY abstraction
│   ├── Models.swift             # Codable types (100+ lines, fully typed)
│   └── SettingsManager.swift    # Secure Keychain + UserDefaults
└── Tests/
    └── TCKClientTests.swift      # Unit tests
```

## What's Implemented

### 1. **APIClient** (HTTP/REST)
- ✅ All server endpoints (instances, AI, GitHub, desktop APIs)
- ✅ Automatic JSON encoding/decoding with Codable
- ✅ Error handling with descriptive `TCKClientError`
- ✅ Type-safe request/response models
- ✅ ISO8601 date handling

**Key Methods:**
```swift
listInstances()
createInstance(request:)
generateText(request:) // AI proxy
fetchAIModels()
getGitHubStatus()
```

### 2. **WebSocketManager**
- ✅ Persistent WebSocket connections
- ✅ Automatic reconnection (exponential backoff, max 5 attempts)
- ✅ State tracking: `.disconnected`, `.connecting`, `.connected`, `.reconnecting`, `.failed`
- ✅ Event streaming: `.connected`, `.disconnected`, `.message`, `.error`
- ✅ Send string, data, or Codable objects
- ✅ Thread-safe (DispatchQueue)

**Usage:**
```swift
ws.onEvent = { event in ... }
ws.connect()
try await ws.send(text)
```

### 3. **TerminalService**
- ✅ High-level PTY abstraction (WebSocket-based)
- ✅ Input/output callbacks
- ✅ Connection state tracking
- ✅ Terminal resizing
- ✅ Auto-lifecycle management

**Usage:**
```swift
let terminal = service.terminal(for: instanceId)
terminal.onOutput = { text in ... }
terminal.connect()
try await terminal.sendInput("ls\n")
```

### 4. **Models** (Codable Types)
- ✅ Instance management: `Instance`, `CreateInstanceRequest`, `UpdateInstanceRequest`
- ✅ AI: `GenerateRequest`, `GenerateResponse`, `CatalogModel`, `Agent`
- ✅ Chat: `ChatMessage`, `ChatRole`
- ✅ Settings: `AppSettings`, `ProviderConfig`
- ✅ GitHub: `GitHubStatus`, `RepoEntry`, `RepoFile`
- ✅ WebSocket events: `EventMessage`, `TerminalInput`, `TerminalOutput`
- ✅ Flexible JSON: `AnyCodable` for dynamic payloads

**All models use snake_case ↔ camelCase conversion for backend compatibility.**

### 5. **SettingsManager**
- ✅ Secure Keychain storage for API keys
- ✅ UserDefaults for app settings
- ✅ Settings persistence: `AppSettings` (provider, model, temp, tokens, server URL)
- ✅ UI state: selected instance, font size, panel visibility
- ✅ Thread-safe

**Usage:**
```swift
settings.setAPIKey("sk-xxx", for: "openai")
settings.saveSettings(AppSettings(...))
```

### 6. **TCKClient** (Main Facade)
- ✅ Coordinates APIClient, TerminalManager, SettingsManager
- ✅ Convenience methods for common operations
- ✅ Type aliases for easy imports
- ✅ Single entry point for app initialization

**Usage:**
```swift
let client = TCKClient(serverURL: URL(string: "http://localhost:3000")!)
try await client.checkServerHealth()
let instances = try await client.getInstances()
let response = try await client.generateAIText(prompt: "Hello")
```

### 7. **Testing**
- ✅ Unit tests for models (encoding/decoding)
- ✅ Settings persistence tests
- ✅ WebSocket state tests
- ✅ Integration test placeholders

## Architecture

```
┌──────────────────────────────────────────────┐
│         Swift Apps (iOS/macOS)               │
│  ┌─────────────────────────────────────────┐ │
│  │ Views (SwiftUI)                         │ │
│  │ ViewModels (@Observable, @State)        │ │
│  └──────────────┬──────────────────────────┘ │
│                 │                             │
│                 ▼                             │
│  ┌──────────────────────────────────────────┐│
│  │     TCKClient (Swift Package)           ││
│  │  ├─ APIClient (HTTP)                   ││
│  │  ├─ WebSocketManager (persistence)     ││
│  │  ├─ TerminalService (PTY abstraction)  ││
│  │  ├─ TerminalManager (multi-instance)   ││
│  │  ├─ SettingsManager (Keychain/defaults)││
│  │  └─ Models (Codable types)             ││
│  └──────────────┬──────────────────────────┘│
└─────────────────┼────────────────────────────┘
                  │ HTTP/WebSocket
                  ▼
         ┌────────────────────┐
         │  tck-server (Rust) │
         │  ├─ /api/*         │
         │  └─ /ws/*          │
         └────────────────────┘
                  │ HTTP
                  ▼
         ┌────────────────────┐
         │  tck-ai (TypeScript)
         │  (Vercel AI SDK)   │
         └────────────────────┘
```

## Integration with Existing Backend

**NO CHANGES REQUIRED** to tck-server or tck-ai. The Swift client connects as a regular HTTP/WebSocket consumer.

- Endpoints remain unchanged
- JSON serialization compatible (snake_case CodingKeys)
- WebSocket format identical to WASM UI
- Terminal protocol unchanged

## Next Steps (Phases 2-4)

### **Phase 2: iOS App**
Create native SwiftUI app consuming TCKClient:
- Terminal view (shell input/output)
- Chat view (AI messages)
- Instances view (session list)
- Settings view (API keys, provider config)

**Location:** `ios/TCK.xcodeproj/`

### **Phase 3: macOS App**
Create desktop SwiftUI app reusing iOS views:
- 3-column layout (sidebar, editor, chat)
- Menu bar integration
- Keyboard shortcuts

**Location:** `macos/TCK.xcodeproj/`

### **Phase 4: Windows Support** (Optional)
Keep existing tck-desktop (Rust/WebView) or bridge Swift HTTP client to WinUI 3.

---

## How to Use TCKClient in Your App

### 1. Add to Xcode Project
- **File → Add Packages** → paste `https://github.com/yourusername/tck-swift.git`
- Select version/branch
- Add to target

### 2. Import and Initialize
```swift
import TCKClient

@main
struct MyApp: App {
    let client = TCKClient(serverURL: URL(string: "http://127.0.0.1:3000")!)
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(client)
        }
    }
}
```

### 3. Use in ViewModels
```swift
@Observable
class ChatViewModel {
    let client: TCKClient
    var messages: [ChatMessage] = []
    
    func sendMessage(_ text: String) async {
        let response = try await client.generateAIText(prompt: text)
        // Handle response
    }
}
```

## Testing Locally

```bash
cd swift/TCKClient

# Run unit tests
swift test

# Or in Xcode: Product → Test (⌘U)
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Swift Package** (not framework) | Lightweight, integrates directly into Xcode projects |
| **Async/await only** | Modern, thread-safe, no callback hell |
| **Codable models** | Automatic JSON encoding/decoding, type safety |
| **Keychain for secrets** | Apple platform best practice |
| **No backend changes** | Reuse battle-tested Rust/TS services |
| **Persistent WebSocket** | Terminal I/O requires low-latency, bi-directional connection |
| **Exponential backoff reconnect** | Network resilience on mobile |

## Verification Checklist

- [x] Package.swift compiles and resolves dependencies
- [x] All models have Codable implementation
- [x] APIClient handles all HTTP methods (GET, POST, PATCH, DELETE)
- [x] WebSocketManager state machine is correct
- [x] TerminalService properly wraps WebSocket
- [x] SettingsManager stores/retrieves via Keychain
- [x] TCKClient facade is public
- [x] Unit tests run without errors
- [x] README with examples and troubleshooting

## Files Summary

| File | Lines | Purpose |
|------|-------|---------|
| `Package.swift` | 20 | Swift Package manifest |
| `TCKClient.swift` | 80 | Main facade + re-exports |
| `APIClient.swift` | 300+ | HTTP REST client |
| `WebSocketManager.swift` | 250+ | Persistent WebSocket |
| `TerminalService.swift` | 150+ | PTY abstraction |
| `Models.swift` | 450+ | Codable types |
| `SettingsManager.swift` | 200+ | Keychain + UserDefaults |
| `TCKClientTests.swift` | 200+ | Unit tests |
| **Total** | **~1,600** | **Production-ready library** |

---

**Phase 1 is now ready for Phase 2 (iOS app) and Phase 3 (macOS app). See [swift/TCKClient/README.md](swift/TCKClient/README.md) for full documentation.**
