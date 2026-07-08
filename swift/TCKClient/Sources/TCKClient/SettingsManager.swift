import Foundation

/// Manages app settings and secure credential storage
public class SettingsManager {
    public static let shared = SettingsManager()
    
    private let userDefaults = UserDefaults(suiteName: "com.tck.app") ?? UserDefaults.standard
    private let keychainPrefix = "com.tck"
    
    private let settingsKey = "app.settings"
    private let providersKey = "app.providers"
    
    public init() {}
    
    // MARK: - App Settings
    
    /// Load app settings
    public func loadSettings() -> AppSettings {
        if let data = userDefaults.data(forKey: settingsKey) {
            let decoder = JSONDecoder()
            decoder.dateDecodingStrategy = .iso8601
            
            if let settings = try? decoder.decode(AppSettings.self, from: data) {
                return settings
            }
        }
        
        return AppSettings()
    }
    
    /// Save app settings
    public func saveSettings(_ settings: AppSettings) {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        
        if let data = try? encoder.encode(settings) {
            userDefaults.set(data, forKey: settingsKey)
        }
    }
    
    // MARK: - Provider Credentials
    
    /// Store API key securely
    public func setAPIKey(_ key: String, for provider: String) throws {
        let keychain = Keychain()
        try keychain.set(key, for: keychainKey(for: provider))
    }
    
    /// Retrieve API key from secure storage
    public func getAPIKey(for provider: String) throws -> String? {
        let keychain = Keychain()
        return try keychain.get(for: keychainKey(for: provider))
    }
    
    /// Delete API key
    public func deleteAPIKey(for provider: String) throws {
        let keychain = Keychain()
        try keychain.delete(for: keychainKey(for: provider))
    }
    
    // MARK: - Server Configuration
    
    public var serverURL: URL {
        get {
            if let urlString = userDefaults.string(forKey: "serverURL"),
               let url = URL(string: urlString) {
                return url
            }
            return URL(string: "http://127.0.0.1:3000") ?? URL(fileURLWithPath: "/")
        }
        set {
            userDefaults.set(newValue.absoluteString, forKey: "serverURL")
        }
    }
    
    // MARK: - UI State
    
    public var selectedInstanceId: String? {
        get { userDefaults.string(forKey: "selectedInstanceId") }
        set { userDefaults.set(newValue, forKey: "selectedInstanceId") }
    }
    
    public var terminalFontSize: CGFloat {
        get {
            let size = userDefaults.float(forKey: "terminalFontSize")
            return size > 0 ? CGFloat(size) : 12.0
        }
        set {
            userDefaults.set(Float(newValue), forKey: "terminalFontSize")
        }
    }
    
    public var chatPanelVisible: Bool {
        get { userDefaults.bool(forKey: "chatPanelVisible") || true }
        set { userDefaults.set(newValue, forKey: "chatPanelVisible") }
    }
    
    // MARK: - Private Helpers
    
    private func keychainKey(for provider: String) -> String {
        "\(keychainPrefix).provider.\(provider).apikey"
    }
}

// MARK: - Keychain Helper

/// Simple Keychain wrapper for secure storage on Apple platforms
class Keychain {
    func set(_ value: String, for key: String) throws {
        guard let data = value.data(using: .utf8) else {
            throw NSError(domain: "Keychain", code: -1, userInfo: nil)
        }
        
        // Try to delete existing item first
        _ = try? delete(for: key)
        
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
        ]
        
        let status = SecItemAdd(query as CFDictionary, nil)
        
        guard status == errSecSuccess else {
            throw NSError(domain: "Keychain", code: Int(status), userInfo: nil)
        }
    }
    
    func get(for key: String) throws -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
        ]
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        guard status != errSecItemNotFound else {
            return nil
        }
        
        guard status == errSecSuccess else {
            throw NSError(domain: "Keychain", code: Int(status), userInfo: nil)
        }
        
        guard let data = result as? Data else {
            return nil
        }
        
        return String(data: data, encoding: .utf8)
    }
    
    func delete(for key: String) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
        ]
        
        let status = SecItemDelete(query as CFDictionary)
        
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw NSError(domain: "Keychain", code: Int(status), userInfo: nil)
        }
    }
}

// MARK: - Extensions

extension CGFloat {
    public static var zero: CGFloat { 0 }
}
