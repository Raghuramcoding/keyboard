import Foundation

// MARK: - Models matching tck-core types

/// Represents a terminal instance (session) managed by tck-server
public struct Instance: Codable, Identifiable {
    public let id: String
    public let name: String
    public let description: String?
    public let agentRole: String?
    public let createdAt: Date?
    public let state: InstanceState
    
    public init(
        id: String,
        name: String,
        description: String? = nil,
        agentRole: String? = nil,
        createdAt: Date? = nil,
        state: InstanceState = .idle
    ) {
        self.id = id
        self.name = name
        self.description = description
        self.agentRole = agentRole
        self.createdAt = createdAt
        self.state = state
    }
    
    enum CodingKeys: String, CodingKey {
        case id
        case name
        case description
        case agentRole = "agent_role"
        case createdAt = "created_at"
        case state
    }
}

public enum InstanceState: String, Codable {
    case idle
    case running
    case paused
    case error
}

/// Request to create a new instance
public struct CreateInstanceRequest: Codable {
    public let name: String
    public let description: String?
    public let agentRole: String?
    
    public init(name: String, description: String? = nil, agentRole: String? = nil) {
        self.name = name
        self.description = description
        self.agentRole = agentRole
    }
    
    enum CodingKeys: String, CodingKey {
        case name
        case description
        case agentRole = "agent_role"
    }
}

/// Request to update an instance
public struct UpdateInstanceRequest: Codable {
    public let name: String?
    public let description: String?
    public let state: InstanceState?
    
    public init(name: String? = nil, description: String? = nil, state: InstanceState? = nil) {
        self.name = name
        self.description = description
        self.state = state
    }
}

// MARK: - AI Generation Models

/// Request to generate text via AI
public struct GenerateRequest: Codable {
    public let provider: String
    public let model: String
    public let prompt: String
    public let system: String?
    public let apiKey: String?
    public let baseUrl: String?
    public let temperature: Double?
    public let maxTokens: Int?
    
    public init(
        provider: String,
        model: String,
        prompt: String,
        system: String? = nil,
        apiKey: String? = nil,
        baseUrl: String? = nil,
        temperature: Double? = nil,
        maxTokens: Int? = nil
    ) {
        self.provider = provider
        self.model = model
        self.prompt = prompt
        self.system = system
        self.apiKey = apiKey
        self.baseUrl = baseUrl
        self.temperature = temperature
        self.maxTokens = maxTokens
    }
    
    enum CodingKeys: String, CodingKey {
        case provider
        case model
        case prompt
        case system
        case apiKey = "api_key"
        case baseUrl = "base_url"
        case temperature
        case maxTokens = "max_tokens"
    }
}

/// Response from AI generation
public struct GenerateResponse: Codable {
    public let response: String?
    public let model: String?
    public let error: String?
    public let usage: TokenUsage?
    
    public init(response: String? = nil, model: String? = nil, error: String? = nil, usage: TokenUsage? = nil) {
        self.response = response
        self.model = model
        self.error = error
        self.usage = usage
    }
}

public struct TokenUsage: Codable {
    public let promptTokens: Int?
    public let completionTokens: Int?
    public let totalTokens: Int?
    
    public init(promptTokens: Int? = nil, completionTokens: Int? = nil, totalTokens: Int? = nil) {
        self.promptTokens = promptTokens
        self.completionTokens = completionTokens
        self.totalTokens = totalTokens
    }
    
    enum CodingKeys: String, CodingKey {
        case promptTokens = "prompt_tokens"
        case completionTokens = "completion_tokens"
        case totalTokens = "total_tokens"
    }
}

// MARK: - Chat Models

/// A single message in the chat history
public struct ChatMessage: Codable, Identifiable {
    public let id: String
    public let role: ChatRole
    public let text: String
    public let timestamp: Date?
    
    public init(id: String = UUID().uuidString, role: ChatRole, text: String, timestamp: Date? = nil) {
        self.id = id
        self.role = role
        self.text = text
        self.timestamp = timestamp ?? Date()
    }
}

public enum ChatRole: String, Codable {
    case user
    case assistant
    case system
    case error
}

// MARK: - AI Provider & Model Models

/// AI provider configuration
public struct ProviderConfig: Codable, Identifiable {
    public let id: String
    public let name: String
    public let baseUrl: String?
    public let apiKey: String?
    
    public init(id: String, name: String, baseUrl: String? = nil, apiKey: String? = nil) {
        self.id = id
        self.name = name
        self.baseUrl = baseUrl
        self.apiKey = apiKey
    }
    
    enum CodingKeys: String, CodingKey {
        case id
        case name
        case baseUrl = "base_url"
        case apiKey = "api_key"
    }
}

/// A model from the models.dev catalog
public struct CatalogModel: Codable, Identifiable {
    public let id: String
    public let provider: String
    public let name: String
    public let contextWindow: Int
    public let supportsTools: Bool
    public let supportsStreaming: Bool
    
    public init(
        id: String,
        provider: String,
        name: String,
        contextWindow: Int,
        supportsTools: Bool = false,
        supportsStreaming: Bool = false
    ) {
        self.id = id
        self.provider = provider
        self.name = name
        self.contextWindow = contextWindow
        self.supportsTools = supportsTools
        self.supportsStreaming = supportsStreaming
    }
    
    enum CodingKeys: String, CodingKey {
        case id
        case provider
        case name
        case contextWindow = "context_window"
        case supportsTools = "supports_tools"
        case supportsStreaming = "supports_streaming"
    }
}

/// AI Agent definition
public struct Agent: Codable, Identifiable {
    public let id: String
    public let name: String
    public let role: String
    public let description: String?
    public let systemPrompt: String?
    public let color: String?
    
