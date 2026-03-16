/**
 * ecs.ts handles ECS data streaming and rendering.
 * 
 * This module:
 * 1. Fetches initial snapshot from Go server (compressed JSON)
 * 2. Connects to WebSocket for real-time delta updates (FlatBuffers)
 * 3. Maintains entity state in memory
 * 4. Renders entities using ECSRenderer
 */

import { Camera } from './camera';
import { ECSRenderer } from './renderer';
import { EntityState, JSONSnapshot } from './types';
import * as flatbuffers from 'flatbuffers';
import { Snapshot } from '../flatbuffers/ecs_generated';

// Bitmask definitions for EntityDelta fields (matching Rust/Go)
const BIT_POSITION = 1 << 0;
const BIT_ORIENTATION = 1 << 1;
const BIT_COLOR = 1 << 2;
const BIT_SHAPE = 1 << 3;
const BIT_SIZE = 1 << 4;
const BIT_NAME = 1 << 5;
const BIT_MATERIAL_TYPE = 1 << 6;
const BIT_COMPONENTS = 1 << 7;
const BIT_VOLUME = 1 << 8;
const BIT_TOXICITY = 1 << 9;
const BIT_DEGRADABILITY = 1 << 10;
const BIT_FAT = 1 << 11;
const BIT_PROTEIN = 1 << 12;
const BIT_CARBOHYDRATE = 1 << 13;
const BIT_WATER_CONTENT = 1 << 14;
const BIT_FIBER_CONTENT = 1 << 15;
const BIT_VITAMINS = 1 << 16;
const BIT_TRACE_ELEMENTS = 1 << 17;
const BIT_CALORIC_VALUE = 1 << 18;
const BIT_HUNGER = 1 << 19;
const BIT_WATER = 1 << 20;


export class ECSClient {
  private entities: Map<string, EntityState>;
  private camera: Camera;
  private renderer: ECSRenderer;
  private ws: WebSocket | null;
  private currentTick: number;

  constructor() {
    this.entities = new Map();
    this.camera = new Camera();
    this.renderer = new ECSRenderer('map-container');
    this.ws = null;
    this.currentTick = 0;
  }

  /**
   * Initialize ECS client.
   * 1. Fetch initial snapshot from Go server
   * 2. Connect to WebSocket for delta updates
   * 3. Setup camera and render
   */
  async init(): Promise<void> {
    const container = this.renderer.getContainer();
    if (!container) {
      throw new Error('map-container not found');
    }

    // Fetch initial snapshot
    await this.fetchSnapshot();

    // Initialize camera - convert EntityState to Entity format expected by Camera
    const entitiesForCamera = Array.from(this.entities.values())
      .filter(e => e.position && Array.isArray(e.position) && e.position.length >= 2)
      .map(e => ({
        id: e.id,
        components: [{
          type: 'Observable',
          position: [e.position[0], e.position[1]] as [number, number], // Extract 2D position from 3D
          size: (e.size && Array.isArray(e.size) && e.size.length >= 2 ? [e.size[0], e.size[1]] : [10, 10]) as [number, number], // Extract 2D size from 3D
          orientation: e.orientation || 0,
          color: e.color,
          shape: e.shape,
          name: e.name,
        }]
      }));
    const screenWidth = container.clientWidth;
    const screenHeight = container.clientHeight;
    this.camera.initializeToCenter(entitiesForCamera, screenWidth, screenHeight);

    // Render initial entities
    this.renderEntities();
    this.renderer.fixZIndex();

    // Setup camera controls
    this.setupCameraControls();

    // Connect to WebSocket for delta updates
    this.connectWebSocket();
  }

  /**
   * Fetch initial snapshot from Go server.
   * Snapshot is compressed JSON containing all entities.
   */
  private async fetchSnapshot(): Promise<void> {
    try {
      const response = await fetch('/api/snapshot');
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      const snapshot: JSONSnapshot = data;

      this.currentTick = snapshot.tick;

      // Parse entities from snapshot
      for (const [id, entity] of Object.entries(snapshot.entities)) {
        // Coordinate transformation: swap x and z, keep y same
        if (entity.position && entity.position.length === 3) {
          entity.position = [entity.position[2], entity.position[1], entity.position[0]];
        }
        if (entity.size && entity.size.length === 3) {
          entity.size = [entity.size[2], entity.size[1], entity.size[0]];
        }
        this.entities.set(id, entity);
      }

      console.log(`Loaded snapshot with ${this.entities.size} entities at tick ${this.currentTick}`);

      // Debug: Log first entity structure
      const firstEntityId = Object.keys(snapshot.entities)[0];
      if (firstEntityId) {
        console.log('First entity data:', snapshot.entities[firstEntityId]);
      }
    } catch (error) {
      console.error('Error fetching snapshot:', error);
      throw error;
    }
  }

