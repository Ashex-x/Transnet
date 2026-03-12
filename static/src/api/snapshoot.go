package api

import (
  "encoding/json"
  "fmt"
  "io"
  "log"
  "net"
  "os"
  "path/filepath"
  "sync"
  "time"

  flatbuffers "github.com/google/flatbuffers/go"
  ecs "github.com/Ashex-x/Island/static/src/api/ecs"
)

// SnapshotAPI handles ECS snapshot and delta streaming.
type SnapshotAPI struct {
  socketPath     string
  conn           *net.UnixConn
  currentTick    uint64
  entities       map[string]*ecs.EntityDelta
  clients        map[*WebSocket]bool
  clientsMutex   sync.RWMutex
}

// NewSnapshotAPI creates a new SnapshotAPI instance.
func NewSnapshotAPI(socketPath string) *SnapshotAPI {
  return &SnapshotAPI{
    socketPath:   socketPath,
    currentTick:  0,
    entities:     make(map[string]*ecs.EntityDelta),
    clients:      make(map[*WebSocket]bool),
  }
}

// Start initializes the UDS connection and starts delta streaming.
func (s *SnapshotAPI) Start() error {
  // Create Unix address
  addr, err := net.ResolveUnixAddr("unix", s.socketPath)
  if err != nil {
    return fmt.Errorf("failed to resolve socket address: %w", err)
  }
  
  // Retry connecting to Rust ECS server until successful
  maxRetries := 30
  retryDelay := 1 * time.Second
  
  for i := 0; i < maxRetries; i++ {
    conn, err := net.DialUnix("unix", nil, addr)
    if err == nil {
      s.conn = conn
      log.Printf("Connected to ECS server at %s", s.socketPath)
      
      // Start reading deltas from Rust
      go s.readDeltas()
      
      return nil
    }
    
    log.Printf("Failed to connect to ECS socket (attempt %d/%d): %v, retrying in %v...", i+1, maxRetries, err, retryDelay)
    time.Sleep(retryDelay)
  }
  
  return fmt.Errorf("failed to connect to ECS socket after %d attempts", maxRetries)
}

// GetSnapshot returns the current full snapshot as compressed JSON.
func (s *SnapshotAPI) GetSnapshot() ([]byte, error) {
  s.clientsMutex.RLock()
  defer s.clientsMutex.RUnlock()
  
  // Convert entities to JSON
  snapshot := make(map[string]interface{})
  snapshot["tick"] = s.currentTick
  snapshot["entities"] = s.entities
  
  data, err := json.Marshal(snapshot)
  if err != nil {
    return nil, fmt.Errorf("failed to marshal snapshot: %w", err)
  }
  
  // Compress data (TODO: implement compression)
  return data, nil
}

// readDeltas continuously reads deltas from Rust ECS server.
func (s *SnapshotAPI) readDeltas() {
  sizeBuf := make([]byte, 4)
  
  for {
    // First read the size prefix (4 bytes)
    _, err := io.ReadFull(s.conn, sizeBuf)
    if err != nil {
      if err == io.EOF {
        log.Println("ECS server closed connection")
        return
      }
      log.Printf("Error reading size prefix: %v", err)
      continue
    }
    
    // Read big-endian size
    sizePrefix := uint32(sizeBuf[0])<<24 | uint32(sizeBuf[1])<<16 | uint32(sizeBuf[2])<<8 | uint32(sizeBuf[3])
    
    // Read the full message
    data := make([]byte, sizePrefix)
    totalRead := 0
    for totalRead < int(sizePrefix) {
      n, err := s.conn.Read(data[totalRead:])
      if err != nil {
        log.Printf("Error reading data: %v", err)
        break
      }
      totalRead += n
    }
    
    // Parse FlatBuffers message
    if err := s.processDelta(data[:totalRead]); err != nil {
      log.Printf("Error processing delta: %v", err)
      continue
    }
    
    // Broadcast to all WebSocket clients (include size prefix for forwarding)
    fullMsg := append(sizeBuf[:], data[:totalRead]...)
    s.broadcastDelta(fullMsg)
  }
}

// processDelta parses a FlatBuffers delta message and updates entity state.
func (s *SnapshotAPI) processDelta(data []byte) error {
  // Get root object
  snapshot := ecs.GetRootAsSnapshot(data, 0)
  
  s.currentTick = snapshot.Tick()
  entitiesLen := snapshot.EntitiesLength()
  
  for i := 0; i < entitiesLen; i++ {
    entity := &ecs.EntityDelta{}
    if !snapshot.Entities(entity, i) {
      continue
    }
    
    id := string(entity.Id())
    
    // Update or create entity
    if existing, ok := s.entities[id]; ok {
      // Update existing entity
      s.updateEntity(existing, entity)
    } else {
      // Create new entity
      newEntity := s.copyEntity(entity)
      s.entities[id] = newEntity
    }
  }
  
  log.Printf("Processed %d entity deltas for tick %d", entitiesLen, s.currentTick)
  return nil
}

