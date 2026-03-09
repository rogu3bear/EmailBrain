//
//  MailService.swift
//  mailbrain
//
//  Created on 7/31/25.
//

import Foundation
import AppKit

struct Email: Identifiable {
    var id = UUID()
    var subject: String
    var sender: String
    var body: String
    var receivedDate: Date
    var recipients: String?
    var threadId: String?
    var isRead: Bool
    var hasAttachments: Bool
}

struct EmailDataPayload: Codable {
    let subject: String
    let sender: String
    let body: String
    let date: String
    let recipients: String?
    let thread_id: String?
}

final class MailService {
    static let shared = MailService()

    private let defaultBackendURL = "http://localhost:3901"

    private init() {}

    func getFirstEmail() -> Email? {
        let appleScript = """
        on joinList(theList, delimiterValue)
            set AppleScript's text item delimiters to delimiterValue
            set joinedValue to theList as string
            set AppleScript's text item delimiters to ""
            return joinedValue
        end joinList

        tell application "Mail"
            set theMessage to missing value

            if (exists message viewer 1) and ((count of selected messages of message viewer 1) > 0) then
                set theMessage to item 1 of (selected messages of message viewer 1)
            else if (count of messages of inbox) > 0 then
                set theMessage to message 1 of inbox
            end if

            if theMessage is missing value then
                return {}
            end if

            set theSubject to subject of theMessage
            set theSender to sender of theMessage
            set theContent to content of theMessage
            set theDate to date received of theMessage
            set isRead to read status of theMessage
            set hasAttach to (count of mail attachments of theMessage) > 0

            set recipientAddresses to {}
            try
                set recipientAddresses to address of every to recipient of theMessage
            end try

            set recipientsText to my joinList(recipientAddresses, ", ")

            set threadIdValue to ""
            try
                set threadIdValue to message id of theMessage as string
            end try

            return {theSubject, theSender, theContent, theDate, isRead, hasAttach, recipientsText, threadIdValue}
        end tell
        """

        let script = NSAppleScript(source: appleScript)
        var errorDict: NSDictionary?
        guard let result = script?.executeAndReturnError(&errorDict) else {
            return nil
        }

        guard result.numberOfItems >= 8 else {
            return nil
        }

        let subject = result.atIndex(1)?.stringValue?.trimmingCharacters(in: .whitespacesAndNewlines)
        let sender = result.atIndex(2)?.stringValue?.trimmingCharacters(in: .whitespacesAndNewlines)
        let body = result.atIndex(3)?.stringValue ?? ""
        let dateReceived = result.atIndex(4)?.dateValue ?? Date()
        let isRead = result.atIndex(5)?.booleanValue ?? false
        let hasAttachments = result.atIndex(6)?.booleanValue ?? false
        let recipients = normalizedOptionalString(result.atIndex(7)?.stringValue)
        let threadId = normalizedOptionalString(result.atIndex(8)?.stringValue)

        guard let subject, !subject.isEmpty, let sender, !sender.isEmpty else {
            return nil
        }

        return Email(
            subject: subject,
            sender: sender,
            body: body,
            receivedDate: dateReceived,
            recipients: recipients,
            threadId: threadId,
            isRead: isRead,
            hasAttachments: hasAttachments
        )
    }

    func sendEmailToBackend(email: Email, completion: @escaping (Result<Void, Error>) -> Void) {
        guard let url = backendEmailURL() else {
            completion(.failure(NSError(domain: "MailService", code: -1, userInfo: [
                NSLocalizedDescriptionKey: "Invalid backend URL"
            ])))
            return
        }

        let dateFormatter = ISO8601DateFormatter()
        dateFormatter.formatOptions = [.withInternetDateTime]
        let dateString = dateFormatter.string(from: email.receivedDate)

        let payload = EmailDataPayload(
            subject: email.subject,
            sender: email.sender,
            body: email.body,
            date: dateString,
            recipients: email.recipients,
            thread_id: email.threadId
        )

        guard let jsonData = try? JSONEncoder().encode(payload) else {
            completion(.failure(NSError(domain: "MailService", code: -1, userInfo: [
                NSLocalizedDescriptionKey: "Failed to encode email data to JSON"
            ])))
            return
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.timeoutInterval = 30
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = jsonData

        let task = URLSession.shared.dataTask(with: request) { data, response, error in
            if let error {
                completion(.failure(error))
                return
            }

            guard let httpResponse = response as? HTTPURLResponse else {
                completion(.failure(NSError(domain: "MailService", code: -1, userInfo: [
                    NSLocalizedDescriptionKey: "Backend response was not an HTTP response"
                ])))
                return
            }

            guard (200..<300).contains(httpResponse.statusCode) else {
                let message: String
                if let data, let responseString = String(data: data, encoding: .utf8), !responseString.isEmpty {
                    message = responseString
                } else {
                    message = "Backend returned status \(httpResponse.statusCode)"
                }

                completion(.failure(NSError(domain: "MailService", code: httpResponse.statusCode, userInfo: [
                    NSLocalizedDescriptionKey: message
                ])))
                return
            }

            completion(.success(()))
        }

        task.resume()
    }

    private func backendEmailURL() -> URL? {
        let configuredBaseURL =
            normalizedOptionalString(Bundle.main.object(forInfoDictionaryKey: "EMAILBRAIN_API_URL") as? String)
            ?? normalizedOptionalString(ProcessInfo.processInfo.environment["EMAILBRAIN_API_URL"])
            ?? defaultBackendURL

        let normalizedBaseURL = configuredBaseURL.hasSuffix("/api/v1/emails")
            ? configuredBaseURL
            : "\(configuredBaseURL.trimmingCharacters(in: CharacterSet(charactersIn: "/")))/api/v1/emails"

        return URL(string: normalizedBaseURL)
    }

    private func normalizedOptionalString(_ value: String?) -> String? {
        guard let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines), !trimmed.isEmpty else {
            return nil
        }
        return trimmed
    }
}
