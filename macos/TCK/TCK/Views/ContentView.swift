import SwiftUI
import TCKClient

struct TerminalViewMac: View {
    @Environment(AppViewModel.self) var appVM
    @State private var terminalVM: TerminalViewModel?
    @State private var commandInput = ""
    @State private var autoScroll = true
    
    let instanceId: String
    
    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack(spacing: 8) {
                Text("Terminal — \(instanceId)")
                    .font(.headline)
                
                Spacer()
                
                Circle()
                    .fill(terminalVM?.isConnected == true ? Color.green : Color.red)
                    .frame(width: 8, height: 8)
                
                Text(terminalVM?.isConnected == true ? "Connected" : "Disconnected")
                    .font(.caption)
                
                Toggle("Auto-scroll", isOn: $autoScroll)
                    .font(.caption)
            }
            .padding(8)
            .background(Color(.controlBackgroundColor))
            
            Divider()
            
            // Output
            ScrollViewReader { proxy in
                ScrollView(.vertical) {
                    Text(terminalVM?.output ?? "Waiting for connection...")
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.green)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(8)
                        .id("bottom")
                        .onChange(of: terminalVM?.output) { _, _ in
                            if autoScroll {
                                withAnimation {
                                    proxy.scrollTo("bottom", anchor: .bottom)
                                }
                            }
                        }
                }
                .background(Color.black)
            }
            
            Divider()
            
            // Input
            HStack(spacing: 8) {
                TextField("Enter command", text: $commandInput)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit(sendCommand)
                
                Button(action: sendCommand) {
                    Image(systemName: "arrow.right")
                }
                .disabled(commandInput.isEmpty || terminalVM?.isConnected == false)
            }
            .padding(8)
        }
        .onAppear {
            let vm = TerminalViewModel(apiClient: appVM.client.apiClient, instanceId: instanceId)
            terminalVM = vm
            vm.connect()
        }
        .onDisappear {
            terminalVM?.disconnect()
        }
    }
    
    private func sendCommand() {
        guard let vm = terminalVM, !commandInput.isEmpty else { return }
        
        Task {
            do {
                try await vm.sendCommand(commandInput + "\n")
                commandInput = ""
            } catch {
                vm.errorMessage = error.localizedDescription
            }
        }
    }
}

struct ChatViewMac: View {
    @Environment(AppViewModel.self) var appVM
    @State private var chatVM: ChatViewModel?
    @State private var messageText = ""
    @State private var autoScroll = true
    
    var body: some View {
        VStack(spacing: 0) {
            Text("Assistant")
                .font(.headline)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(8)
                .background(Color(.controlBackgroundColor))
            
            Divider()
            
            // Messages
            ScrollViewReader { proxy in
                ScrollView(.vertical) {
                    VStack(alignment: .leading, spacing: 8) {
                        ForEach(chatVM?.messages ?? []) { message in
                            MessageBubbleMac(message: message)
                                .id(message.id)
                        }
                    }
                    .padding(8)
                    .onChange(of: chatVM?.messages) { _, _ in
                        if autoScroll {
                            withAnimation {
                                proxy.scrollTo((chatVM?.messages.last?.id), anchor: .bottom)
                            }
                        }
                    }
                }
                .background(Color(.controlBackgroundColor))
            }
            
            Divider()
            
            // Input
            VStack(spacing: 8) {
                HStack(spacing: 8) {
                    TextField("Type a message...", text: $messageText, axis: .vertical)
                        .lineLimit(3)
                        .textFieldStyle(.roundedBorder)
                    
                    Button(action: sendMessage) {
                        Image(systemName: "paperplane.fill")
                    }
                    .disabled(messageText.isEmpty || chatVM?.isLoading == true)
                }
                
                if chatVM?.isLoading == true {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                }
            }
            .padding(8)
        }
        .onAppear {
            let vm = ChatViewModel(client: appVM.client)
            chatVM = vm
        }
    }
    
    private func sendMessage() {
        guard let vm = chatVM, !messageText.isEmpty else { return }
        
        let text = messageText
        messageText = ""
        
        Task {
            await vm.sendMessage(text)
        }
    }
}

struct MessageBubbleMac: View {
    let message: ChatMessage
    