  /**
   * Connect to WebSocket for real-time delta updates.
   * Deltas are FlatBuffers binary format.
   */
  private connectWebSocket(): void {
    const wsUrl = `ws://${window.location.host}/api/ws`;
    this.ws = new WebSocket(wsUrl);

    this.ws.binaryType = 'arraybuffer';

    this.ws.onopen = () => {
      console.log('WebSocket connected to ECS stream');
    };

    this.ws.onmessage = (event) => {
      this.handleDeltaMessage(event.data);
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    this.ws.onclose = () => {
      console.log('WebSocket disconnected, attempting to reconnect...');
      setTimeout(() => this.connectWebSocket(), 3000);
    };
  }

  /**
   * Handle incoming delta message from WebSocket.
   * Parse FlatBuffers and update entity state.
   */
  private handleDeltaMessage(data: ArrayBuffer): void {
    try {
      const buf = new Uint8Array(data);
      const bb = new flatbuffers.ByteBuffer(buf);
      const snapshot = Snapshot.getRootAsSnapshot(bb);

      this.currentTick = Number(snapshot.tick());

      // Update entities based on deltas
      const entitiesLength = snapshot.entitiesLength();
      for (let i = 0; i < entitiesLength; i++) {
        const entityDelta = snapshot.entities(i);
        if (entityDelta) {
          this.updateEntityFromFlatBuffers(entityDelta);
        }
      }

      // Re-render entities
      this.renderEntities();
    } catch (error) {
      console.error('Error handling delta message:', error);
    }
  }

  /**
   * Update entity state from FlatBuffers delta.
   */
  private updateEntityFromFlatBuffers(entityDelta: any): void {
    const id = entityDelta.id();
    let entity = this.entities.get(id);

    if (!entity) {
      // Create new entity
      entity = {
        id: id,
        position: [0, 0, 0],
        orientation: 0,
      };
      this.entities.set(id, entity);
    }

    // Update fields based on bitmask
    const bitmask = entityDelta.bitmask();

    if (bitmask & BIT_POSITION && entityDelta.positionLength() === 3) {
      entity.position = [
        entityDelta.position(2),
        entityDelta.position(1),
        entityDelta.position(0),
      ];
    }

    if (bitmask & BIT_ORIENTATION) {
      entity.orientation = entityDelta.orientation();
    }

    if (bitmask & BIT_COLOR) {
      entity.color = entityDelta.color();
    }

    if (bitmask & BIT_SHAPE) {
      entity.shape = entityDelta.shape();
    }

    if (bitmask & BIT_SIZE && entityDelta.sizeLength() === 3) {
      entity.size = [
        entityDelta.size(2),
        entityDelta.size(1),
        entityDelta.size(0),
      ];
    }

    if (bitmask & BIT_NAME) {
      entity.name = entityDelta.name();
    }

    if (bitmask & BIT_MATERIAL_TYPE) {
      entity.material_type = entityDelta.materialType();
    }

    if (bitmask & BIT_COMPONENTS) {
      entity.components = entityDelta.components();
    }

    if (bitmask & BIT_VOLUME) {
      entity.volume = entityDelta.volume();
    }

    if (bitmask & BIT_TOXICITY) {
      entity.toxicity = entityDelta.toxicity();
    }

    if (bitmask & BIT_DEGRADABILITY) {
      entity.degradability = entityDelta.degradability();
    }

    if (bitmask & BIT_FAT) {
      entity.fat = entityDelta.fat();
    }

    if (bitmask & BIT_PROTEIN) {
      entity.protein = entityDelta.protein();
    }

    if (bitmask & BIT_CARBOHYDRATE) {
      entity.carbohydrate = entityDelta.carbohydrate();
    }

    if (bitmask & BIT_WATER_CONTENT) {
      entity.water_content = entityDelta.waterContent();
    }

    if (bitmask & BIT_FIBER_CONTENT) {
      entity.fiber_content = entityDelta.fiberContent();
    }

    if (bitmask & BIT_VITAMINS) {
      entity.vitamins = entityDelta.vitamins();
    }

    if (bitmask & BIT_TRACE_ELEMENTS) {
      entity.trace_elements = entityDelta.traceElements();
    }

    if (bitmask & BIT_CALORIC_VALUE) {
      entity.caloric_value = entityDelta.caloricValue();
    }

    if (bitmask & BIT_HUNGER) {
      entity.hunger = entityDelta.hunger();
    }

    if (bitmask & BIT_WATER) {
      entity.water = entityDelta.water();
    }
  }

  /**
   * Render all entities to DOM.
   */
  private renderEntities(): void {
    this.renderer.render(this.entities, this.camera);
  }

  /**
   * Setup camera controls with re-render callback.
   */
  private setupCameraControls(): void {
    const mapContainer = this.renderer.getContainer();
    if (!mapContainer) return;

    this.camera.setupControls(mapContainer, () => {
      this.renderEntities();
    });
  }

  /**
   * Destroy ECS client and cleanup.
   */
  destroy(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.renderer.destroy();
    this.entities.clear();
  }
}
