//
//  ContentView.swift
//  mailbrain
//
//  Created by blue on 7/17/25.
//

import SwiftUI

struct ContentView: View {
    @State private var email: Email?
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var showSuccess = false
    @State private var successMessage = "Operation completed successfully."
    
    var body: some View {
        VStack(spacing: 20) {
            Text("Mail.app Integration")
                .font(.largeTitle)
                .fontWeight(.bold)
            
            if isLoading {
                ProgressView("Processing...")
            } else if let error = errorMessage {
                Text("Error: \(error)")
                    .foregroundColor(.red)
                    .padding()
                    .multilineTextAlignment(.center)
            } else if let email = email {
                // Email details view
                VStack(alignment: .leading, spacing: 12) {
                    Text("Subject: \(email.subject)")
                        .font(.headline)
                    
                    Text("From: \(email.sender)")
                        .font(.subheadline)
                    
                    Text("Date: \(formattedDate(email.receivedDate))")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                    
                    Divider()
                    
                    Text("Body:")
                        .font(.headline)
                    
                    ScrollView {
                        Text(email.body)
                            .font(.body)
                            .padding(.vertical, 4)
                    }
                    .frame(height: 200)
                    .background(Color.gray.opacity(0.1))
                    .cornerRadius(8)
                    
                    HStack {
                        Image(systemName: email.isRead ? "envelope.open" : "envelope")
                        Text(email.isRead ? "Read" : "Unread")
                        
                        Spacer()
                        
                        if email.hasAttachments {
                            Image(systemName: "paperclip")
                            Text("Has attachments")
                        }
                    }
                    .foregroundColor(.secondary)
                    
                    Button(action: {
                        sendEmailToBackend()
                    }) {
                        HStack {
                            Image(systemName: "arrow.up.doc")
                            Text("Send to Backend")
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .padding(.top, 8)
                }
                .padding()
                .background(Color.white)
                .cornerRadius(12)
                .shadow(radius: 2)
                .padding()
            } else {
                Text("No email data available")
                    .foregroundColor(.secondary)
                    .padding()
            }
            
            HStack(spacing: 15) {
                Button(action: {
                    fetchFirstEmail()
                }) {
                    HStack {
                        Image(systemName: "envelope.badge")
                        Text("Fetch Selected Email")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
                .disabled(isLoading)
                
                Button(action: {
                    extractEmailForLoRA()
                }) {
                    HStack {
                        Image(systemName: "brain")
                        Text("Extract for LoRA")
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(isLoading || email == nil)
            }
            .padding(.horizontal)
        }
        .padding()
        .alert("Success", isPresented: $showSuccess) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(successMessage)
        }
    }
    
    // Format date for display
    private func formattedDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
    
    // Fetch the first email from Mail.app
    private func fetchFirstEmail() {
        isLoading = true
        errorMessage = nil
        
        // Use background thread for potentially slow operation
        DispatchQueue.global(qos: .userInitiated).async {
            let result = MailService.shared.getFirstEmail()
            
            // Update UI on main thread
            DispatchQueue.main.async {
                isLoading = false
                
                if let fetchedEmail = result {
                    email = fetchedEmail
                } else {
                    errorMessage = "No selected email was available from Mail.app. Make sure Mail is running, a message is selected, and permissions are granted."
                }
            }
        }
    }
    
    // Send email data to backend
    private func sendEmailToBackend() {
        guard let email = email else { return }

        isLoading = true
        errorMessage = nil

        MailService.shared.sendEmailToBackend(email: email) { result in
            DispatchQueue.main.async {
                isLoading = false

                switch result {
                case .success:
                    successMessage = "Email data sent to backend successfully."
                    showSuccess = true
                case .failure(let error):
                    errorMessage = error.localizedDescription
                }
            }
        }
    }
    
    /// Extracts the current email data for LoRA adapter training
    private func extractEmailForLoRA() {
        guard let email = email else { return }
        
        isLoading = true
        errorMessage = nil
        
        // Use background thread for potentially slow operation
        DispatchQueue.global(qos: .userInitiated).async {
            let filePath = EmailExtractor.shared.extractAndSaveEmailData(from: email)
            
            // Update UI on main thread
            DispatchQueue.main.async {
                isLoading = false
                
                if filePath != nil {
                    successMessage = "Email data extracted for LoRA training."
                    showSuccess = true
                } else {
                    errorMessage = "Failed to extract email data for LoRA training."
                }
            }
        }
    }
}

#Preview {
    ContentView()
}