    var body: some View {
        HStack {
            if message.role == .user {
                Spacer()
            }
            
            VStack(alignment: message.role == .user ? .trailing : .leading, spacing: 4) {
                Text(message.text)
                    .padding(8)
                    .background(backgroundColor)
                    .foregroundColor(textColor)
                    .cornerRadius(8)
                
                if let timestamp = message.timestamp {
                    Text(timestamp.formatted(date: .omitted, time: .shortened))
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            
            if message.role == .assistant {
                Spacer()
            }
        }
    }
    
    private var backgroundColor: Color {
        switch message.role {
        case .user:
            return .blue
        case .assistant:
            return Color(.controlBackgroundColor)
        case .error:
            return .red
        default:
            return .gray
        }
    }
    
    private var textColor: Color {
        switch message.role {
        case .error:
            return .white
        default:
            return message.role == .user ? .white : .primary
        }
    }
}

struct SettingsWindowMac: View {
    @Environment(\.dismiss) var dismiss
    @State private var settingsVM: SettingsViewModel?
    @State private var apiKeyInput = ""
    @State private var selectedProviderForKey: String?
    @State private var showAPIKeySheet = false
    
    var body: some View {
        VStack {
            TabView {
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
                }
                .tabItem {
                    Label("General", systemImage: "gear")
                }
                
                Form {
                    Section("Temperature") {
                        Slider(
                            value: Binding(
                                get: { settingsVM?.temperature ?? 0.7 },
                                set: { settingsVM?.temperature = $0 }
                            ),
                            in: 0...2,
                            step: 0.1
                        )
                        Text(String(format: "%.1f", settingsVM?.temperature ?? 0.7))
                            .font(.caption)
                    }
                    
                    Section("Max Tokens") {
                        Stepper(
                            "\(settingsVM?.maxTokens ?? 4096)",
                            value: Binding(
                                get: { settingsVM?.maxTokens ?? 4096 },
                                set: { settingsVM?.maxTokens = $0 }
                            ),
                            in: 256...16384,
                            step: 256
                        )
                    }
                }
                .tabItem {
                    Label("Generation", systemImage: "sparkles")
                }
                
                Form {
                    Section("System Prompt") {
                        TextEditor(text: Binding(
                            get: { settingsVM?.systemPrompt ?? "" },
                            set: { settingsVM?.systemPrompt = $0 }
                        ))
                    }
                }
                .tabItem {
                    Label("Prompt", systemImage: "text.aligncenter")
                }
                
                VStack(spacing: 12) {
                    Text("API Keys")
                        .font(.headline)
                    
                    List {
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
                }
                .tabItem {
                    Label("API Keys", systemImage: "key")
                }
            }
            
            Divider()
            
            HStack(spacing: 8) {
                Spacer()
                Button("Cancel") {
                    dismiss()
                }
                .keyboardShortcut(.cancelAction)
                
                Button("Save") {
                    settingsVM?.save()
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
            }
            .padding(12)
        }
        .frame(width: 500, height: 600)
        .onAppear {
            if settingsVM == nil {
                // We need to get the AppViewModel from environment
                settingsVM = SettingsViewModel(settings: SettingsManager.shared)
            }
        }
        .sheet(isPresented: $showAPIKeySheet) {
            APIKeySheetMac(
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

struct APIKeySheetMac: View {
    let provider: String
    @Binding var apiKey: String
    @Binding var isPresented: Bool
    let onSave: (String, String) -> Void
    @State private var showPassword = false
    
    var body: some View {
        VStack(spacing: 12) {
            Text("\(provider.capitalized) API Key")
                .font(.headline)
            
            if showPassword {
                TextField("API Key", text: $apiKey)
                    .textFieldStyle(.roundedBorder)
            } else {
                SecureField("API Key", text: $apiKey)
                    .textFieldStyle(.roundedBorder)
            }
            
            Toggle("Show", isOn: $showPassword)
            
            HStack(spacing: 8) {
                Spacer()
                Button("Cancel") {
                    isPresented = false
                }
                .keyboardShortcut(.cancelAction)
                
                Button("Save") {
                    onSave(provider, apiKey)
                    isPresented = false
                }
                .keyboardShortcut(.defaultAction)
                .disabled(apiKey.isEmpty)
            }
        }
        .padding(20)
        .frame(width: 400)
    }
}

#Preview {
    ContentView()
        .environment(AppViewModel())
}
import SwiftUI
import TCKClient

struct ContentView: View {
    @Environment(AppViewModel.self) var appVM
    @State private var selectedInstanceId: String?
    
    var body: some View {
        HStack(spacing: 0) {
            // Sidebar: Instances
            VStack(spacing: 0) {
                InstancesSidebarView(selectedInstanceId: $selectedInstanceId)
                    .environmentObject(appVM)
            }
            .frame(width: 250)
            .background(Color(.controlBackgroundColor))
            
            Divider()
            
            // Main content area
            if let instanceId = selectedInstanceId {
                HStack(spacing: 0) {
                    // Left: Terminal
                    TerminalViewMac(instanceId: instanceId)
                        .frame(maxWidth: .infinity)
                    
                    Divider()
                    
                    // Right: Chat
                    ChatViewMac()
                        .frame(width: 300)
                }
            } else {
                VStack {
                    Image(systemName: "square.stack")
                        .font(.system(size: 48))
                        .foregroundColor(.gray)
                    Text("Select an instance to begin")
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .onAppear {
            selectedInstanceId = appVM.selectedInstanceId
        }
        .onChange(of: appVM.selectedInstanceId) { _, newId in
            selectedInstanceId = newId
        }
        .toolbar {
            ToolbarItem(placement: .automatic) {
                Button(action: { showSettings = true }) {
                    Image(systemName: "gear")
                }
            }
        }
        .sheet(isPresented: $showSettings) {
            SettingsWindowMac()
        }
    }
    
    @State private var showSettings = false
}

struct InstancesSidebarView: View {
    @Binding var selectedInstanceId: String?
    @EnvironmentObject var appVM: AppViewModel
    @State private var showNewInstanceSheet = false
    @State private var newInstanceName = ""
    
    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Instances")
                    .font(.headline)
                Spacer()
                Button(action: { showNewInstanceSheet = true }) {
                    Image(systemName: "plus")
                }
                .buttonStyle(.plain)
            }
            .padding(12)
            
            Divider()
            
            ScrollView(.vertical) {
                VStack(alignment: .leading, spacing: 4) {
                    if appVM.instances.isEmpty {
                        Text("No instances")
                            .foregroundColor(.secondary)
                            .padding(12)
                    } else {
                        ForEach(appVM.instances) { instance in
                            InstanceSidebarRow(
                                instance: instance,
                                isSelected: selectedInstanceId == instance.id,
                                onSelect: {
                                    selectedInstanceId = instance.id
                                    appVM.selectInstance(instance.id)
                                },
                                onDelete: {
                                    Task {
                                        try? await appVM.deleteInstance(instance.id)
                                    }
                                }
                            )
                        }
                    }
                }
                .padding(8)
            }
            
            Spacer()
            
            Divider()
            
            HStack(spacing: 4) {
                if appVM.serverHealth {
                    Circle()
                        .fill(Color.green)
                        .frame(width: 8, height: 8)
                    Text("Server")
                        .font(.caption)
                } else {
                    Circle()
                        .fill(Color.red)
                        .frame(width: 8, height: 8)
                    Text("Offline")
                        .font(.caption)
                }
                
                Spacer()
                
                Button(action: { Task { await appVM.refreshInstances() } }) {
                    Image(systemName: "arrow.clockwise")
                }
                .buttonStyle(.plain)
            }
            .padding(12)
        }
        .sheet(isPresented: $showNewInstanceSheet) {
            NewInstanceSheetMac(isPresented: $showNewInstanceSheet) { name in
                Task {
                    try? await appVM.createNewInstance(name: name)
                }
            }
        }
    }
}

struct InstanceSidebarRow: View {
    let instance: Instance
    let isSelected: Bool
    let onSelect: () -> Void
    let onDelete: () -> Void
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(instance.name)
                    .font(.headline)
                    .lineLimit(1)
                if let role = instance.agentRole {
                    Text(role)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            
            Spacer()
            
            Circle()
                .fill(statusColor)
                .frame(width: 6, height: 6)
        }
        .padding(8)
        .background(isSelected ? Color.blue.opacity(0.2) : Color.clear)
        .cornerRadius(4)
        .onTapGesture(perform: onSelect)
        .contextMenu {
            Button(role: .destructive) {
                onDelete()
            } label: {
                Label("Delete", systemImage: "trash")
            }
        }
    }
    
    private var statusColor: Color {
        switch instance.state {
        case .running:
            return .green
        case .idle:
            return .gray
        case .paused:
            return .yellow
        case .error:
            return .red
        }
    }
}

struct NewInstanceSheetMac: View {
    @Binding var isPresented: Bool
    let onCreate: (String) -> Void
    @State private var name = ""
    
    var body: some View {
        VStack(spacing: 12) {
            Text("New Instance")
                .font(.headline)
            
            TextField("Name", text: $name)
                .textFieldStyle(.roundedBorder)
            
            HStack(spacing: 8) {
                Button("Cancel") {
                    isPresented = false
                }
                .keyboardShortcut(.cancelAction)
                
                Spacer()
                
                Button("Create") {
                    onCreate(name)
                    isPresented = false
                }
                .keyboardShortcut(.defaultAction)
                .disabled(name.isEmpty)
            }
        }
        .padding(20)
        .frame(width: 400)
    }
}

#Preview {
    ContentView()
        .environment(AppViewModel())
}
