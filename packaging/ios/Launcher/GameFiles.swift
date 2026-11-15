import Security
import SwiftUI
import UIKit
import UniformTypeIdentifiers

/// Prepared game files downloaded automatically on first launch, or imported from a custom link.
/// Both routes use the game's checked importer. Raw Java assets are prepared on a computer.
@MainActor
final class GameFilesDownload: NSObject, ObservableObject {
    enum State: Equatable {
        case idle
        case downloading(received: Int64, total: Int64)
        case installing
        case done(String)
        case failed(String)
    }

    @Published private(set) var state: State = .idle
    @Published var link: String = UserDefaults.standard.string(forKey: "gameFilesLink") ?? ""
    @Published var token: String = Keychain.read() ?? ""

    typealias ImportArchive = @MainActor (String) async -> [String: Any]
    private let importArchive: ImportArchive
    private let automaticSource: @MainActor () async -> [String: Any]
    private let diagnostic: @MainActor (String) -> Void
    private var startedAt = ProcessInfo.processInfo.systemUptime
    private var lastProgressAt = 0.0
    private var receivedBytes: Int64 = 0
    private var expectedBytes: Int64 = 0

    private func trace(_ event: String) {
        let elapsed = Int(max(0, ProcessInfo.processInfo.systemUptime - startedAt) * 1000)
        diagnostic("Game files [\(attempt.uuidString.prefix(8))] \(event) elapsed_ms=\(elapsed)")
    }

    private func traceStorage() {
        let free = (try? FileManager.default.attributesOfFileSystem(forPath: GameFolder.documents.path)[.systemFreeSize] as? NSNumber)?.int64Value
        trace("storage free_bytes=\(free ?? -1)")
    }

    /// Never log NSError.userInfo/descriptions: these can contain URLs and credentials.
    nonisolated static func errorFacts(_ error: Error) -> String {
        let e = error as NSError
        let domain = [NSURLErrorDomain, NSPOSIXErrorDomain, NSCocoaErrorDomain].contains(e.domain) ? e.domain : "other"
        let kind: String
        switch (e.domain, e.code) {
        case (NSURLErrorDomain, NSURLErrorCannotFindHost): kind = "dns"
        case (NSURLErrorDomain, NSURLErrorCannotConnectToHost): kind = "connection"
        case (NSURLErrorDomain, NSURLErrorTimedOut): kind = "timeout"
        case (NSURLErrorDomain, NSURLErrorNotConnectedToInternet): kind = "offline"
        case (NSURLErrorDomain, NSURLErrorNetworkConnectionLost): kind = "connection_lost"
        case (NSURLErrorDomain, NSURLErrorCancelled): kind = "cancelled"
        case (NSURLErrorDomain, -1206 ... -1200): kind = "tls"
        case (NSCocoaErrorDomain, NSFileWriteOutOfSpaceError): kind = "storage_full"
        default: kind = "other"
        }
        var facts = "kind=\(kind) domain=\(domain) code=\(e.code)"
        if let underlying = e.userInfo[NSUnderlyingErrorKey] as? NSError {
            let safe = [NSURLErrorDomain, NSPOSIXErrorDomain, NSCocoaErrorDomain].contains(underlying.domain) ? underlying.domain : "other"
            facts += " underlying_domain=\(safe) underlying_code=\(underlying.code)"
        }
        return facts
    }
    private var automaticAttempted = false
    // Same packed-ZIP limit as horizon_client::import::MAX_PACKED. Enforce
    // it during transfer, before an oversized archive fills the sandbox.
    private nonisolated static let maximumArchiveBytes: Int64 = 2 << 30
    private var resourcesOnly = false
    private var resolvingAutomatic = false
    private var attempt = UUID()
    private var session: URLSession?

    init(importArchive: @escaping ImportArchive = { await Bridge.send($0) },
         automaticSource: @escaping @MainActor () async -> [String: Any] = { await Bridge.send("game-files-source") },
         diagnostic: @escaping @MainActor (String) -> Void = { Diagnostics.record($0) }) {
        self.importArchive = importArchive
        self.automaticSource = automaticSource
        self.diagnostic = diagnostic
        super.init()
    }
    private var background: UIBackgroundTaskIdentifier = .invalid

    var busy: Bool {
        if resolvingAutomatic { return true }
        switch state {
        case .downloading, .installing: return true
        default: return false
        }
    }

