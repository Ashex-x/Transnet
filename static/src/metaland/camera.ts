/**
 * camera.ts handles camera system for 2D world navigation.
 * Supports zooming and panning with coordinate transformations.
 */

interface Component {
  type: string;
  position?: [number, number];
}

interface Entity {
  components: Component[];
}

interface ScreenCoordinates {
  x: number;
  y: number;
}

interface CameraState {
  cameraX: number;
  cameraY: number;
  zoom: number;
}

type CameraChangeCallback = () => void;

export class Camera {
  private cameraX: number;
  private cameraY: number;
  private zoom: number;
  private minZoom: number;
  private maxZoom: number;
  
  private isPanning: boolean;
  private panStartX: number;
  private panStartY: number;
  private cameraStartX: number;
  private cameraStartY: number;

  constructor() {
    this.cameraX = 0;  // Camera X position in world coordinates
    this.cameraY = 0;  // Camera Y position in world coordinates
    this.zoom = 1.0;  // Zoom scale factor
    this.minZoom = 0.01;  // Minimum zoom
    this.maxZoom = 2.0;  // Maximum zoom
    
    this.isPanning = false;
    this.panStartX = 0;
    this.panStartY = 0;
    this.cameraStartX = 0;
    this.cameraStartY = 0;
  }

  /**
   * Initialize camera to world center (bounding box center) and fit entities on screen
   * @param entities - Array of entities to calculate center from
   * @param screenWidth - Screen width
   * @param screenHeight - Screen height
   */
  initializeToCenter(entities: Entity[], screenWidth: number, screenHeight: number): void {
    if (entities.length === 0) return;

    // Calculate bounding box center
    let minX = Infinity;
    let maxX = -Infinity;
    let minY = Infinity;
    let maxY = -Infinity;

    entities.forEach(entity => {
      const observable = entity.components.find(c => c.type === 'Observable');
      if (observable && observable.position) {
        const [x, y] = observable.position;
        minX = Math.min(minX, x);
        maxX = Math.max(maxX, x);
        minY = Math.min(minY, y);
        maxY = Math.max(maxY, y);
      }
    });

    // Set camera to bounding box center
    this.cameraX = (minX + maxX) / 2;
    this.cameraY = (minY + maxY) / 2;

    // Calculate zoom to fit entities on screen with 90% padding
    const worldWidth = maxX - minX;
    const worldHeight = maxY - minY;
    const scaleX = (screenWidth * 0.9) / worldWidth;
    const scaleY = (screenHeight * 0.9) / worldHeight;
    this.zoom = Math.min(scaleX, scaleY, 1.0); // Cap at 1.0

    console.log(`Camera initialized to center: (${this.cameraX.toFixed(2)}, ${this.cameraY.toFixed(2)}) with zoom: ${this.zoom.toFixed(4)}`);
  }

  /**
   * Convert world coordinates to screen coordinates
   * @param worldX - World X coordinate
   * @param worldY - World Y coordinate
   * @param screenWidth - Screen width
   * @param screenHeight - Screen height
   * @returns {x, y} - Screen coordinates
   */
  worldToScreen(worldX: number, worldY: number, screenWidth: number, screenHeight: number): ScreenCoordinates {
    const screenX = (worldX - this.cameraX) * this.zoom + screenWidth / 2;
    const screenY = (worldY - this.cameraY) * this.zoom + screenHeight / 2;
    return { x: screenX, y: screenY };
  }

  /**
   * Convert screen coordinates to world coordinates
   * @param screenX - Screen X coordinate
   * @param screenY - Screen Y coordinate
   * @param screenWidth - Screen width
   * @param screenHeight - Screen height
   * @returns {x, y} - World coordinates
   */
  screenToWorld(screenX: number, screenY: number, screenWidth: number, screenHeight: number): ScreenCoordinates {
    const worldX = (screenX - screenWidth / 2) / this.zoom + this.cameraX;
    const worldY = (screenY - screenHeight / 2) / this.zoom + this.cameraY;
    return { x: worldX, y: worldY };
  }

  /**
   * Zoom in/out
   * @param factor - Zoom factor (e.g., 1.1 for zoom in, 0.9 for zoom out)
   */
  zoomBy(factor: number): void {
    const newZoom = this.zoom * factor;
    if (newZoom >= this.minZoom && newZoom <= this.maxZoom) {
      this.zoom = newZoom;
    }
  }

  /**
   * Set zoom level
   * @param zoom - New zoom level
   */
  setZoom(zoom: number): void {
    this.zoom = Math.max(this.minZoom, Math.min(this.maxZoom, zoom));
    console.log(`Zoom set to: ${this.zoom.toFixed(4)}`);
  }

  /**
   * Move camera by world coordinates
   * @param dx - World X delta
   * @param dy - World Y delta
   */
  moveBy(dx: number, dy: number): void {
    this.cameraX += dx;
    this.cameraY += dy;
  }

  /**
   * Set camera position
   * @param x - World X coordinate
   * @param y - World Y coordinate
   */
  setPosition(x: number, y: number): void {
    this.cameraX = x;
    this.cameraY = y;
  }

  /**
   * Setup mouse event handlers for zooming and panning
   * @param container - Container element
   * @param onCameraChange - Callback when camera changes
   */
  setupControls(container: HTMLElement, onCameraChange?: CameraChangeCallback): void {
    // Wheel event for zooming
    container.addEventListener('wheel', (e: WheelEvent) => {
      e.preventDefault();
      const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
      this.zoomBy(zoomFactor);
      if (onCameraChange) onCameraChange();
    }, { passive: false });

    // Middle mouse button for panning
    container.addEventListener('mousedown', (e: MouseEvent) => {
      if (e.button === 1) {  // Middle mouse button
        e.preventDefault();
        this.isPanning = true;
        this.panStartX = e.clientX;
        this.panStartY = e.clientY;
        this.cameraStartX = this.cameraX;
        this.cameraStartY = this.cameraY;
        container.style.cursor = 'grabbing';
      }
    });

    document.addEventListener('mousemove', (e: MouseEvent) => {
      if (this.isPanning) {
        const dx = (e.clientX - this.panStartX) / this.zoom;
        const dy = (e.clientY - this.panStartY) / this.zoom;
        this.cameraX = this.cameraStartX - dx;
        this.cameraY = this.cameraStartY - dy;
        if (onCameraChange) onCameraChange();
      }
    });

    document.addEventListener('mouseup', (_e: MouseEvent) => {
      if (this.isPanning) {
        this.isPanning = false;
        container.style.cursor = 'default';
      }
    });

    // Prevent context menu on middle click
    container.addEventListener('contextmenu', (e: MouseEvent) => {
      if (e.button === 1) {
        e.preventDefault();
      }
    });
  }

  /**
   * Get camera state
   * @returns Camera state
   */
  getState(): CameraState {
    return {
      cameraX: this.cameraX,
      cameraY: this.cameraY,
      zoom: this.zoom
    };
  }

  /**
   * Reset camera to default state
   */
  reset(): void {
    this.cameraX = 0;
    this.cameraY = 0;
    this.zoom = 1.0;
    console.log('Camera reset');
  }
}