// updateEntity updates an existing entity with delta data.
func (s *SnapshotAPI) updateEntity(existing, delta *ecs.EntityDelta) {
  bitmask := delta.Bitmask()
  
  // Update fields based on bitmask
  if bitmask&0x01 != 0 && delta.PositionLength() == 3 {
    log.Printf("DEBUG updateEntity: delta position = [%.2f, %.2f, %.2f]", delta.Position(0), delta.Position(1), delta.Position(2))
    existing.MutatePosition(0, delta.Position(0))
    existing.MutatePosition(1, delta.Position(1))
    existing.MutatePosition(2, delta.Position(2))
    log.Printf("DEBUG updateEntity: existing position after update = [%.2f, %.2f, %.2f]", existing.Position(0), existing.Position(1), existing.Position(2))
  }
  
  if bitmask&0x02 != 0 {
    existing.MutateOrientation(delta.Orientation())
  }
  
  if bitmask&0x04 != 0 {
    // Color is read-only in FlatBuffers, skip mutation
  }
  
  if bitmask&0x08 != 0 {
    // Shape is read-only in FlatBuffers, skip mutation
  }
  
  if bitmask&0x10 != 0 && delta.SizeLength() == 3 {
    existing.MutateSize(0, delta.Size(0))
    existing.MutateSize(1, delta.Size(1))
    existing.MutateSize(2, delta.Size(2))
  }
  
  if bitmask&0x20 != 0 {
    // Name is read-only in FlatBuffers, skip mutation
  }
  
  if bitmask&0x40 != 0 {
    // MaterialType is read-only in FlatBuffers, skip mutation
  }
  
  if bitmask&0x80 != 0 {
    // Components is read-only in FlatBuffers, skip mutation
  }
  
  if bitmask&0x100 != 0 {
    existing.MutateVolume(delta.Volume())
  }
  
  if bitmask&0x200 != 0 {
    existing.MutateToxicity(delta.Toxicity())
  }
  
  if bitmask&0x400 != 0 {
    existing.MutateDegradability(delta.Degradability())
  }
  
  if bitmask&0x800 != 0 {
    existing.MutateFat(delta.Fat())
  }
  
  if bitmask&0x1000 != 0 {
    existing.MutateProtein(delta.Protein())
  }
  
  if bitmask&0x2000 != 0 {
    existing.MutateCarbohydrate(delta.Carbohydrate())
  }
  
  if bitmask&0x4000 != 0 {
    existing.MutateWaterContent(delta.WaterContent())
  }
  
  if bitmask&0x8000 != 0 {
    existing.MutateFiberContent(delta.FiberContent())
  }
  
  if bitmask&0x10000 != 0 {
    existing.MutateVitamins(delta.Vitamins())
  }
  
  if bitmask&0x20000 != 0 {
    existing.MutateTraceElements(delta.TraceElements())
  }
  
  if bitmask&0x40000 != 0 {
    existing.MutateCaloricValue(delta.CaloricValue())
  }
  
  if bitmask&0x80000 != 0 {
    existing.MutateHunger(delta.Hunger())
  }
  
  if bitmask&0x100000 != 0 {
    existing.MutateWater(delta.Water())
  }
}

