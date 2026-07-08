import Foundation

/// Service for managing terminal/PTY interactions
public class TerminalService {
    private let apiClient: APIClient
    private let wsManager: WebSocketManager
    private let instanceId: String
    
    public var onOutput: ((String) -> Void)?
    public var onError: ((Error) -> Void)?
    
    public init(apiClient: APIClient, instanceId: String) {
        self.apiClient = apiClient
        self.instanceId = instanceId
        
        let wsURL = apiClient.baseURL
            .appendingPathComponent("ws/pty/\(instanceId)")
            .scheme("ws")
        
        self.wsManager = WebSocketManager(url: wsURL)
        setupWebSocketHandlers()
    }
    
    // MARK: - Public API
    
    /// Connect to the terminal
    public func connect() {
        wsManager.connect()
    }
    
    /// Disconnect from the terminal
    public func disconnect() {
        wsManager.disconnect()
    }
    
    /// Send input to the terminal
    public func sendInput(_ text: String) async throws {
        try await wsManager.send(text)
    }
    
    /// Resize the terminal
    public func resize(rows: Int, cols: Int) async throws {
        try await apiClient.resizeTerminal(instanceId: instanceId, rows: rows, cols: cols)
    }
    
    /// Get current connection state
    public var connectionState: WebSocketManager.ConnectionState {
        wsManager.state
    }
    
    /// Observe connection state changes
    public var onStateChange: ((WebSocketManager.ConnectionState) -> Void)? {
        get { wsManager.onStateChange }
        set { wsManager.onStateChange = newValue }
    }
    
    // MARK: - Private Methods
    
    private func setupWebSocketHandlers() {
        wsManager.onEvent = { [weak self] event in
            switch event {
            case .message(let text):
                self?.onOutput?(text)
            case .error(let error):
                self?.onError?(error)
            case .connected:
                break
            case .disconnected:
                break
            }
        }
    }
}

/// Manages multiple terminal sessions
public class TerminalManager {
    private let apiClient: APIClient
    private var terminals: [String: TerminalService] = [:]
    private let queue = DispatchQueue(label: "com.tck.terminal-manager")
    
    public init(apiClient: APIClient) {
        self.apiClient = apiClient
    }
    
    /// Get or create a terminal service for an instance
    public func terminal(for instanceId: String) -> TerminalService {
        queue.sync {
            if let terminal = terminals[instanceId] {
                return terminal
            }
            
            let terminal = TerminalService(apiClient: apiClient, instanceId: instanceId)
            terminals[instanceId] = terminal
            return terminal
        }
    }
    
    /// Remove a terminal service
    public func removeTerminal(instanceId: String) {
        queue.sync {
            terminals.removeValue(forKey: instanceId)?.disconnect()
        }
    }
    
    /// Connect terminal
    public func connect(instanceId: String) {
        queue.async {
            self.terminals[instanceId]?.connect()
        }
    }
    
    /// Disconnect terminal
    public func disconnect(instanceId: String) {
        queue.async {
            self.terminals[instanceId]?.disconnect()
        }
    }
    
    /// Disconnect all terminals
    public func disconnectAll() {
        queue.sync {
            terminals.values.forEach { $0.disconnect() }
        }
    }
}

// MARK: - URL Extensions for scheme conversion

extension URL {
    /// Convert http to ws and https to wss
    func scheme(_ newScheme: String) -> URL {
        var components = URLComponents(url: self, resolvingAgainstBaseURL: false)
        
        // Map http/https to ws/wss
        if newScheme == "ws" {
            switch components?.scheme {
            case "https":
                components?.scheme = "wss"
            default:
                components?.scheme = "ws"
            }
        } else if newScheme == "wss" {
            components?.scheme = "wss"
        } else if newScheme == "http" {
            components?.scheme = "http"
        } else if newScheme == "https" {
            components?.scheme = "https"
        } else {
            components?.scheme = newScheme
        }
        
        return components?.url ?? self
    }
}
