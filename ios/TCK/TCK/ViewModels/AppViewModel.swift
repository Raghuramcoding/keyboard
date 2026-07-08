import SwiftUI
import TCKClient

@Observable
final class AppViewModel {
    let client: TCKClient
    
    @ObservationIgnored
    private(set) var instances: [Instance] = []
    
    @ObservationIgnored
    private(set) var selectedInstanceId: String?
    
    @ObservationIgnored
    private(set) var serverHealth: Bool = false
    
    @ObservationIgnored
    private(set) var aiHealth: Bool = false
    
    @ObservationIgnored
    private(set) var isLoading = false
    
    @ObservationIgnored
    private(set) var errorMessage: String?
    
    init(serverURL: URL = URL(string: "http://127.0.0.1:3000")!) {
        self.client = TCKClient(serverURL: serverURL)
        self.selectedInstanceId = client.settings.selectedInstanceId
    }
    
    func initialize() async {
        isLoading = true
        
        do {
            // Check server health
            try await client.checkServerHealth()
            serverHealth = true
            
            // Check AI service health
            try? await client.checkAIServiceHealth()
            aiHealth = true
            
            // Fetch instances
            await refreshInstances()
        } catch {
            errorMessage = "Failed to initialize: \(error.localizedDescription)"
            serverHealth = false
        }
        
        isLoading = false
    }
    
    func refreshInstances() async {
        do {
            instances = try await client.getInstances()
        } catch {
            errorMessage = "Failed to load instances: \(error.localizedDescription)"
        }
    }
    
    func selectInstance(_ id: String) {
        selectedInstanceId = id
        client.settings.selectedInstanceId = id
    }
    
    func createNewInstance(name: String, agentRole: String? = nil) async throws -> Instance {
        let instance = try await client.createInstance(name: name, agentRole: agentRole)
        instances.append(instance)
        selectInstance(instance.id)
        return instance
    }
    
    func deleteInstance(_ id: String) async throws {
        try await client.deleteInstance(id)
        instances.removeAll { $0.id == id }
        if selectedInstanceId == id {
            selectedInstanceId = instances.first?.id
        }
    }
    
    func clearError() {
        errorMessage = nil
    }
}
