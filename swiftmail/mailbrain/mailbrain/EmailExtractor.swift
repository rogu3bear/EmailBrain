//
//  EmailExtractor.swift
//  mailbrain
//
//  Created on 7/31/25.
//

import Foundation

/// EmailExtractor is responsible for retrieving email data and formatting it
/// for use in training LoRA adapters.
class EmailExtractor {
    // Singleton instance
    static let shared = EmailExtractor()
    
    private init() {}
    
    /// Extracts information from the most recent email and saves it to a JSON file
    /// - Parameter email: The email selected in the Swift UI
    /// - Returns: The path to the saved JSON file, or nil if extraction failed
    func extractAndSaveEmailData(from email: Email) -> String? {
        // Step 1: Format the email content for LoRA adapter training
        let formattedData = formatEmailForLoRA(email)
        
        // Step 2: Save the formatted data to a JSON file
        return saveToJSON(formattedData)
    }
    
    /// Formats an email for LoRA adapter training
    /// - Parameter email: The email to format
    /// - Returns: A dictionary containing the formatted data
    private func formatEmailForLoRA(_ email: Email) -> [String: Any] {
        // Create a dictionary with metadata and content
        let dateFormatter = DateFormatter()
        dateFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss'Z'"
        
        // Format the email body for training
        // Remove excessive whitespace and normalize line breaks
        let formattedBody = email.body
            .replacingOccurrences(of: "\r\n", with: "\n")
            .replacingOccurrences(of: "\r", with: "\n")
            .components(separatedBy: .newlines)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
            .joined(separator: "\n")
        
        // Create training examples from the email content
        let trainingExamples = [
            // Example 1: Subject line completion
            [
                "instruction": "Complete the email subject line:",
                "input": email.subject.prefix(email.subject.count / 2),
                "output": email.subject
            ],
            // Example 2: Sender identification
            [
                "instruction": "Who sent this email?",
                "input": "Subject: \(email.subject)",
                "output": email.sender
            ],
            // Example 3: Email summarization
            [
                "instruction": "Summarize this email:",
                "input": formattedBody,
                "output": "This is an email from \(email.sender) about \(email.subject)."
            ],
            // Example 4: Email response generation
            [
                "instruction": "Generate a response to this email:",
                "input": "From: \(email.sender)\nSubject: \(email.subject)\n\n\(formattedBody)",
                "output": "Thank you for your email regarding \(email.subject). I have received your message and will respond shortly."
            ]
        ]
        
        let metadata: [String: Any] = [
            "source": "Mail.app",
            "sender": email.sender,
            "subject": email.subject,
            "date": dateFormatter.string(from: email.receivedDate),
            "isRead": email.isRead,
            "hasAttachments": email.hasAttachments
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
        
        // Keep one canonical top-level shape while preserving the legacy keys
        // expected by the existing Python tooling.
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
    
    /// Saves the formatted data to a JSON file
    /// - Parameter data: The data to save
    /// - Returns: The path to the saved file, or nil if saving failed
    private func saveToJSON(_ data: [String: Any]) -> String? {
        do {
            // Convert the data to JSON
            let jsonData = try JSONSerialization.data(withJSONObject: data, options: .prettyPrinted)
            
            // Create a unique filename based on the current timestamp
            let timestamp = Int(Date().timeIntervalSince1970)
            let filename = "email_data_\(timestamp).json"
            
            // Create the adapters/data directory if it doesn't exist
            let fileManager = FileManager.default
            let directoryURL = URL(fileURLWithPath: fileManager.currentDirectoryPath)
                .appendingPathComponent("adapters")
                .appendingPathComponent("data")
            
            if !fileManager.fileExists(atPath: directoryURL.path) {
                try fileManager.createDirectory(at: directoryURL, withIntermediateDirectories: true)
            }
            
            // Create the file path
            let fileURL = directoryURL.appendingPathComponent(filename)
            
            // Write the JSON data to the file
            try jsonData.write(to: fileURL)
            
            print("Email data saved to: \(fileURL.path)")
            return fileURL.path
        } catch {
            print("Error saving email data: \(error)")
            return nil
        }
    }
}