    /// Only automatic setup failures belong in the Play panel. Custom-link and
    /// local-file errors remain in their own setup UI.
    var automaticFailure: String? {
        guard resourcesOnly, case let .failed(message) = state else { return nil }
        return message
    }

    /// Where a finished download waits for the importer; it consumes the file.
    private nonisolated static var destination: URL {
        GameFolder.documents.appendingPathComponent("horizon-download-\(UUID().uuidString).zip")
    }

    func start() {
        guard !busy else { return }
        attempt = UUID()
        startedAt = ProcessInfo.processInfo.systemUptime
        resourcesOnly = false
        trace("custom_start")
        let text = link.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let url = URL(string: text), let scheme = url.scheme?.lowercased(),
              scheme == "https" || scheme == "http", url.host != nil
        else {
            trace("custom_source_rejected invalid_url")
            state = .failed("That is not a web link. It starts with https:// (or http:// on your own network).")
            return
        }
        let token = token.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            try Keychain.write(token.isEmpty ? nil : token)
        } catch {
            trace("credential_storage_failed \(Self.errorFacts(error))")
            state = .failed(error.localizedDescription)
            return
        }
        UserDefaults.standard.set(text, forKey: "gameFilesLink")
        begin(url: url, token: token.isEmpty ? nil : token)
    }

    /// First-run setup stays in the launcher; it starts no game or renderer.
    func startAutomaticallyIfNeeded() async {
        guard !GameFolder.hasGameFiles, !automaticAttempted, state == .idle, !busy else { return }
        await startAutomatic()
    }

    func startAutomatic() async {
        guard !busy else { return }
        automaticAttempted = true
        resourcesOnly = true
        resolvingAutomatic = true
        attempt = UUID()
        startedAt = ProcessInfo.processInfo.systemUptime
        let generation = attempt
        state = .downloading(received: 0, total: 0)
        trace("source_lookup_start")
        let answer = await automaticSource()
        guard generation == attempt else { return }
        trace("source_lookup_result has_url=\(answer["url"] is String) has_error=\(answer.error != nil)")
        resolvingAutomatic = false
        guard !Task.isCancelled else { trace("source_lookup_cancelled"); state = .idle; return }
        // A cable/manual import can finish while the source request is pending.
        guard !GameFolder.hasGameFiles else { trace("already_installed"); state = .done("Game files are already installed."); return }
        guard let text = answer["url"] as? String, let url = URL(string: text),
              ["https", "http"].contains(url.scheme?.lowercased() ?? ""), url.host != nil else {
            trace("source_lookup_failed detail=\(Diagnostics.redact(answer.error ?? "missing_or_invalid_url"))")
            state = .failed(answer.error ?? "The automatic download is unavailable. Retry or use a download link.")
            return
        }
        // A custom link's token belongs only to that link, never the public source.
        begin(url: url, token: nil)
    }

    /// Import through the same resource-only validator as automatic setup.
    func importPreparedFile(_ url: URL) async {
        guard !busy else { return }
        resourcesOnly = false
        attempt = UUID()
        startedAt = ProcessInfo.processInfo.systemUptime
        trace("file_import_start")
        traceStorage()
        let generation = attempt
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        let local = Self.destination
        defer { try? FileManager.default.removeItem(at: local) }
        state = .installing
        do {
            // Files providers may need to hydrate a large ZIP. Keep that I/O off
            // the main actor, and reject oversized files before duplicating them.
            try await Task.detached(priority: .utility) {
                var source = url
                source.removeAllCachedResourceValues()
                let values = try source.resourceValues(forKeys: [.fileSizeKey, .isRegularFileKey])
                guard values.isRegularFile == true,
                      let size = values.fileSize, Int64(size) <= Self.maximumArchiveBytes else {
                    throw CocoaError(.fileReadTooLarge)
                }
                try FileManager.default.copyItem(at: url, to: local)
            }.value
            guard generation == attempt, !Task.isCancelled else { return }
            let answer = await importArchive("import-game-files \(local.path)")
            guard generation == attempt else { return }
            if let error = answer.error { trace("file_import_failed detail=\(Diagnostics.redact(error))"); state = .failed(error); Haptics.failed() }
            else { trace("file_import_complete files_ready=\(GameFolder.hasGameFiles)"); state = .done(Self.describe(answer)); Haptics.done() }
        } catch {
            guard generation == attempt, !Task.isCancelled else { return }
            trace("file_import_failed \(Self.errorFacts(error))")
            state = .failed(error.localizedDescription)
            Haptics.failed()
        }
    }

    private func begin(url: URL, token: String?) {
        receivedBytes = 0
        expectedBytes = 0
        lastProgressAt = ProcessInfo.processInfo.systemUptime
        // Only the shipped public endpoint gets a name. Custom paths/queries stay private.
        let endpoint = url.host?.lowercased() == "cdn.gayfeet.lol" ? "default-cdn" : "custom"
        trace("transfer_start endpoint=\(endpoint) secure=\(url.scheme?.lowercased() == "https")")
        traceStorage()
        var request = URLRequest(url: url)
        if let token { request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization") }
        // A prepared tree is ~600 MB: ask for the time to finish if the player looks away.
        background = UIApplication.shared.beginBackgroundTask(withName: "Horizon game files") { [weak self] in
            let download = self
            Task { @MainActor in download?.trace("background_time_expired"); download?.endBackground() }
        }
        let session = URLSession(configuration: .default, delegate: self, delegateQueue: nil)
        session.sessionDescription = String(attempt.uuidString.prefix(8))
        self.session = session
        state = .downloading(received: 0, total: 0)
        session.downloadTask(with: request).resume()
    }

    func cancel() {
        if busy { trace("cancel_requested received=\(receivedBytes) expected=\(expectedBytes)") }
        attempt = UUID()
        resolvingAutomatic = false
        session?.invalidateAndCancel()
        session = nil
        state = .idle
        endBackground()
    }

    private func endBackground() {
        if background != .invalid {
            UIApplication.shared.endBackgroundTask(background)
            background = .invalid
        }
    }

    private func finished(_ outcome: Result<URL, Error>, from source: URLSession) async {
        guard session === source else {
            if case let .success(file) = outcome { try? FileManager.default.removeItem(at: file) }
            return
        }
        let generation = attempt
        session?.finishTasksAndInvalidate()
        session = nil
        switch outcome {
        case let .failure(error):
            trace("transfer_failed \(Self.errorFacts(error)) received=\(receivedBytes) expected=\(expectedBytes)")
            state = .failed(error.localizedDescription)
            Haptics.failed()
        case let .success(file):
            trace("transfer_complete received=\(receivedBytes) expected=\(expectedBytes)")
            defer { try? FileManager.default.removeItem(at: file) }
            // A manual/cable import can also finish after the HTTP transfer began.
            guard !resourcesOnly || !GameFolder.hasGameFiles else {
                state = .done("Game files are already installed.")
                break
            }
            state = .installing
            trace("import_start")
            traceStorage()
            let command = resourcesOnly ? "import-game-files" : "import"
            let answer = await importArchive("\(command) \(file.path)")
            guard generation == attempt else { return }
            if let error = answer.error {
                trace("import_failed detail=\(Diagnostics.redact(error))")
                state = .failed(error)
                Haptics.failed()
            } else {
                trace("import_complete files_ready=\(GameFolder.hasGameFiles)")
                state = .done(Self.describe(answer))
                Haptics.done()
            }
        }
        endBackground()
    }

    private static func describe(_ answer: [String: Any]) -> String {
        let name = answer["name"] as? String ?? ""
        switch answer["kind"] as? String {
        case "resources":
            return GameFolder.hasGameFiles
                ? "Game files installed. If the game was already running, close Erkam and open it again to use them."
                : "Game files installed."
        case "world": return "World \(name) added."
        case "resource_pack": return "Resource pack \(name) added."
        default: return "Installed."
        }
    }
}

