import SwiftUI
import TCKClient

struct TerminalView: View {
    @Environment(AppViewModel.self) var appVM
    @State private var terminalVM: TerminalViewModel?
    @State private var commandInput = ""
    @State private var autoScroll = true
    
    let instanceId: String
    
    var body: some View {
        VStack(spacing: 0) {
            // Output area
            VStack(alignment: .leading) {
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
            }
            .frame(maxHeight: .infinity)
            
            Divider()
            
            // Input area
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
            
            // Connection status
            HStack(spacing: 8) {
                Circle()
                    .fill(terminalVM?.isConnected == true ? Color.green : Color.red)
                    .frame(width: 8, height: 8)
                
                Text(terminalVM?.isConnected == true ? "Connected" : "Disconnected")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                Spacer()
                
                Toggle("Auto-scroll", isOn: $autoScroll)
                    .font(.caption)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(Color(.systemGray6))
        }
        .onAppear {
            let vm = TerminalViewModel(apiClient: appVM.client.apiClient, instanceId: instanceId)
            terminalVM = vm
            vm.connect()
        }
        .onDisappear {
            terminalVM?.disconnect()
        }
        .alert("Error", isPresented: .constant(terminalVM?.errorMessage != nil)) {
            Button("OK") {
                terminalVM?.clearError()
            }
        } message: {
            Text(terminalVM?.errorMessage ?? "Unknown error")
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

#Preview {
    TerminalView(instanceId: "test-123")
        .environment(AppViewModel())
}
