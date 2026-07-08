import SwiftUI
import TCKClient

@Observable
final class ChatViewModel {
    let client: TCKClient
    
    @ObservationIgnored
    private(set) var messages: [ChatMessage] = []
    
    @ObservationIgnored
    private(set) var isLoading = false
    
    @ObservationIgnored
    private(set) var errorMessage: String?
    
    @ObservationIgnored
    private(set) var availableModels: [CatalogModel] = []
    
    init(client: TCKClient) {
        self.client = client
        Task {
            await fetchModels()
        }
    }
    
    func sendMessage(_ text: String) async {
        // Add user message
        messages.append(ChatMessage(role: .user, text: text))
        
        isLoading = true
        errorMessage = nil
        
        do {
            if let response = try await client.generateAIText(prompt: text) {
                messages.append(ChatMessage(role: .assistant, text: response))
            } else {
                errorMessage = "No response from AI"
            }
        } catch {
            errorMessage = error.localizedDescription
            messages.append(ChatMessage(role: .error, text: "Error: \(error.localizedDescription)"))
        }
        
        isLoading = false
    }
    
    func fetchModels() async {
        do {
            availableModels = try await client.getAIModels()
        } catch {
            errorMessage = "Failed to load models: \(error.localizedDescription)"
        }
    }
    
    func clearMessages() {
        messages.removeAll()
    }
    
    func clearError() {
        errorMessage = nil
    }
}
