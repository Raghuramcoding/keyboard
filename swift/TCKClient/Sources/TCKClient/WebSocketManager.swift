import Foundation

/// WebSocket connection manager with auto-reconnection
public class WebSocketManager: NSObject, URLSessionWebSocketDelegate {
    public enum ConnectionState {
        case disconnected
        case connecting
        case connected
        case reconnecting
        case failed(Error)
    }
    
    public enum WebSocketEvent {
        case connected
        case disconnected
        case message(String)
        case error(Error)
    }
    
    private var webSocket: URLSessionWebSocket?
    private let url: URL
    private let queue: DispatchQueue
    private var receiveTask: Task<Void, Never>?
    private var reconnectTask: Task<Void, Never>?
    
    public var state: ConnectionState = .disconnected {
        didSet {
            Task { @MainActor in
                self.onStateChange?(state)
            }
        }
    }
    
    public var onEvent: ((WebSocketEvent) -> Void)?
    public var onStateChange: ((ConnectionState) -> Void)?
    
    private var maxReconnectAttempts = 5
    private var reconnectAttempts = 0
    private var reconnectDelay: TimeInterval = 1.0
    private let maxReconnectDelay: TimeInterval = 32.0
    
    public init(url: URL) {
        self.url = url
        self.queue = DispatchQueue(label: "com.tck.websocket.\(UUID().uuidString)")
        super.init()
    }
    
    /// Connect to the WebSocket
    public func connect() {
        queue.async {
            guard self.webSocket == nil else { return }
            
            self.state = .connecting
            
            let config = URLSessionConfiguration.default
            config.waitsForConnectivity = true
            let session = URLSession(configuration: config, delegate: self, delegateQueue: nil)
            
            let webSocket = session.webSocketTask(with: self.url)
            self.webSocket = webSocket
            
            webSocket.resume()
            
            self.receiveMessages()
            self.reconnectAttempts = 0
            self.reconnectDelay = 1.0
        }
    }
    
    /// Disconnect from the WebSocket
    public func disconnect() {
        queue.async {
            self.reconnectTask?.cancel()
            self.receiveTask?.cancel()
            self.webSocket?.cancel(with: .goingAway, reason: nil)
            self.webSocket = nil
            self.state = .disconnected
        }
    }
    
    /// Send a string message
    public func send(_ message: String) async throws {
        try await send(.string(message))
    }
    
    /// Send a text message
    public func send(text: String) async throws {
        try await send(.string(text))
    }
    
    /// Send data
    public func send(data: Data) async throws {
        try await send(.data(data))
    }
    
    /// Send JSON-encoded object
    public func send<T: Encodable>(_ object: T) async throws {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        let data = try encoder.encode(object)
        try await send(.data(data))
    }
    
    // MARK: - Private Methods
    
    private func send(_ message: URLSessionWebSocket.Message) async throws {
        guard let webSocket = webSocket else {
            throw TCKClientError.unexpectedResponse
        }
        
        return try await withCheckedThrowingContinuation { continuation in
            webSocket.send(message) { error in
                if let error = error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            }
        }
    }
    
    private func receiveMessages() {
        receiveTask = Task {
            while !Task.isCancelled {
                guard let webSocket = self.webSocket else {
                    break
                }
                
                do {
                    let message = try await withCheckedThrowingContinuation { continuation in
                        webSocket.receive { result in
                            continuation.resume(with: result)
                        }
                    }
                    
                    self.handleMessage(message)
                } catch {
                    if !Task.isCancelled {
                        self.queue.async {
                            self.handleWebSocketError(error)
                        }
                    }
                    break
                }
            }
        }
    }
    
    private func handleMessage(_ message: URLSessionWebSocket.Message) {
        switch message {
        case .string(let text):
            onEvent?(.message(text))
        case .data(let data):
            if let text = String(data: data, encoding: .utf8) {
                onEvent?(.message(text))
            }
        @unknown default:
            break
        }
    }
    
    private func handleWebSocketError(_ error: Error) {
        onEvent?(.error(error))
        
        if shouldReconnect() {
            scheduleReconnect()
        } else {
            state = .failed(error)
            webSocket = nil
        }
    }
    
    private func shouldReconnect() -> Bool {
        reconnectAttempts < maxReconnectAttempts
    }
    
    private func scheduleReconnect() {
        state = .reconnecting
        reconnectAttempts += 1
        
        reconnectTask?.cancel()
        reconnectTask = Task {
            try? await Task.sleep(nanoseconds: UInt64(reconnectDelay * 1_000_000_000))
            
            if !Task.isCancelled {
                self.queue.async {
                    self.webSocket?.cancel(with: .goingAway, reason: nil)
                    self.webSocket = nil
                    self.connect()
                }
            }
            
            // Exponential backoff
            self.reconnectDelay = min(self.reconnectDelay * 2, self.maxReconnectDelay)
        }
    }
    
    // MARK: - URLSessionWebSocketDelegate
    
    public func urlSession(
        _ session: URLSession,
        webSocketTask: URLSessionWebSocket,
        didOpenWithProtocol protocol: String?
    ) {
        queue.async {
            self.state = .connected
            self.onEvent?(.connected)
        }
    }
    
    public func urlSession(
        _ session: URLSession,
        webSocketTask: URLSessionWebSocket,
        didCloseWith closeCode: URLSessionWebSocket.CloseCode,
        reason: Data?
    ) {
        queue.async {
            self.reconnectAttempts = 0
            self.reconnectDelay = 1.0
            
            if self.shouldReconnect() {
                self.scheduleReconnect()
            } else {
                self.state = .disconnected
                self.onEvent?(.disconnected)
                self.webSocket = nil
            }
        }
    }
}