extension GameFilesDownload: URLSessionDownloadDelegate {
    nonisolated func urlSession(_ session: URLSession, task: URLSessionTask, didFinishCollecting metrics: URLSessionTaskMetrics) {
        let id = session.sessionDescription ?? "unknown"
        func ms(_ start: Date?, _ end: Date?) -> Int {
            guard let start, let end else { return -1 }
            return Int(max(0, end.timeIntervalSince(start)) * 1000)
        }
        let last = metrics.transactionMetrics.last
        let status = (last?.response as? HTTPURLResponse)?.statusCode ?? 0
        let facts = "Game files [\(id)] network_metrics redirects=\(metrics.redirectCount) transactions=\(metrics.transactionMetrics.count) http_status=\(status) dns_ms=\(ms(last?.domainLookupStartDate,last?.domainLookupEndDate)) connect_ms=\(ms(last?.connectStartDate,last?.connectEndDate)) tls_ms=\(ms(last?.secureConnectionStartDate,last?.secureConnectionEndDate)) response_ms=\(ms(last?.responseStartDate,last?.responseEndDate))"
        // Metrics may arrive after the terminal callback. Keep their original attempt ID.
        Task { @MainActor in self.diagnostic(facts) }
    }

    nonisolated func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didWriteData _: Int64,
                                totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64) {
        let oversized = totalBytesWritten > Self.maximumArchiveBytes || totalBytesExpectedToWrite > Self.maximumArchiveBytes
        if oversized { downloadTask.cancel() }
        Task { @MainActor in
            guard self.session === session else { return }
            self.receivedBytes = totalBytesWritten
            self.expectedBytes = max(totalBytesExpectedToWrite, 0)
            let now = ProcessInfo.processInfo.systemUptime
            if oversized || now - self.lastProgressAt >= 15 {
                self.lastProgressAt = now
                self.trace("transfer_progress received=\(totalBytesWritten) expected=\(max(totalBytesExpectedToWrite, 0)) oversized=\(oversized)")
            }
            if oversized {
                self.cancel()
                self.state = .failed("The archive exceeds the 2 GiB game-file limit.")
                return
            }
            self.state = .downloading(received: totalBytesWritten, total: max(totalBytesExpectedToWrite, 0))
        }
    }

    nonisolated func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask,
                                didFinishDownloadingTo location: URL) {
        // The file at `location` is gone once this returns: move it now, on this queue.
        let outcome: Result<URL, Error>
        let status = (downloadTask.response as? HTTPURLResponse)?.statusCode ?? 200
        if downloadTask.countOfBytesReceived > Self.maximumArchiveBytes || downloadTask.countOfBytesExpectedToReceive > Self.maximumArchiveBytes {
            outcome = .failure(DownloadError("The archive exceeds the 2 GiB game-file limit."))
        } else if !(200..<300).contains(status) {
            let reason = status == 401 || status == 403
                ? "The download server refused the request (\(status))."
                : "The server answered \(status) \(HTTPURLResponse.localizedString(forStatusCode: status))."
            outcome = .failure(DownloadError(reason))
        } else {
            do {
                let destination = Self.destination
                try? FileManager.default.removeItem(at: destination)
                try FileManager.default.moveItem(at: location, to: destination)
                outcome = .success(destination)
            } catch {
                outcome = .failure(error)
            }
        }
        let received = downloadTask.countOfBytesReceived
        let expected = max(downloadTask.countOfBytesExpectedToReceive, 0)
        Task { @MainActor in
            guard self.session === session else {
                if case let .success(file) = outcome { try? FileManager.default.removeItem(at: file) }
                return
            }
            self.receivedBytes = received
            self.expectedBytes = expected
            self.trace("http_response status=\(status) received=\(received) expected=\(expected)")
            await self.finished(outcome, from: session)
        }
    }

    nonisolated func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        guard let error else { return }
        let failure: Error = task.countOfBytesReceived > Self.maximumArchiveBytes || task.countOfBytesExpectedToReceive > Self.maximumArchiveBytes
            ? DownloadError("The archive exceeds the 2 GiB game-file limit.") : error
        Task { @MainActor in
            guard self.session === session else { return }
            self.receivedBytes = task.countOfBytesReceived
            self.expectedBytes = max(task.countOfBytesExpectedToReceive, 0)
            await self.finished(.failure(failure), from: session)
        }
    }
}

