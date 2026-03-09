# System Architecture Sequence Diagram

This document describes the current request flow through EmailBrain's primary chat path and the optional JKCA fallback path.

## Adapter-Capable Chat Path

```mermaid
sequenceDiagram
    participant User
    participant Web Frontend
    participant Rust Backend
    participant Database
    participant LM Studio

    User->>Web Frontend: Select adapter and submit prompt
    Web Frontend->>Rust Backend: POST /api/v1/chat
    Rust Backend->>Database: Resolve adapter metadata
    Database-->>Rust Backend: Adapter path
    Rust Backend->>LM Studio: POST /v1/chat/completions with LoRA adapter
    LM Studio-->>Rust Backend: Generated text
    Rust Backend->>Database: Store chat log
    Rust Backend-->>Web Frontend: Chat completion
    Web Frontend-->>User: Display adapted response
```

## Base-Chat Fallback Path

```mermaid
sequenceDiagram
    participant User
    participant Web Frontend
    participant Rust Backend
    participant JKCA

    User->>Web Frontend: Submit prompt without adapter
    Web Frontend->>Rust Backend: POST /api/v1/chat
    Rust Backend->>JKCA: POST /api/runtime/generate
    JKCA-->>Rust Backend: Generated text
    Rust Backend-->>Web Frontend: Normalized chat completion
```

## Notes

- Adapter-backed requests are routed through LM Studio.
- JKCA is only used for base-model chat when configured as the fallback provider.
- All chat traffic is normalized by the Rust backend before the frontend consumes it.
