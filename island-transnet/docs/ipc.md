# IPC Communication

This document describes the Inter-Process Communication (IPC) between Island OS and Transnet using FlatBuffers.

## Overview

Transnet communicates with Island OS through FlatBuffers messages over Unix Domain Sockets. This allows Anima (Island OS's AI) to request translations and access translation data.

## Protocol

- **Transport**: Unix Domain Socket
- **Serialization**: FlatBuffers
- **Schema File**: `flatbuffers/transnet_fb.fbs`
- **Socket Path**: `/tmp/transnet_ipc.sock`

## FlatBuffers Schema

### Schema File: `flatbuffers/transnet_fb.fbs`

```flatbuffers
namespace TransnetIPC;

// Request types
enum RequestType : byte {
    Translation = 0,
    WordAnalysis = 1,
    AuthVerify = 2,
    GetHistory = 3,
    GetFavorites = 4,
    HealthCheck = 5
}

// Response types
enum ResponseType : byte {
    Translation = 0,
    WordAnalysis = 1,
    AuthResult = 2,
    History = 3,
    Favorites = 4,
    Health = 5,
    Error = 6
}

// Error codes
enum ErrorCode : byte {
    Success = 0,
    InvalidRequest = 1,
    Unauthorized = 2,
    NotFound = 3,
    InternalError = 4,
    LlmError = 5,
    DbError = 6
}

// Part of speech
enum PartOfSpeech : byte {
    Unknown = 0,
    Noun = 1,
    Verb = 2,
    Adjective = 3,
    Adverb = 4,
    Pronoun = 5,
    Preposition = 6,
    Conjunction = 7,
    Article = 8,
    Interjection = 9
}

// Word relationship type
enum RelationType : byte {
    Synonym = 0,
    Antonym = 1,
    Hypernym = 2,
    Hyponym = 3,
    Related = 4
}

// Word meaning
table WordMeaning {
    word: string;
    pos: PartOfSpeech;
    meaning: string;
    translation: string;
    relations: [Relation];
}

// Word relationship
table Relation {
    word: string;
    type: RelationType;
    meaning: string;
    intensity: int = 0;  // 0-100 for adjectives
}

// Translation request
table TranslationRequest {
    request_id: string;
    user_id: string;
    text: string;
    source_lang: string;
    target_lang: string;
    input_type: string;  // "text", "sentence", "phrase", "word"
    include_related: bool = true;
}

// Translation part
table TranslationPart {
    original: string;
    translated: string;
    pos: PartOfSpeech;
    position: int;
    relations: [Relation];
}

// Translation response
table TranslationResponse {
    request_id: string;
    translation: string;
    input_type: string;
    parts: [TranslationPart];
    word_analysis: WordMeaning;
    cached: bool = false;
}

// Word analysis request
table WordAnalysisRequest {
    request_id: string;
    word: string;
    source_lang: string;
    target_lang: string;
}

// Word analysis response
table WordAnalysisResponse {
    request_id: string;
    word_analysis: WordMeaning;
    cached: bool = false;
}

// Auth verification request
table AuthVerifyRequest {
    request_id: string;
    token: string;
}

// User info
table UserInfo {
    user_id: string;
    username: string;
    email: string;
    created_at: string;
}

// Auth verification response
table AuthVerifyResponse {
    request_id: string;
    valid: bool;
    user: UserInfo;
}

// History entry
table HistoryEntry {
    id: string;
    source_text: string;
    source_lang: string;
    target_lang: string;
    translation: string;
    input_type: string;
    created_at: string;
}

// Get history request
table GetHistoryRequest {
    request_id: string;
    user_id: string;
    page: int = 1;
    limit: int = 20;
}

// Get history response
table HistoryResponse {
    request_id: string;
    entries: [HistoryEntry];
    total: int;
    page: int;
    limit: int;
}

// Favorite entry
table FavoriteEntry {
    id: string;
    word: string;
    pos: PartOfSpeech;
    meaning: string;
    translation: string;
    note: string;
    created_at: string;
}

// Get favorites request
table GetFavoritesRequest {
    request_id: string;
    user_id: string;
    page: int = 1;
    limit: int = 20;
}

// Get favorites response
table FavoritesResponse {
    request_id: string;
    entries: [FavoriteEntry];
    total: int;
    page: int;
    limit: int;
}

// Health check request
table HealthCheckRequest {
    request_id: string;
}

// Service status
table ServiceStatus {
    name: string;
    status: string;  // "healthy", "degraded", "down"
}

// Health check response
table HealthCheckResponse {
    request_id: string;
    healthy: bool;
    services: [ServiceStatus];
    version: string;
    timestamp: string;
}

// Error response
table ErrorResponse {
    request_id: string;
    error_code: ErrorCode;
    message: string;
    details: string;
}

// Root message type (union)
union Message {
    TranslationRequest,
    TranslationResponse,
    WordAnalysisRequest,
    WordAnalysisResponse,
    AuthVerifyRequest,
    AuthVerifyResponse,
    GetHistoryRequest,
    HistoryResponse,
    GetFavoritesRequest,
    FavoritesResponse,
    HealthCheckRequest,
    HealthCheckResponse,
    ErrorResponse
}

// Root table
table Root {
    request_id: string;
    timestamp: int;
    message: Message;
}

root_type Root;
```

## Message Flow

### 1. Translation Request (Anima → Transnet)

```
Anima                              Transnet
  │                                   │
  │  Root {                            │
  │    message: TranslationRequest    │
  │  }                                 │
  ├──────────────────────────────────►│
  │                                   │
  │                                   ├─ Classify input
  │                                   ├─ Check cache
  │                                   ├─ Call LLM
  │                                   ├─ Update Qdrant
  │                                   └─ Update SQLite
  │                                   │
  │  Root {                            │
  │    message: TranslationResponse    │
  │  }                                 │
  │◄──────────────────────────────────┤
  │                                   │
```

### 2. Word Analysis Request

```
Anima                              Transnet
  │                                   │
  │  Root {                            │
  │    message: WordAnalysisRequest    │
  │  }                                 │
  ├──────────────────────────────────►│
  │                                   │
  │                                   ├─ Look up in SQLite
  │                                   ├─ Get from Qdrant
  │                                   └─ Return word analysis
  │                                   │
  │  Root {                            │
  │    message: WordAnalysisResponse   │
  │  }                                 │
  │◄──────────────────────────────────┤
  │                                   │
```

### 3. Auth Verification

```
Island OS                          Transnet
  │                                   │
  │  Root {                            │
  │    message: AuthVerifyRequest     │
  │  }                                 │
  ├──────────────────────────────────►│
  │                                   │
  │                                   ├─ Validate JWT
  │                                   ├─ Check session
  │                                   └─ Return user info
  │                                   │
  │  Root {                            │
  │    message: AuthVerifyResponse     │
  │  }                                 │
  │◄──────────────────────────────────┤
  │                                   │
```

## Rust Implementation

### Transnet IPC Server

```rust
use flatbuffers::FlatBufferBuilder;
use tokio::net::UnixListener;
use transnet_ipc_generated::transnet_ipc_generated;

// IPC Server
pub struct IpcServer {
    listener: UnixListener,
    translation_service: Arc<TranslationService>,
}

impl IpcServer {
    pub async fn new(path: &str, translation_service: Arc<TranslationService>) 
        -> Result<Self> {
        let listener = UnixListener::bind(path)?;
        Ok(Self { listener, translation_service })
    }

    pub async fn run(&self) {
        loop {
            match self.listener.accept().await {
                Ok((mut socket, _)) => {
                    let service = self.translation_service.clone();
                    tokio::spawn(async move {
                        Self::handle_connection(&mut socket, service).await;
                    });
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        socket: &mut UnixStream,
        service: Arc<TranslationService>
    ) {
        // Read message
        let mut buf = vec![0u8; 4096];
        let n = socket.read(&mut buf).await?;
        
        // Parse FlatBuffers
        let root = transnet_ipc_generated::root_root(&buf[..n]);
        let request_id = root.request_id().unwrap();
        
        match root.message_type() {
            transnet_ipc_generated::Message::TranslationRequest => {
                let req = root.message_as_translation_request().unwrap();
                let result = service.translate(req).await;
                
                // Build response
                let mut builder = FlatBufferBuilder::new();
                let response = build_translation_response(&mut builder, request_id, result);
                builder.finish(response, None);
                
                // Send response
                socket.write_all(builder.finished_data()).await?;
            }
            // Handle other message types...
        }
    }
}

// Build translation response
fn build_translation_response(
    builder: &mut FlatBufferBuilder,
    request_id: &str,
    result: TranslationResult
) -> FlatBufferBuilder<'static> {
    // Implementation...
}
```

### Island OS IPC Client

```rust
use tokio::net::UnixStream;
use flatbuffers::FlatBufferBuilder;

pub struct TransnetClient {
    socket: UnixStream,
}

impl TransnetClient {
    pub async fn connect(path: &str) -> Result<Self> {
        let socket = UnixStream::connect(path).await?;
        Ok(Self { socket })
    }

    pub async fn translate(
        &mut self,
        user_id: &str,
        text: &str,
        source_lang: &str,
        target_lang: &str
    ) -> Result<TranslationResponse> {
        // Build request
        let mut builder = FlatBufferBuilder::new();
        let request_id = uuid::Uuid::new_v4().to_string();
        
        let text_offset = builder.create_string(text);
        let source_lang_offset = builder.create_string(source_lang);
        let target_lang_offset = builder.create_string(target_lang);
        let user_id_offset = builder.create_string(user_id);
        
        let req = transnet_ipc_generated::TranslationRequest::create(
            &mut builder,
            &transnet_ipc_generated::TranslationRequestArgs {
                request_id: Some(builder.create_string(&request_id)),
                user_id: Some(user_id_offset),
                text: Some(text_offset),
                source_lang: Some(source_lang_offset),
                target_lang: Some(target_lang_offset),
                input_type: Some(builder.create_string("auto")),
                include_related: Some(true),
                ..Default::default()
            }
        );
        
        let message = transnet_ipc_generated::Message::TranslationRequest;
        let root = transnet_ipc_generated::Root::create(
            &mut builder,
            &transnet_ipc_generated::RootArgs {
                request_id: Some(builder.create_string(&request_id)),
                timestamp: chrono::Utc::now().timestamp() as i64,
                message_type: message,
                message: Some(req.as_union_value()),
                ..Default::default()
            }
        );
        
        builder.finish(root, None);
        
        // Send request
        self.socket.write_all(builder.finished_data()).await?;
        
        // Read response
        let mut buf = vec![0u8; 8192];
        let n = self.socket.read(&mut buf).await?;
        
        // Parse response
        let root = transnet_ipc_generated::root_root(&buf[..n]);
        match root.message_type() {
            transnet_ipc_generated::Message::TranslationResponse => {
                let resp = root.message_as_translation_response().unwrap();
                Ok(parse_translation_response(resp))
            }
            transnet_ipc_generated::Message::ErrorResponse => {
                let err = root.message_as_error_response().unwrap();
                Err(anyhow::anyhow!("{}", err.message().unwrap()))
            }
            _ => Err(anyhow::anyhow!("Unexpected response type"))
        }
    }

    pub async fn word_analysis(
        &mut self,
        word: &str,
        source_lang: &str,
        target_lang: &str
    ) -> Result<WordAnalysis> {
        // Similar to translate...
    }
}
```

## Go Implementation

### Go Gateway IPC Client

```go
package main

import (
    "context"
    "flatbuffers/go"
    "net"
    "transnet_ipc_generated"
)

type TransnetClient struct {
    conn *net.UnixConn
}

func NewTransnetClient(path string) (*TransnetClient, error) {
    conn, err := net.DialUnix("unix", nil, &net.UnixAddr{Name: path, Net: "unix"})
    if err != nil {
        return nil, err
    }
    return &TransnetClient{conn: conn}, nil
}

func (c *TransnetClient) Translate(ctx context.Context, req *TranslationRequest) (*TranslationResponse, error) {
    builder := flatbuffers.NewBuilder(1024)
    
    requestId := builder.CreateString(uuid.New().String())
    userId := builder.CreateString(req.UserId)
    text := builder.CreateString(req.Text)
    sourceLang := builder.CreateString(req.SourceLang)
    targetLang := builder.CreateString(req.TargetLang)
    
    transnet_ipc_generated.TranslationRequestStart(builder)
    transnet_ipc_generated.TranslationRequestAddRequestId(builder, requestId)
    transnet_ipc_generated.TranslationRequestAddUserId(builder, userId)
    transnet_ipc_generated.TranslationRequestAddText(builder, text)
    transnet_ipc_generated.TranslationRequestAddSourceLang(builder, sourceLang)
    transnet_ipc_generated.TranslationRequestAddTargetLang(builder, targetLang)
    transnet_ipc_generated.TranslationRequestAddInputType(builder, builder.CreateString("auto"))
    transnet_ipc_generated.TranslationRequestAddIncludeRelated(builder, true)
    reqOffset := transnet_ipc_generated.TranslationRequestEnd(builder)
    
    transnet_ipc_generated.RootStart(builder)
    transnet_ipc_generated.RootAddRequestId(builder, requestId)
    transnet_ipc_generated.RootAddTimestamp(builder, time.Now().Unix())
    transnet_ipc_generated.RootAddMessageType(builder, transnet_ipc_generated.MessageTranslationRequest)
    transnet_ipc_generated.RootAddMessage(builder, reqOffset)
    rootOffset := transnet_ipc_generated.RootEnd(builder)
    
    builder.Finish(rootOffset)
    
    // Send request
    _, err := c.conn.Write(builder.FinishedBytes())
    if err != nil {
        return nil, err
    }
    
    // Read response
    buf := make([]byte, 8192)
    n, err := c.conn.Read(buf)
    if err != nil {
        return nil, err
    }
    
    // Parse response
    root := transnet_ipc_generated.GetRootAsRoot(buf[:n], 0)
    if root.MessageType() == transnet_ipc_generated.MessageTranslationResponse {
        resp := new(transnet_ipc_generated.TranslationResponse)
        root.Message(resp)
        return parseTranslationResponse(resp), nil
    }
    
    return nil, fmt.Errorf("unexpected response type")
}

func (c *TransnetClient) Close() error {
    return c.conn.Close()
}
```

## Usage Examples

### Example 1: Anima Requests Translation

```rust
// In Anima (Island OS)
async fn learn_new_word() {
    let mut client = TransnetClient::connect("/tmp/transnet_ipc.sock").await?;
    
    let response = client.translate(
        "anima_user_id",
        "serendipity",
        "en",
        "es"
    ).await?;
    
    println!("Translation: {}", response.translation);
    println!("Meaning: {}", response.word_analysis.meaning);
    
    for relation in response.word_analysis.relations {
        println!("Related: {} ({})", relation.word, relation.meaning);
    }
}
```

### Example 2: Anima Accesses User History

```rust
async fn review_translation_history(user_id: &str) {
    let mut client = TransnetClient::connect("/tmp/transnet_ipc.sock").await?;
    
    let response = client.get_history(user_id, 1, 20).await?;
    
    for entry in response.entries {
        println!("{} → {}", entry.source_text, entry.translation);
    }
}
```

### Example 3: Shared Auth Verification

```rust
async fn verify_token(token: &str) -> Result<UserInfo> {
    let mut client = TransnetClient::connect("/tmp/transnet_ipc.sock").await?;
    
    let response = client.verify_auth(token).await?;
    
    if !response.valid {
        return Err(anyhow::anyhow!("Invalid token"));
    }
    
    Ok(response.user.unwrap())
}
```

## Error Handling

### Error Response Format

```rust
table ErrorResponse {
    request_id: string;
    error_code: ErrorCode;
    message: string;
    details: string;
}
```

### Error Codes

| Code | Description |
|------|-------------|
| `Success` | Operation successful |
| `InvalidRequest` | Request data is invalid |
| `Unauthorized` | Authentication failed |
| `NotFound` | Resource not found |
| `InternalError` | Server error |
| `LlmError` | LLM service error |
| `DbError` | Database error |

### Error Handling Example

```rust
match result {
    Ok(response) => {
        // Handle success
    }
    Err(e) => {
        // Send error response
        let error_response = ErrorResponse {
            request_id: request_id.clone(),
            error_code: ErrorCode::InternalError,
            message: e.to_string(),
            details: String::new(),
        };
        send_error(socket, error_response).await?;
    }
}
```

## Performance Considerations

### Connection Pooling
- Reuse Unix socket connections
- Keep connections alive with keep-alive messages

### Message Batching
- Batch multiple requests in a single message (future enhancement)
- Use pipelining for multiple independent requests

### Buffer Sizing
- Use appropriate buffer sizes (4KB-16KB typical)
- Resize buffers dynamically if needed

### Latency Optimization
- Keep IPC messages small
- Use zero-copy where possible
- Cache frequently accessed data

## Security

### Socket Permissions
```bash
# Set appropriate permissions
chmod 600 /tmp/transnet_ipc.sock
chown transnet:transnet /tmp/transnet_ipc.sock
```

### Authentication
- Verify JWT tokens in AuthVerifyRequest
- Check user permissions for sensitive operations
- Use user_id from token, not from request

### Data Validation
- Validate all input fields
- Sanitize user-provided data
- Check for buffer overflows

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_translation_request() {
        let server = IpcServer::new("/tmp/test.sock", service).await?;
        tokio::spawn(async move { server.run().await });
        
        let mut client = TransnetClient::connect("/tmp/test.sock").await?;
        let response = client.translate("test_user", "hello", "en", "es").await?;
        
        assert_eq!(response.translation, "hola");
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_full_workflow() {
    // Start Transnet server
    let transnet = spawn_transnet_server();
    
    // Connect Island OS
    let mut client = TransnetClient::connect(transnet.socket_path()).await?;
    
    // Test translation
    let response = client.translate("user", "test", "en", "es").await?;
    assert!(response.translation.len() > 0);
    
    // Test word analysis
    let analysis = client.word_analysis("test", "en", "es").await?;
    assert!(analysis.word_analysis.is_some());
}
```

## Future Enhancements

1. **Bidirectional Streaming**: Real-time translation streaming
2. **Message Batching**: Batch multiple requests
3. **Compression**: Compress large messages
4. **Encryption**: Encrypt sensitive data over IPC
5. **Metrics**: Track IPC latency and throughput
6. **Heartbeat**: Keep-alive mechanism for connection health
7. **Reconnection**: Automatic reconnection on failure