private struct DownloadError: LocalizedError {
    let errorDescription: String?
    init(_ reason: String) { errorDescription = reason }
}

/// The access token, kept in the keychain rather than in preferences.
private enum Keychain {
    private static let query: [String: Any] = [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: "dev.horizon.launcher",
        kSecAttrAccount as String: "game-files-token",
    ]

    static func read() -> String? {
        var item: CFTypeRef?
        var search = query
        search[kSecReturnData as String] = true
        guard SecItemCopyMatching(search as CFDictionary, &item) == errSecSuccess, let data = item as? Data else {
            return nil
        }
        return String(data: data, encoding: .utf8)
    }

    static func write(_ token: String?) throws {
        guard let token else {
            let status = SecItemDelete(query as CFDictionary)
            guard status == errSecSuccess || status == errSecItemNotFound else { throw storageError() }
            return
        }
        // Never remove a working credential before its replacement is saved.
        let values: [String: Any] = [kSecValueData as String: Data(token.utf8),
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlock]
        var status = SecItemUpdate(query as CFDictionary, values as CFDictionary)
        if status == errSecItemNotFound {
            var item = query
            values.forEach { item[$0.key] = $0.value }
            status = SecItemAdd(item as CFDictionary, nil)
        }
        guard status == errSecSuccess else { throw storageError() }
    }

