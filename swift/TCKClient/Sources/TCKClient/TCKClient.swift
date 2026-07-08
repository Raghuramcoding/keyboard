import Foundation

/// Main T.C.K client coordinating API, WebSocket, and terminal services
public class TCKClient {
    public let apiClient: APIClient
    public let terminalManager: TerminalManager
    public let settings: SettingsManager
    
    public init(serverURL: URL = URL(string: "http://127.0.0.1:3000")!) {
        self.apiClient = APIClient(baseURL: serverURL)
        self.terminalManager = TerminalManager(apiClient: apiClient)
        self.settings = SettingsManager.shared
        
        // Update settings with the provided server URL
        self.settings.serverURL = serverURL
    }
    
    /// Check if the T.C.K server is reachable
    public func checkServerHealth() async throws {
        try await apiClient.healthCheck()
    }
    
    /// Check if the AI service is available
    public func checkAIServiceHealth() async throws {
        try await apiClient.checkAIHealth()
    }
    
    // MARK: - Convenience Methods for Common Operations
    
    /// Get all terminal instances
    public func getInstances() async throws -> [Instance] {
        try await apiClient.listInstances()
    }
    
    /// Create a new terminal instance
    public func createInstance(name: String, agentRole: String? = nil) async throws -> Instance {
        let request = CreateInstanceRequest(name: name, agentRole: agentRole)
        return try await apiClient.createInstance(request: request)
    }
    
    /// Delete an instance
    public func deleteInstance(_ id: String) async throws {
        try await apiClient.deleteInstance(id: id)
    }
    
    /// Generate text using AI
    public func generateAIText(
        prompt: String,
        system: String? = nil,
        provider: String? = nil,
        model: String? = nil
    ) async throws -> String? {
        let settings = self.settings.loadSettings()
        
        let request = GenerateRequest(
            provider: provider ?? settings.selectedProvider,
            model: model ?? settings.selectedModel,
            prompt: prompt,
            system: system ?? settings.systemPrompt,
            temperature: settings.temperature,
            maxTokens: settings.maxTokens
        )
        
        let response = try await apiClient.generateText(request: request)
        
        if let error = response.error {
            throw NSError(domain: "AIGeneration", code: -1, userInfo: [NSLocalizedDescriptionKey: error])
        }
        
        return response.response
    }
    
    /// Fetch available AI models
    public func getAIModels() async throws -> [CatalogModel] {
        try await apiClient.fetchAIModels()
    }
    
    /// Get GitHub status
    public func getGitHubStatus() async throws -> GitHubStatus {
        try await apiClient.getGitHubStatus()
    }
    
    /// List GitHub repositories
    public func listGitHubRepos() async throws -> [RepoEntry] {
        try await apiClient.listGitHubRepos()
    }
    
    /// Get GitHub repository contents
    public func getGitHubRepoContents(repo: String, path: String = "") async throws -> [RepoFile] {
        try await apiClient.getGitHubRepoContents(repo: repo, path: path)
    }
}

// MARK: - Re-exports for convenience

public typealias TCKSettings = AppSettings
public typealias TCKInstance = Instance
public typealias TCKChatMessage = ChatMessage
public typealias TCKError = TCKClientError
