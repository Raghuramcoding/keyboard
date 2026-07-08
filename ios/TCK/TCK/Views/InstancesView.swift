import SwiftUI
import TCKClient

struct InstancesView: View {
    @Environment(AppViewModel.self) var appVM
    @State private var showNewInstanceSheet = false
    @State private var newInstanceName = ""
    
    var body: some View {
        NavigationStack {
            List {
                if appVM.instances.isEmpty {
                    Text("No instances yet")
                        .foregroundColor(.gray)
                } else {
                    ForEach(appVM.instances) { instance in
                        InstanceRow(instance: instance, isSelected: appVM.selectedInstanceId == instance.id)
                            .onTapGesture {
                                appVM.selectInstance(instance.id)
                            }
                            .swipeActions(edge: .trailing) {
                                Button(role: .destructive) {
                                    Task {
                                        try? await appVM.deleteInstance(instance.id)
                                    }
                                } label: {
                                    Label("Delete", systemImage: "trash")
                                }
                            }
                    }
                }
            }
            .navigationTitle("Instances")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button(action: { showNewInstanceSheet = true }) {
                        Image(systemName: "plus")
                    }
                }
            }
            .sheet(isPresented: $showNewInstanceSheet) {
                NewInstanceSheet(isPresented: $showNewInstanceSheet) { name in
                    Task {
                        try? await appVM.createNewInstance(name: name)
                    }
                }
            }
            .refreshable {
                await appVM.refreshInstances()
            }
        }
    }
}

struct InstanceRow: View {
    let instance: Instance
    let isSelected: Bool
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(instance.name)
                    .font(.headline)
                if let description = instance.description {
                    Text(description)
                        .font(.caption)
                        .foregroundColor(.gray)
                }
            }
            
            Spacer()
            
            VStack(alignment: .trailing, spacing: 4) {
                StatusBadge(state: instance.state)
                if let role = instance.agentRole {
                    Text(role)
                        .font(.caption2)
                        .foregroundColor(.blue)
                }
            }
            
            if isSelected {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundColor(.blue)
            }
        }
        .contentShape(Rectangle())
    }
}

struct StatusBadge: View {
    let state: InstanceState
    
    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(statusColor)
                .frame(width: 8, height: 8)
            Text(state.rawValue.capitalized)
                .font(.caption2)
                .foregroundColor(.secondary)
        }
    }
    
    private var statusColor: Color {
        switch state {
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

struct NewInstanceSheet: View {
    @Binding var isPresented: Bool
    let onCreate: (String) -> Void
    @State private var name = ""
    
    var body: some View {
        NavigationStack {
            Form {
                TextField("Instance Name", text: $name)
            }
            .navigationTitle("New Instance")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Cancel") {
                        isPresented = false
                    }
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Create") {
                        onCreate(name)
                        isPresented = false
                    }
                    .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty)
                }
            }
        }
    }
}

#Preview {
    InstancesView()
        .environment(AppViewModel())
}