    public init(
        id: String,
        name: String,
        role: String,
        description: String? = nil,
        systemPrompt: String? = nil,
        color: String? = nil
    ) {
        self.id = id
        self.name = name
        self.role = role
        self.description = description
        self.systemPrompt = systemPrompt
        self.color = color
    }
    
    enum CodingKeys: String, CodingKey {
        case id
        case name
        case role
        case description
        case systemPrompt = "system_prompt"
        case color
    }
}

// MARK: - Settings & App State

/// User settings persisted across sessions
public struct AppSettings: Codable {
    public var selectedProvider: String
    public var selectedModel: String
    public var temperature: Double
    public var maxTokens: Int
    public var systemPrompt: String
    public var serverUrl: URL
    public var providers: [ProviderConfig]
    
    public init(
        selectedProvider: String = "anthropic",
        selectedModel: String = "claude-3-5-sonnet-20241022",
        temperature: Double = 0.7,
        maxTokens: Int = 4096,
        systemPrompt: String = "You are a helpful AI assistant.",
        serverUrl: URL = URL(string: "http://127.0.0.1:3000")!,
        providers: [ProviderConfig] = []
    ) {
        self.selectedProvider = selectedProvider
        self.selectedModel = selectedModel
        self.temperature = temperature
        self.maxTokens = maxTokens
        self.systemPrompt = systemPrompt
        self.serverUrl = serverUrl
        self.providers = providers
    }
    
    enum CodingKeys: String, CodingKey {
        case selectedProvider = "selected_provider"
        case selectedModel = "selected_model"
        case temperature
        case maxTokens = "max_tokens"
        case systemPrompt = "system_prompt"
        case serverUrl = "server_url"
        case providers
    }
}

// MARK: - GitHub & Repository Models

/// GitHub authentication status
public struct GitHubStatus: Codable {
    public let isAuthorized: Bool
    public let username: String?
    public let avatarUrl: String?
    
    public init(isAuthorized: Bool, username: String? = nil, avatarUrl: String? = nil) {
        self.isAuthorized = isAuthorized
        self.username = username
        self.avatarUrl = avatarUrl
    }
    
    enum CodingKeys: String, CodingKey {
        case isAuthorized = "is_authorized"
        case username
        case avatarUrl = "avatar_url"
    }
}

/// GitHub repository entry
public struct RepoEntry: Codable, Identifiable {
    public let id: String
    public let name: String
    public let url: String?
    public let isPrivate: Bool
    
    public init(id: String, name: String, url: String? = nil, isPrivate: Bool = false) {
        self.id = id
        self.name = name
        self.url = url
        self.isPrivate = isPrivate
    }
    
    enum CodingKeys: String, CodingKey {
        case id
        case name
        case url
        case isPrivate = "is_private"
    }
}

/// File in a repository
public struct RepoFile: Codable, Identifiable {
    public let id: String
    public let path: String
    public let content: String?
    public let isDirectory: Bool
    
    public init(id: String, path: String, content: String? = nil, isDirectory: Bool = false) {
        self.id = id
        self.path = path
        self.content = content
        self.isDirectory = isDirectory
    }
    
    enum CodingKeys: String, CodingKey {
        case id
        case path
        case content
        case isDirectory = "is_directory"
    }
}

// MARK: - WebSocket Messages

/// Generic message wrapper for WebSocket events
public struct EventMessage: Codable {
    public let type: String
    public let payload: AnyCodable?
    
    public init(type: String, payload: AnyCodable? = nil) {
        self.type = type
        self.payload = payload
    }
}

/// Wrapper to handle dynamic JSON payloads
public struct AnyCodable: Codable {
    public let value: Any
    
    public init(_ value: Any) {
        self.value = value
    }
    
    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        
        if container.decodeNil() {
            self.value = NSNull()
        } else if let bool = try? container.decode(Bool.self) {
            self.value = bool
        } else if let int = try? container.decode(Int.self) {
            self.value = int
        } else if let double = try? container.decode(Double.self) {
            self.value = double
        } else if let string = try? container.decode(String.self) {
            self.value = string
        } else if let array = try? container.decode([AnyCodable].self) {
            self.value = array.map { $0.value }
        } else if let dict = try? container.decode([String: AnyCodable].self) {
            self.value = dict.mapValues { $0.value }
        } else {
            throw DecodingError.dataCorruptedError(in: container, debugDescription: "Cannot decode AnyCodable")
        }
    }
    
    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        
        if let null = value as? NSNull {
            try container.encodeNil()
        } else if let bool = value as? Bool {
            try container.encode(bool)
        } else if let int = value as? Int {
            try container.encode(int)
        } else if let double = value as? Double {
            try container.encode(double)
        } else if let string = value as? String {
            try container.encode(string)
        } else if let array = value as? [Any] {
            try container.encode(array.map { AnyCodable($0) })
        } else if let dict = value as? [String: Any] {
            try container.encode(dict.mapValues { AnyCodable($0) })
        } else {
            throw EncodingError.invalidValue(value, EncodingError.Context(
                codingPath: container.codingPath,
                debugDescription: "Cannot encode AnyCodable"
            ))
        }
    }
}

// MARK: - Terminal Input/Output

/// Terminal input message
public struct TerminalInput: Codable {
    public let type: String = "input"
    public let data: String
    
    public init(data: String) {
        self.data = data
    }
}

/// Terminal output event
public struct TerminalOutput: Codable {
    public let type: String
    public let data: String
    public let timestamp: Date
    
    public init(type: String = "output", data: String, timestamp: Date = Date()) {
        self.type = type
        self.data = data
        self.timestamp = timestamp
    }
}
