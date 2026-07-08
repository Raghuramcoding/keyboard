import Foundation

/// Error types for TCK API client
public enum TCKClientError: LocalizedError {
    case invalidURL
    case networkError(Error)
    case serverError(statusCode: Int, message: String)
    case decodingError(Error)
    case encodingError(Error)
    case unexpectedResponse
    case healthCheckFailed
    
    public var errorDescription: String? {
        switch self {
        case .invalidURL:
            return "Invalid URL"
        case .networkError(let error):
            return "Network error: \(error.localizedDescription)"
        case .serverError(let code, let msg):
            return "Server error (\(code)): \(msg)"
        case .decodingError(let error):
            return "Failed to decode response: \(error.localizedDescription)"
        case .encodingError(let error):
            return "Failed to encode request: \(error.localizedDescription)"
        case .unexpectedResponse:
            return "Unexpected response from server"
        case .healthCheckFailed:
            return "Server health check failed"
        }
    }
}

/// HTTP client for T.C.K server
public class APIClient {
    private let session: URLSession
    private let baseURL: URL
    private let decoder: JSONDecoder
    private let encoder: JSONEncoder
    
    public init(baseURL: URL = URL(string: "http://127.0.0.1:3000")!, session: URLSession = .shared) {
        self.baseURL = baseURL
        self.session = session
        
        self.decoder = JSONDecoder()
        self.decoder.dateDecodingStrategy = .iso8601
        
        self.encoder = JSONEncoder()
        self.encoder.dateEncodingStrategy = .iso8601
    }
    
    /// Check server health
    public func healthCheck() async throws {
        let url = baseURL.appendingPathComponent("api/health")
        let (data, response) = try await session.data(from: url)
        
        guard let httpResponse = response as? HTTPURLResponse else {
            throw TCKClientError.unexpectedResponse
        }
        
        guard (200...299).contains(httpResponse.statusCode) else {
            let errorMsg = String(data: data, encoding: .utf8) ?? "Unknown error"
            throw TCKClientError.serverError(statusCode: httpResponse.statusCode, message: errorMsg)
        }
    }
    
    // MARK: - Instance Management
    
    /// Fetch all instances
    public func listInstances() async throws -> [Instance] {
        let url = baseURL.appendingPathComponent("api/instances")
        return try await get(from: url)
    }
    
    /// Create a new instance
    public func createInstance(request: CreateInstanceRequest) async throws -> Instance {
        let url = baseURL.appendingPathComponent("api/instances")
        return try await post(to: url, body: request)
    }
    
    /// Get a specific instance
    public func getInstance(id: String) async throws -> Instance {
        let url = baseURL.appendingPathComponent("api/instances/\(id)")
        return try await get(from: url)
    }
    
    /// Update an instance
    public func updateInstance(id: String, request: UpdateInstanceRequest) async throws -> Instance {
        let url = baseURL.appendingPathComponent("api/instances/\(id)")
        return try await patch(to: url, body: request)
    }
    
    /// Delete an instance
    public func deleteInstance(id: String) async throws {
        let url = baseURL.appendingPathComponent("api/instances/\(id)")
        try await delete(from: url)
    }
    
    /// Send input to instance terminal
    public func sendTerminalInput(instanceId: String, input: String) async throws {
        let url = baseURL.appendingPathComponent("api/instances/\(instanceId)/stdin")
        let data = TerminalInput(data: input)
        try await post(to: url, body: data)
    }
    
    /// Resize instance terminal
    public func resizeTerminal(instanceId: String, rows: Int, cols: Int) async throws {
        let url = baseURL.appendingPathComponent("api/instances/\(instanceId)/resize")
        let payload = ["rows": rows, "cols": cols] as [String: Any]
        try await post(to: url, body: payload)
    }
    
    // MARK: - AI Generation
    
    /// Generate text using AI (proxied to tck-ai)
    public func generateText(request: GenerateRequest) async throws -> GenerateResponse {
        let url = baseURL.appendingPathComponent("api/ai/generate")
        return try await post(to: url, body: request)
    }
    
    /// Fetch available AI models
    public func fetchAIModels() async throws -> [CatalogModel] {
        let url = baseURL.appendingPathComponent("api/ai/models")
        return try await get(from: url)
    }
    
    /// Fetch available AI providers
    public func fetchAIProviders() async throws -> [ProviderConfig] {
        let url = baseURL.appendingPathComponent("api/ai/providers")
        return try await get(from: url)
    }
    
    /// Check AI service health
    public func checkAIHealth() async throws {
        let url = baseURL.appendingPathComponent("api/ai/health")
        let (_, response) = try await session.data(from: url)
        
        guard let httpResponse = response as? HTTPURLResponse else {
            throw TCKClientError.unexpectedResponse
        }
        
        guard (200...299).contains(httpResponse.statusCode) else {
            throw TCKClientError.healthCheckFailed
        }
    }
    
