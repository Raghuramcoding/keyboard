import SwiftUI
import TCKClient

struct ChatView: View {
    @Environment(AppViewModel.self) var appVM
    @State private var chatVM: ChatViewModel?
    @State private var messageText = ""
    @State private var autoScroll = true
    
    var body: some View {
        VStack(spacing: 0) {
            // Messages area
            VStack(alignment: .leading, spacing: 8) {
                ScrollViewReader { proxy in
                    ScrollView(.vertical) {
                        VStack(alignment: .leading, spacing: 12) {
                            ForEach(chatVM?.messages ?? []) { message in
                                MessageBubble(message: message)
                                    .id(message.id)
                            }
                        }
                        .padding(12)
                        .onChange(of: chatVM?.messages) { _, _ in
                            if autoScroll {
                                withAnimation {
                                    proxy.scrollTo((chatVM?.messages.last?.id), anchor: .bottom)
                                }
                            }
                        }
                    }
                    .background(Color(.systemGray6))
                }
            }
            .frame(maxHeight: .infinity)
            
            Divider()
            
            // Input area
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
                
                Toggle("Auto-scroll", isOn: $autoScroll)
                    .font(.caption)
            }
            .padding(8)
            
            if chatVM?.isLoading == true {
                ProgressView()
                    .font(.caption)
            }
        }
        .onAppear {
            let vm = ChatViewModel(client: appVM.client)
            chatVM = vm
        }
        .alert("Error", isPresented: .constant(chatVM?.errorMessage != nil)) {
            Button("OK") {
                chatVM?.clearError()
            }
        } message: {
            Text(chatVM?.errorMessage ?? "Unknown error")
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

struct MessageBubble: View {
    let message: ChatMessage
    
    var body: some View {
        HStack {
            if message.role == .user {
                Spacer()
            }
            
            VStack(alignment: message.role == .user ? .trailing : .leading, spacing: 4) {
                Text(message.text)
                    .padding(12)
                    .background(backgroundColor)
                    .foregroundColor(textColor)
                    .cornerRadius(12)
                
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
            return .gray
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
            return message.role == .user ? .white : .black
        }
    }
}

#Preview {
    ChatView()
        .environment(AppViewModel())
}