    private static func storageError() -> DownloadError {
        DownloadError("The download credential could not be updated securely. Unlock this device and try again.")
    }
}

// MARK: - The card

/// Get the game files: the link, the token, the progress and what happened.
struct GameFilesCard: View {
    @ObservedObject var download: GameFilesDownload
    /// The game has none yet: say so first.
    var missing: Bool
    var setupHelp: (() -> Void)? = nil

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 8) {
                Image(systemName: missing ? "exclamationmark.triangle.fill" : "arrow.down.circle.fill")
                    .foregroundStyle(missing ? Theme.warn : Theme.accentText)
                Text(missing ? "Game files needed" : "Download game files")
                    .font(Theme.pixel(18)).foregroundStyle(Theme.text)
            }
            if missing {
                Text("Erkam downloads and installs the game files automatically. Keep this screen open until setup finishes.")
                    .font(Theme.sans(12)).foregroundStyle(Theme.textMuted)
            }
            progress
            if missing, let setupHelp {
                Button("Other ways to get game files", action: setupHelp)
                    .accessibilityIdentifier("launcher-game-files-help")
            }

            if download.busy {
                SecondaryButton(title: "Cancel", action: download.cancel)
                    .disabled(download.state == .installing)
            } else {
                if missing {
                    PrimaryButton(title: "Download game files", icon: "arrow.down") {
                        Task { await download.startAutomatic() }
                    }
                    .accessibilityIdentifier("launcher-download-game-files")
                }
            }
            DisclosureGroup(missing ? "Use a download link instead" : "Download from a link") {
                Text("""
                Erkam plays with the files your own Minecraft: Java Edition makes. Prepare them on a computer \
                (horizon-prepare, then pack them as a zip), put the zip somewhere this phone can reach, such as your \
                computer on this Wi-Fi or a server of your own, and paste its link. Worlds and resource packs \
                (.zip) work too.
                """)
                .font(Theme.sans(12)).foregroundStyle(Theme.textMuted)
                .fixedSize(horizontal: false, vertical: true)

                Field(title: "LINK", placeholder: "https://example.com/horizon-26.3.zip", text: $download.link,
                      secure: false)
                Field(title: "ACCESS TOKEN (OPTIONAL)", placeholder: "Sent as a Bearer token", text: $download.token,
                      secure: true)

                PrimaryButton(title: "Download from link", icon: "arrow.down", action: download.start)
                    .disabled(download.busy || download.link.trimmingCharacters(in: .whitespaces).isEmpty)
            }
            Text("You can also send the zip from the game's own screen over Wi-Fi, or drop it into Erkam's folder in the Files app.")
                .font(Theme.sans(11)).foregroundStyle(Theme.textFaint)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(16)
        .background(Theme.surface, in: RoundedRectangle(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(missing ? Theme.noticeBorder : Theme.border))
    }

    @ViewBuilder private var progress: some View {
        switch download.state {
        case .idle:
            EmptyView()
        case let .downloading(received, total):
            VStack(alignment: .leading, spacing: 6) {
                if total > 0 {
                    ProgressView(value: Double(received), total: Double(total)).tint(Theme.accent)
                } else {
                    ProgressView().tint(Theme.accent)
                }
                Text(total > 0
                    ? "\(Self.size(received)) of \(Self.size(total))"
                    : "\(Self.size(received)) downloaded")
                    .font(Theme.mono(10)).foregroundStyle(Theme.textMuted)
            }
        case .installing:
            HStack(spacing: 8) {
                ProgressView().tint(Theme.accent)
                Text("Unpacking…").font(Theme.mono(10)).foregroundStyle(Theme.textMuted)
            }
        case let .done(message):
            Label(message, systemImage: "checkmark.circle.fill")
                .font(Theme.sans(12)).foregroundStyle(Theme.ready)
        case let .failed(message):
            Label(message, systemImage: "xmark.octagon.fill")
                .font(Theme.sans(12)).foregroundStyle(Theme.danger)
        }
    }

    private static func size(_ bytes: Int64) -> String {
        ByteCountFormatter.string(fromByteCount: bytes, countStyle: .file)
    }
}


