//
//  MailService.swift
//  mailbrain
//
//  Created on 7/31/25.
//

import Foundation
import AppKit

// Structure to represent an email
struct Email: Identifiable, Codable {
    var id = UUID()
    var subject: String
    var sender: String
    var body: String
    var receivedDate: Date
    
    // Additional properties that might be useful
    var isRead: Bool
    var hasAttachments: Bool
    
    // For JSON encoding/decoding to match backend API
    enum CodingKeys: String, CodingKey {
        case subject, sender, body
        case receivedDate = "date"
    }
}

// Structure for API payload to backend
struct EmailDataPayload: Codable {
    let subject: String
    let sender: String
    let body: String
    let date: String
    let recipients: String?
    let thread_id: String?
}

class MailService {
    // Singleton instance
    static let shared = MailService()
    
    private init() {}
    
    // Retrieve the first email from Mail.app
    func getFirstEmail() -> Email? {
        // AppleScript to get the first email from Mail.app
        let appleScript = """
        tell application "Mail"
            if (count of messages of inbox) > 0 then
                set theMessage to message 1 of inbox
                set theSubject to subject of theMessage
                set theSender to sender of theMessage
                set theContent to content of theMessage
                set theDate to date received of theMessage
                set isRead to read status of theMessage
                set hasAttach to (count of mail attachments of theMessage) > 0
                
                return {theSubject, theSender, theContent, theDate, isRead, hasAttach}
            else
                return {"No messages", "", "", current date, false, false}
            end if
        end tell
        """
        
        // Create and execute the AppleScript
        let script = NSAppleScript(source: appleScript)
        var errorDict: NSDictionary?
        guard let result = script?.executeAndReturnError(&errorDict) else {
            if let error = errorDict {
                print("Error executing AppleScript: \(error)")
            }
            return nil
        }
        
        // Process the result
        if result.numberOfItems >= 6 {
            let subject = result.atIndex(1)?.stringValue ?? "No Subject"
            let sender = result.atIndex(2)?.stringValue ?? "Unknown Sender"
            let body = result.atIndex(3)?.stringValue ?? ""
            let dateReceived = result.atIndex(4)?.dateValue ?? Date()
            let isRead = result.atIndex(5)?.booleanValue ?? false
            let hasAttachments = result.atIndex(6)?.booleanValue ?? false
            
            return Email(
                subject: subject,
                sender: sender,
                body: body,
                receivedDate: dateReceived,
                isRead: isRead,
                hasAttachments: hasAttachments
            )
        }
        
        return nil
    }
    
    // Method to send email data to the backend
    func sendEmailToBackend(email: Email) {
        // Backend URL
        guard let url = URL(string: "http://localhost:8000/api/v1/emails") else {
            print("Error: Invalid backend URL")
            return
        }
        
        // Format date as ISO string
        let dateFormatter = ISO8601DateFormatter()
        let dateString = dateFormatter.string(from: email.receivedDate)
        
        // Create payload matching backend API schema
        let payload = EmailDataPayload(
            subject: email.subject,
            sender: email.sender,
            body: email.body,
            date: dateString,
            recipients: nil,
            thread_id: nil
        )
        
        // Encode to JSON
        guard let jsonData = try? JSONEncoder().encode(payload) else {
            print("Error: Failed to encode email data to JSON")
            return
        }
        
        // Create request
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = jsonData
        
        // Send request
        let task = URLSession.shared.dataTask(with: request) { data, response, error in
            if let error = error {
                print("Error sending email to backend: \(error.localizedDescription)")
                return
            }
            
            if let httpResponse = response as? HTTPURLResponse {
                print("Backend response status: \(httpResponse.statusCode)")
                
                if httpResponse.statusCode == 200 {
                    print("✅ Email successfully sent to backend")
                    if let data = data, let responseString = String(data: data, encoding: .utf8) {
                        print("Response: \(responseString)")
                    }
                } else {
                    print("❌ Failed to send email to backend. Status: \(httpResponse.statusCode)")
                    if let data = data, let responseString = String(data: data, encoding: .utf8) {
                        print("Error response: \(responseString)")
                    }
                }
            }
        }
        
        task.resume()
        
        print("=== SENDING EMAIL DATA TO BACKEND ===")
        print("Subject: \(email.subject)")
        print("From: \(email.sender)")
        print("Date: \(email.receivedDate)")
        print("Body: \(email.body.prefix(100))...")
        print("=====================================")
    }
}