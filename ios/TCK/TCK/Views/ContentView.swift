import SwiftUI
import TCKClient

struct ContentView: View {
    @Environment(AppViewModel.self) var appVM
    @State private var selectedTab: TabSelection = .instances
    
    enum TabSelection {
        case instances
        case terminal
        case chat
        case settings
    }
    
    var body: some View {
        ZStack {
            TabView(selection: $selectedTab) {
                InstancesView()
                    .tag(TabSelection.instances)
                    .tabItem {
                        Label("Instances", systemImage: "square.stack")
                    }
                
                if let instanceId = appVM.selectedInstanceId {
                    TerminalView(instanceId: instanceId)
                        .tag(TabSelection.terminal)
                        .tabItem {
                            Label("Terminal", systemImage: "terminal")
                        }
                    
                    ChatView()
                        .tag(TabSelection.chat)
                        .tabItem {
                            Label("Chat", systemImage: "bubble.left.and.bubble.right")
                        }
                } else {
                    Text("Select an instance first")
                        .tag(TabSelection.terminal)
                        .tabItem {
                            Label("Terminal", systemImage: "terminal")
                        }
                    
                    Text("Select an instance first")
                        .tag(TabSelection.chat)
                        .tabItem {
                            Label("Chat", systemImage: "bubble.left.and.bubble.right")
                        }
                }
                
                SettingsView()
                    .tag(TabSelection.settings)
                    .tabItem {
                        Label("Settings", systemImage: "gear")
                    }
            }
            
            if appVM.isLoading {
                ProgressView()
                    .tint(.blue)
            }
        }
        .alert("Error", isPresented: .constant(appVM.errorMessage != nil)) {
            Button("OK") {
                appVM.clearError()
            }
        } message: {
            Text(appVM.errorMessage ?? "Unknown error")
        }
    }
}

#Preview {
    ContentView()
        .environment(AppViewModel())
}
