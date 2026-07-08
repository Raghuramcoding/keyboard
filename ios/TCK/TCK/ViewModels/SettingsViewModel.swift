import SwiftUI
import TCKClient

@Observable
final class SettingsViewModel {
    let settings: SettingsManager
    
    @ObservationIgnored
    var serverURL: String
    
    @ObservationIgnored
    var selectedProvider: String
    
    @ObservationIgnored
    var selectedModel: String
    
    @ObservationIgnored
    var temperature: Double
    
    @ObservationIgnored
    var maxTokens: Int
    
    @ObservationIgnored
    var systemPrompt: String
    
    @ObservationIgnored
    var providers: [ProviderConfig]
    
    init(settings: SettingsManager) {
        self.settings = settings
        
        let loaded = settings.loadSettings()
        self.serverURL = loaded.serverURL.absoluteString
        self.selectedProvider = loaded.selectedProvider
        self.selectedModel = loaded.selectedModel
        self.temperature = loaded.temperature
        self.maxTokens = loaded.maxTokens
        self.systemPrompt = loaded.systemPrompt
        self.providers = loaded.providers
    }
    
    func save() {
        var settings = self.settings.loadSettings()
        settings.selectedProvider = selectedProvider
        settings.selectedModel = selectedModel
        settings.temperature = temperature
        settings.maxTokens = maxTokens
        settings.systemPrompt = systemPrompt
        
        if let url = URL(string: serverURL) {
            settings.serverURL = url
        }
        
        self.settings.saveSettings(settings)
    }
    
    func setAPIKey(_ key: String, for provider: String) throws {
        try settings.setAPIKey(key, for: provider)
    }
    
    func getAPIKey(for provider: String) throws -> String? {
        try settings.getAPIKey(for: provider)
    }
}

struct SettingsView: View {
    @Environment(AppViewModel.self) var appVM
    @State private var settingsVM: SettingsViewModel?
    @State private var showAPIKeySheet = false
    @State private var apiKeyInput = ""
    @State private var selectedProviderForKey: String?
    
    var body: some View {
        NavigationStack {
            Form {
                Section("Server") {
                    TextField("Server URL", text: Binding(
                        get: { settingsVM?.serverURL ?? "" },
                        set: { settingsVM?.serverURL = $0 }
                    ))
                }
                
                Section("AI Provider") {
                    Picker("Provider", selection: Binding(
                        get: { settingsVM?.selectedProvider ?? "anthropic" },
                        set: { settingsVM?.selectedProvider = $0 }
                    )) {
                        ForEach(["anthropic", "openai", "google", "ollama"], id: \.self) { provider in
                            Text(provider.capitalized).tag(provider)
                        }
                    }
                    
                    TextField("Model", text: Binding(
                        get: { settingsVM?.selectedModel ?? "" },
                        set: { settingsVM?.selectedModel = $0 }
                    ))
                }
                
                Section("Generation Parameters") {
                    Slider(
                        value: Binding(
                            get: { settingsVM?.temperature ?? 0.7 },
                            set: { settingsVM?.temperature = $0 }
                        ),
                        in: 0...2,
                        step: 0.1
                    )
                    Text("Temperature: \(String(format: "%.1f", settingsVM?.temperature ?? 0.7))")
                        .font(.caption)
                    
                    Stepper(
                        "Max Tokens: \(settingsVM?.maxTokens ?? 4096)",
                        value: Binding(
                            get: { settingsVM?.maxTokens ?? 4096 },
                            set: { settingsVM?.maxTokens = $0 }
                        ),
                        in: 256...16384,
                        step: 256
                    )
                }
                
                Section("System Prompt") {
                    TextEditor(text: Binding(
                        get: { settingsVM?.systemPrompt ?? "" },
                        set: { settingsVM?.systemPrompt = $0 }
                    ))
                    .frame(minHeight: 100)
                }
                
                Section("API Keys") {
                    ForEach(["anthropic", "openai", "google", "ollama"], id: \.self) { provider in
                        HStack {
                            Text(provider.capitalized)
                            Spacer()
                            Button("Configure") {
                                selectedProviderForKey = provider
                                apiKeyInput = (try? settingsVM?.getAPIKey(for: provider)) ?? ""
                                showAPIKeySheet = true
                            }
                        }
                    }
                }
                
                Section {
                    Button("Save") {
                        settingsVM?.save()
                    }
                    .frame(maxWidth: .infinity)
                    .tint(.blue)
                }
            }
            .navigationTitle("Settings")
            .onAppear {
                if settingsVM == nil {
                    settingsVM = SettingsViewModel(settings: appVM.client.settings)
                }
            }
            .sheet(isPresented: $showAPIKeySheet) {
                APIKeySheet(
                    provider: selectedProviderForKey ?? "openai",
                    apiKey: $apiKeyInput,
                    isPresented: $showAPIKeySheet,
                    onSave: { provider, key in
                        try? settingsVM?.setAPIKey(key, for: provider)
                    }
                )
            }
        }
    }
}

struct APIKeySheet: View {
    let provider: String
    @Binding var apiKey: String
    @Binding var isPresented: Bool
    let onSave: (String, String) -> Void
    @State private var showPassword = false
    
    var body: some View {
        NavigationStack {
            Form {
                Section("API Key") {
                    if showPassword {
                        TextField("\(provider) API Key", text: $apiKey)
                    } else {
                        SecureField("\(provider) API Key", text: $apiKey)
                    }
                    
                    Toggle("Show", isOn: $showPassword)
                }
            }
            .navigationTitle("\(provider.capitalized) API Key")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Cancel") {
                        isPresented = false
                    }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Save") {
                        onSave(provider, apiKey)
                        isPresented = false
                    }
                    .disabled(apiKey.isEmpty)
                }
            }
        }
    }
}

#Preview {
    SettingsView()
        .environment(AppViewModel())
}
