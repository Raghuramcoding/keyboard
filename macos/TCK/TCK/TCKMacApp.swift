import SwiftUI
import TCKClient

@main
struct TCKMacApp: App {
    @State private var appVM = AppViewModel()
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(appVM)
                .onAppear {
                    Task {
                        await appVM.initialize()
                    }
                }
        }
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentMinSize(CGSize(width: 800, height: 600)))
    }
}
