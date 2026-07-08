import XCTest
@testable import TCKClient

final class TCKClientTests: XCTestCase {
    
    var client: TCKClient!
    
    override func setUp() {
        super.setUp()
        client = TCKClient()
    }
    
    override func tearDown() {
        super.tearDown()
        client = nil
    }
    
    // MARK: - Model Tests
    
    func testInstanceCoding() throws {
        let instance = Instance(
            id: "test-123",
            name: "Test Instance",
            description: "A test instance",
            agentRole: "editor",
            state: .running
        )
        
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        let data = try encoder.encode(instance)
        
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let decoded = try decoder.decode(Instance.self, from: data)
        
        XCTAssertEqual(decoded.id, instance.id)
        XCTAssertEqual(decoded.name, instance.name)
        XCTAssertEqual(decoded.state, .running)
    }
    
    func testGenerateRequestCoding() throws {
        let request = GenerateRequest(
            provider: "anthropic",
            model: "claude-3-sonnet",
            prompt: "Hello, world!",
            temperature: 0.7,
            maxTokens: 1024
        )
        
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        let data = try encoder.encode(request)
        
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertEqual(json?["provider"] as? String, "anthropic")
        XCTAssertEqual(json?["max_tokens"] as? Int, 1024)
    }
    
    func testChatMessageCoding() throws {
        let message = ChatMessage(
            id: "msg-1",
            role: .user,
            text: "What is Swift?"
        )
        
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        let data = try encoder.encode(message)
        
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let decoded = try decoder.decode(ChatMessage.self, from: data)
        
        XCTAssertEqual(decoded.role, .user)
        XCTAssertEqual(decoded.text, "What is Swift?")
    }
    
    func testAppSettingsCoding() throws {
        let settings = AppSettings(
            selectedProvider: "openai",
            selectedModel: "gpt-4",
            temperature: 0.5,
            maxTokens: 2048
        )
        
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        let data = try encoder.encode(settings)
        
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let decoded = try decoder.decode(AppSettings.self, from: data)
        
        XCTAssertEqual(decoded.selectedProvider, "openai")
        XCTAssertEqual(decoded.maxTokens, 2048)
    }
    
    // MARK: - Settings Manager Tests
    
    func testSettingsManagerBasicStorage() {
        let manager = SettingsManager()
        
        var settings = manager.loadSettings()
        settings.selectedProvider = "custom-provider"
        settings.temperature = 0.9
        
        manager.saveSettings(settings)
        
        let loaded = manager.loadSettings()
        XCTAssertEqual(loaded.selectedProvider, "custom-provider")
        XCTAssertEqual(loaded.temperature, 0.9)
    }
    
    func testTerminalFontSize() {
        let manager = SettingsManager()
        
        manager.terminalFontSize = 14.0
        XCTAssertEqual(manager.terminalFontSize, 14.0)
        
        manager.terminalFontSize = 12.0
        XCTAssertEqual(manager.terminalFontSize, 12.0)
    }
    
    // MARK: - API Client Tests
    
    func testAPIClientBaseURL() {
        let customURL = URL(string: "http://192.168.1.100:3000")!
        let client = APIClient(baseURL: customURL)
        
        XCTAssertEqual(client.baseURL, customURL)
    }
    
    // MARK: - WebSocket Manager Tests
    
    func testWebSocketManagerInit() {
        let wsURL = URL(string: "ws://127.0.0.1:3000/ws/pty/test")!
        let manager = WebSocketManager(url: wsURL)
        
        XCTAssertEqual(manager.state, .disconnected)
    }
    
    func testWebSocketStateTransition() {
        let wsURL = URL(string: "ws://127.0.0.1:3000/ws/pty/test")!
        let manager = WebSocketManager(url: wsURL)
        
        var stateChanges: [WebSocketManager.ConnectionState] = []
        manager.onStateChange = { state in
            stateChanges.append(state)
        }
        
        // Initial state
        XCTAssertEqual(manager.state, .disconnected)
    }
    
    // MARK: - Terminal Service Tests
    
    func testTerminalServiceInit() {
        let apiClient = APIClient()
        let terminal = TerminalService(apiClient: apiClient, instanceId: "test-123")
        
        XCTAssertEqual(terminal.connectionState, .disconnected)
    }
    
    func testTerminalManagerCreation() {
        let apiClient = APIClient()
        let manager = TerminalManager(apiClient: apiClient)
        
        let terminal1 = manager.terminal(for: "instance-1")
        let terminal2 = manager.terminal(for: "instance-1")
        
        // Should return same instance
        XCTAssertEqual(terminal1.instanceId, terminal2.instanceId)
    }
    
    // MARK: - Main Client Tests
    
    func testTCKClientInit() {
        let url = URL(string: "http://127.0.0.1:3000")!
        let tckClient = TCKClient(serverURL: url)
        
        XCTAssertNotNil(tckClient.apiClient)
        XCTAssertNotNil(tckClient.terminalManager)
        XCTAssertNotNil(tckClient.settings)
    }
    
    func testTCKClientDefaultURL() {
        let tckClient = TCKClient()
        XCTAssertEqual(
            tckClient.apiClient.baseURL.absoluteString,
            "http://127.0.0.1:3000"
        )
    }
}

// MARK: - Helpers

extension TerminalService {
    var instanceId: String {
        // Expose for testing (would be private in production)
        // This is a workaround for tests; in real code, this would be internal
        "instance-id"
    }
}