    // MARK: - GitHub Integration
    
    /// Fetch GitHub authentication status
    public func getGitHubStatus() async throws -> GitHubStatus {
        let url = baseURL.appendingPathComponent("api/github/status")
        return try await get(from: url)
    }
    
    /// List GitHub repositories
    public func listGitHubRepos() async throws -> [RepoEntry] {
        let url = baseURL.appendingPathComponent("api/github/repos")
        return try await get(from: url)
    }
    
    /// Get GitHub repository contents
    public func getGitHubRepoContents(repo: String, path: String) async throws -> [RepoFile] {
        let url = baseURL.appendingPathComponent("api/github/repos/\(repo)/contents")
            .appendingQueryItem("path", value: path)
        return try await get(from: url)
    }
    
    // MARK: - Desktop-specific APIs
    
    /// Get desktop window state
    public func getDesktopWindowState() async throws -> [String: Any] {
        let url = baseURL.appendingPathComponent("api/desktop/window-state")
        let (data, response) = try await session.data(from: url)
        
        guard let httpResponse = response as? HTTPURLResponse else {
            throw TCKClientError.unexpectedResponse
        }
        
        guard (200...299).contains(httpResponse.statusCode) else {
            let errorMsg = String(data: data, encoding: .utf8) ?? "Unknown error"
            throw TCKClientError.serverError(statusCode: httpResponse.statusCode, message: errorMsg)
        }
        
        guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw TCKClientError.unexpectedResponse
        }
        
        return json
    }
    
    // MARK: - Helper methods
    
    private func get<T: Decodable>(from url: URL) async throws -> T {
        do {
            let (data, response) = try await session.data(from: url)
            return try handleResponse(data, response: response)
        } catch is TCKClientError {
            throw error
        } catch {
            throw TCKClientError.networkError(error)
        }
    }
    
    private func post<T: Encodable, U: Decodable>(to url: URL, body: T) async throws -> U {
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        do {
            request.httpBody = try encoder.encode(body)
            let (data, response) = try await session.data(for: request)
            return try handleResponse(data, response: response)
        } catch is TCKClientError {
            throw error
        } catch let error as EncodingError {
            throw TCKClientError.encodingError(error)
        } catch {
            throw TCKClientError.networkError(error)
        }
    }
    
    private func post(to url: URL, body: [String: Any]) async throws {
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        do {
            request.httpBody = try JSONSerialization.data(withJSONObject: body)
            let (data, response) = try await session.data(for: request)
            let _: Void = try handleResponse(data, response: response)
        } catch is TCKClientError {
            throw error
        } catch {
            throw TCKClientError.networkError(error)
        }
    }
    
    private func patch<T: Encodable, U: Decodable>(to url: URL, body: T) async throws -> U {
        var request = URLRequest(url: url)
        request.httpMethod = "PATCH"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        do {
            request.httpBody = try encoder.encode(body)
            let (data, response) = try await session.data(for: request)
            return try handleResponse(data, response: response)
        } catch is TCKClientError {
            throw error
        } catch let error as EncodingError {
            throw TCKClientError.encodingError(error)
        } catch {
            throw TCKClientError.networkError(error)
        }
    }
    
    private func delete(from url: URL) async throws {
        var request = URLRequest(url: url)
        request.httpMethod = "DELETE"
        
        do {
            let (data, response) = try await session.data(for: request)
            let _: Void = try handleResponse(data, response: response)
        } catch is TCKClientError {
            throw error
        } catch {
            throw TCKClientError.networkError(error)
        }
    }
    
    private func handleResponse<T: Decodable>(_ data: Data, response: URLResponse) throws -> T {
        guard let httpResponse = response as? HTTPURLResponse else {
            throw TCKClientError.unexpectedResponse
        }
        
        guard (200...299).contains(httpResponse.statusCode) else {
            let errorMsg = String(data: data, encoding: .utf8) ?? "Unknown error"
            throw TCKClientError.serverError(statusCode: httpResponse.statusCode, message: errorMsg)
        }
        
        if T.self == Void.self {
            return () as! T
        }
        
        do {
            return try decoder.decode(T.self, from: data)
        } catch {
            throw TCKClientError.decodingError(error)
        }
    }
}

// MARK: - URL Extensions

extension URL {
    func appendingQueryItem(_ name: String, value: String) -> URL {
        var components = URLComponents(url: self, resolvingAgainstBaseURL: true)
        var queryItems = components?.queryItems ?? []
        queryItems.append(URLQueryItem(name: name, value: value))
        components?.queryItems = queryItems
        return components?.url ?? self
    }
}
