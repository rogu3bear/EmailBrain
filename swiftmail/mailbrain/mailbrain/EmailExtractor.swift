//
//  EmailExtractor.swift
//  mailbrain
//
//  Created on 7/31/25.
//

import Foundation

/// EmailExtractor is responsible for retrieving email data and formatting it
/// for use in training LoRA adapters.
final class EmailExtractor {
    static let shared = EmailExtractor()

    private init() {}

    func extractAndSaveEmailData(from email: Email) -> String? {
        let formattedData = formatEmailForLoRA(email)
        return saveToJSON(formattedData)
    }

    private func formatEmailForLoRA(_ email: Email) -> [String: Any] {
        let dateFormatter = DateFormatter()
        dateFormatter.locale = Locale(identifier: "en_US_POSIX")
        dateFormatter.timeZone = TimeZone(secondsFromGMT: 0)
        dateFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss'Z'"

        let formattedBody = email.body
            .replacingOccurrences(of: "\r\n", with: "\n")
            .replacingOccurrences(of: "\r", with: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        let summaryBody = formattedBody.isEmpty ? "No body content provided." : formattedBody

        let trainingExamples = [
            [
                "instruction": "Complete the email subject line:",
                "input": String(email.subject.prefix(max(1, email.subject.count / 2))),
                "output": email.subject
            ],
            [
                "instruction": "Who sent this email?",
                "input": "Subject: \(email.subject)",
                "output": email.sender
            ],
            [
                "instruction": "Summarize this email:",
                "input": summaryBody,
                "output": "This is an email from \(email.sender) about \(email.subject)."
            ],
            [
                "instruction": "Generate a response to this email:",
                "input": "From: \(email.sender)\nSubject: \(email.subject)\n\n\(summaryBody)",
                "output": "Thank you for your email regarding \(email.subject). I have received your message and will respond shortly."
            ]
        ]

        let metadata: [String: Any] = [
            "source": "Mail.app",
            "sender": email.sender,
            "subject": email.subject,
            "date": dateFormatter.string(from: email.receivedDate),
            "isRead": email.isRead,
            "hasAttachments": email.hasAttachments,
            "recipients": email.recipients as Any,
            "threadId": email.threadId as Any
        ]
        let emailData: [String: Any] = [
            "subject": email.subject,
            "sender": email.sender,
            "body": formattedBody
        ]
        let contextPairs = trainingExamples.map { example in
            [
                "input": example["input"] ?? "",
                "output": example["output"] ?? ""
            ]
        }

        return [
            "metadata": metadata,
            "content": formattedBody,
            "training_examples": trainingExamples,
            "emailData": emailData,
            "trainingData": [
                "contextPairs": contextPairs
            ]
        ]
    }

    private func saveToJSON(_ data: [String: Any]) -> String? {
        do {
            let jsonData = try JSONSerialization.data(withJSONObject: data, options: [.prettyPrinted, .sortedKeys])
            let fileManager = FileManager.default
            let directoryURL = try preferredDataDirectory(using: fileManager)
            if !fileManager.fileExists(atPath: directoryURL.path) {
                try fileManager.createDirectory(at: directoryURL, withIntermediateDirectories: true)
            }

            let filename = "email_data_\(Int(Date().timeIntervalSince1970 * 1000))_\(UUID().uuidString.prefix(8)).json"
            let fileURL = directoryURL.appendingPathComponent(filename)
            try jsonData.write(to: fileURL, options: .atomic)
            return fileURL.path
        } catch {
            return nil
        }
    }

    private func preferredDataDirectory(using fileManager: FileManager) throws -> URL {
        let sourceURL = URL(fileURLWithPath: #filePath)
        let repoRootURL = sourceURL
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let repoDataDirectory = repoRootURL
            .appendingPathComponent("adapters")
            .appendingPathComponent("data")

        if fileManager.fileExists(atPath: repoRootURL.path) {
            return repoDataDirectory
        }

        guard let applicationSupportDirectory = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else {
            throw NSError(domain: "EmailExtractor", code: -1, userInfo: [
                NSLocalizedDescriptionKey: "Unable to resolve an output directory for extracted email data"
            ])
        }

        return applicationSupportDirectory
            .appendingPathComponent("EmailBrain")
            .appendingPathComponent("adapters")
            .appendingPathComponent("data")
    }
}