/// Recovery appears on an explicit play/setup request, never over background setup.
struct GameFilesRecovery: View {
    @ObservedObject var download: GameFilesDownload
    let version: String
    let signedIn: Bool
    var configureManually: (() -> Void)? = nil
    @State private var choosingFile = false
    @State private var fileIssue: String?
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    if case let .failed(message) = download.state {
                        Text(download.automaticFailure != nil ? "Automatic file download failed." : "Game-file setup failed.")
                            .font(.headline)
                        Text(message).foregroundStyle(.secondary).textSelection(.enabled)
                        Text("You can retry, or go to Settings → Game files to configure your own files manually.")
                    } else if download.busy {
                        Text("Game files are being set up. You can play when installation finishes.")
                    } else {
                        Text("Erkam needs Minecraft game files before you can play, open a world or join a server.")
                    }
                    Text("You can retry automatic setup below, or prepare and transfer the files from a computer.")
                }
                Section("Download from Mojang with Microsoft") {
                    Text(signedIn
                         ? "Your Microsoft account is signed in here. To prepare game files, also sign in to the Erkam desktop launcher with an account that owns Minecraft: Java Edition."
                         : "On a computer, sign in to the Erkam desktop launcher with a Microsoft account that owns Minecraft: Java Edition.")
                    Text("Choose Download and prepare for Minecraft \(version). The desktop launcher downloads from Mojang and prepares the files for Erkam.")
                    Text("Signing in on this iPad alone does not download or prepare these files yet. Transfer the prepared files using the steps below.")
                        .foregroundStyle(.secondary)
                    Link("Get the Erkam desktop launcher", destination: URL(string: "https://github.com/horizon-rust/horizon/releases")!)
                }
                Section("Prepare or transfer your own files") {
                    if let configureManually {
                        Button("Open Settings → Game files", action: configureManually)
                            .accessibilityIdentifier("launcher-game-files-open-settings")
                    }
                    Text("If Minecraft \(version) is already installed on your computer, use the desktop launcher’s From my install option to prepare it instead.")
                    Text("Export the prepared folder with the desktop game executable:")
                    Text("horizon-client --pack-resources <prepared-folder> --out Horizon-\(version).zip")
                        .font(.footnote.monospaced()).textSelection(.enabled)
                    Text("Replace <prepared-folder> with the folder created by preparation. A Minecraft JAR, world ZIP or resource pack alone cannot replace these files.")
                    Text("Copy the prepared ZIP into Erkam’s Documents folder using Finder or Apple Devices file sharing. You can also put it at a link this iPad can reach and use Download from link below.")
                    Button("Choose prepared ZIP") { choosingFile = true }
                        .disabled(download.busy)
                        .accessibilityIdentifier("launcher-import-prepared-files")
                    Text("Choose the ZIP from Files after transferring it. When installation finishes, return to Play. Your worlds and sign-in are kept.")
                    Link("Read game-file transfer instructions", destination: URL(string: "https://github.com/heyskylo/horizon-ios/blob/main/docs/native-client.md#carry-your-game-files-to-a-phone")!)
                }
                Section("Automatic setup or download link") {
                    GameFilesCard(download: download, missing: true)
                }
            }
            .navigationTitle("Game files needed")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
        .fileImporter(isPresented: $choosingFile, allowedContentTypes: [.zip]) { result in
            switch result {
            case let .success(url): Task { await download.importPreparedFile(url) }
            case let .failure(error): fileIssue = error.localizedDescription
            }
        }
        .alert("Couldn’t open game files", isPresented: Binding(
            get: { fileIssue != nil }, set: { if !$0 { fileIssue = nil } }
        )) { Button("OK", role: .cancel) {} } message: { Text(fileIssue ?? "") }
        .accessibilityIdentifier("launcher-game-files-recovery")
    }
}