// copyEntity creates a deep copy of an entity delta.
func (s *SnapshotAPI) copyEntity(src *ecs.EntityDelta) *ecs.EntityDelta {
  builder := flatbuffers.NewBuilder(1024)
  
  // Copy all fields
  id := builder.CreateString(string(src.Id()))
  log.Printf("DEBUG copyEntity: source position = [%.2f, %.2f, %.2f], size = [%.2f, %.2f, %.2f]", src.Position(0), src.Position(1), src.Position(2), src.Size(0), src.Size(1), src.Size(2))
  var position flatbuffers.UOffsetT
  if src.PositionLength() == 3 {
    ecs.EntityDeltaStartPositionVector(builder, src.PositionLength())
    for i := src.PositionLength() - 1; i >= 0; i-- {
      builder.PrependFloat32(src.Position(i))
    }
    position = builder.EndVector(src.PositionLength())
  }
  var size flatbuffers.UOffsetT
  if src.SizeLength() == 3 {
    ecs.EntityDeltaStartSizeVector(builder, src.SizeLength())
    for i := src.SizeLength() - 1; i >= 0; i-- {
      builder.PrependFloat32(src.Size(i))
    }
    size = builder.EndVector(src.SizeLength())
  }
  
  var color, shape, name, materialType, components flatbuffers.UOffsetT
  if len(src.Color()) > 0 {
    color = builder.CreateString(string(src.Color()))
  }
  if len(src.Shape()) > 0 {
    shape = builder.CreateString(string(src.Shape()))
  }
  if len(src.Name()) > 0 {
    name = builder.CreateString(string(src.Name()))
  }
  if len(src.MaterialType()) > 0 {
    materialType = builder.CreateString(string(src.MaterialType()))
  }
  if len(src.Components()) > 0 {
    components = builder.CreateString(string(src.Components()))
  }
  
  ecs.EntityDeltaStart(builder)
  ecs.EntityDeltaAddId(builder, id)
  ecs.EntityDeltaAddBitmask(builder, src.Bitmask())
  
  if position != 0 {
    ecs.EntityDeltaAddPosition(builder, position)
  }
  if size != 0 {
    ecs.EntityDeltaAddSize(builder, size)
  }
  if color != 0 {
    ecs.EntityDeltaAddColor(builder, color)
  }
  if shape != 0 {
    ecs.EntityDeltaAddShape(builder, shape)
  }
  if name != 0 {
    ecs.EntityDeltaAddName(builder, name)
  }
  if materialType != 0 {
    ecs.EntityDeltaAddMaterialType(builder, materialType)
  }
  if components != 0 {
    ecs.EntityDeltaAddComponents(builder, components)
  }
  
  ecs.EntityDeltaAddOrientation(builder, src.Orientation())
  ecs.EntityDeltaAddVolume(builder, src.Volume())
  ecs.EntityDeltaAddToxicity(builder, src.Toxicity())
  ecs.EntityDeltaAddDegradability(builder, src.Degradability())
  ecs.EntityDeltaAddFat(builder, src.Fat())
  ecs.EntityDeltaAddProtein(builder, src.Protein())
  ecs.EntityDeltaAddCarbohydrate(builder, src.Carbohydrate())
  ecs.EntityDeltaAddWaterContent(builder, src.WaterContent())
  ecs.EntityDeltaAddFiberContent(builder, src.FiberContent())
  ecs.EntityDeltaAddVitamins(builder, src.Vitamins())
  ecs.EntityDeltaAddTraceElements(builder, src.TraceElements())
  ecs.EntityDeltaAddCaloricValue(builder, src.CaloricValue())
  ecs.EntityDeltaAddHunger(builder, src.Hunger())
  ecs.EntityDeltaAddWater(builder, src.Water())
  
  entityOffset := ecs.EntityDeltaEnd(builder)
  builder.Finish(entityOffset)
  
  // Create EntityDelta from the built buffer
  buf := builder.FinishedBytes()
  return ecs.GetRootAsEntityDelta(buf, 0)
}

// broadcastDelta sends delta data to all connected WebSocket clients.
func (s *SnapshotAPI) broadcastDelta(data []byte) {
  s.clientsMutex.RLock()
  defer s.clientsMutex.RUnlock()
  
  for client := range s.clients {
    if err := client.Send(data); err != nil {
      log.Printf("Error sending to client: %v", err)
      s.unregisterClient(client)
    }
  }
}

// registerClient adds a new WebSocket client.
func (s *SnapshotAPI) registerClient(client *WebSocket) {
  s.clientsMutex.Lock()
  defer s.clientsMutex.Unlock()
  
  s.clients[client] = true
  log.Printf("Client registered. Total clients: %d", len(s.clients))
}

// unregisterClient removes a WebSocket client.
func (s *SnapshotAPI) unregisterClient(client *WebSocket) {
  s.clientsMutex.Lock()
  defer s.clientsMutex.Unlock()
  
  delete(s.clients, client)
  log.Printf("Client unregistered. Total clients: %d", len(s.clients))
}

// WebSocket is a placeholder for WebSocket connection.
// TODO: Replace with actual WebSocket implementation.
type WebSocket struct {
  conn net.Conn
}

// Send sends binary data to WebSocket client.
func (ws *WebSocket) Send(data []byte) error {
  // TODO: Implement WebSocket send
  return nil
}

// Close closes the WebSocket connection.
func (ws *WebSocket) Close() error {
  if ws.conn != nil {
    return ws.conn.Close()
  }
  return nil
}

// Ensure socket directory exists
func initSocketDir(socketPath string) error {
  dir := filepath.Dir(socketPath)
  if err := os.MkdirAll(dir, 0755); err != nil {
    return fmt.Errorf("failed to create socket directory: %w", err)
  }
  return nil
}

// Set socket file permissions
func setSocketPermissions(socketPath string) error {
  // Set socket to be readable/writable by all
  if err := os.Chmod(socketPath, 0777); err != nil {
    return fmt.Errorf("failed to set socket permissions: %w", err)
  }
  return nil
}