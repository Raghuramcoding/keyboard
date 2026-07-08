import SwiftUI
import TCKClient

@main
struct TCKApp: App {
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
    }
}
