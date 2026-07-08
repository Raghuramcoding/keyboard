import SwiftUI
import TCKClient

@Observable
final class TerminalViewModel {
    let terminalService: TerminalService
    
    @ObservationIgnored
    private(set) var output: String = ""
    
    @ObservationIgnored
    private(set) var isConnected = false
    
    @ObservationIgnored
    private(set) var errorMessage: String?
    
    init(apiClient: APIClient, instanceId: String) {
        self.terminalService = TerminalService(apiClient: apiClient, instanceId: instanceId)
        setupHandlers()
    }
    
    private func setupHandlers() {
        terminalService.onOutput = { [weak self] text in
            self?.output.append(text)
        }
        
        terminalService.onError = { [weak self] error in
            self?.errorMessage = error.localizedDescription
        }
        
        terminalService.onStateChange = { [weak self] state in
            self?.isConnected = (state == .connected)
        }
    }
    
    func connect() {
        terminalService.connect()
    }
    
    func disconnect() {
        terminalService.disconnect()
    }
    
    func sendCommand(_ command: String) async throws {
        try await terminalService.sendInput(command)
    }
    
    func resize(rows: Int, cols: Int) async throws {
        try await terminalService.resize(rows: rows, cols: cols)
    }
    
    func clearOutput() {
        output = ""
    }
    
    func clearError() {
        errorMessage = nil
    }
}
