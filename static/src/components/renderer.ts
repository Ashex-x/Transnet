import { Camera } from './camera';
import { EntityState } from './types';

export class ECSRenderer {
  private container: HTMLElement | null;
  private entityElements: Map<string, HTMLDivElement>;

  constructor(containerId: string) {
    this.container = document.getElementById(containerId);
    this.entityElements = new Map();
    if (!this.container) {
      console.error(`Container with id '${containerId}' not found.`);
    }
  }

  /**
   * Render all entities to DOM.
   */
  render(entities: Map<string, EntityState>, camera: Camera): void {
    if (!this.container) return;

    console.log(`Rendering ${entities.size} entities`);

    let renderedCount = 0;
    let skippedCount = 0;

    // Track which entities are still present
    const presentEntityIds = new Set<string>();

    entities.forEach((entity, id) => {
      presentEntityIds.add(id);

      // Skip if entity doesn't have position
      if (!entity.position || !Array.isArray(entity.position) || entity.position.length < 3) {
        skippedCount++;
        return;
      }

      renderedCount++;

      // Check if entity element already exists
      let entityEl = this.entityElements.get(id);

      if (!entityEl) {
        // Create new entity element
        entityEl = this.createEntityElement(entity, camera);
        this.container!.appendChild(entityEl);
        this.entityElements.set(id, entityEl);
      } else {
        // Update existing entity element
        this.updateEntityElement(entityEl, entity, camera);
      }
    });

    console.log(`Rendered: ${renderedCount}, Skipped (no position): ${skippedCount}`);

    // Remove entities that no longer exist
    this.entityElements.forEach((el, id) => {
      if (!presentEntityIds.has(id)) {
        el.remove();
        this.entityElements.delete(id);
      }
    });
  }

  /**
   * Create a new entity DOM element.
   */
  private createEntityElement(entity: EntityState, camera: Camera): HTMLDivElement {
    const el = document.createElement('div');
    el.className = 'entity';
    el.id = entity.id;

    // Set initial styles
    el.style.position = 'absolute';

    // Check if this is Myuna or has zero size - render as pulsing pink dot
    const isMyuna = entity.name === 'Myuna';
    const hasZeroSize = entity.size && entity.size.every(s => s === 0);

    if (isMyuna || hasZeroSize) {
      el.setAttribute('data-myuna', 'true');

      const innerDot = document.createElement('div');
      innerDot.className = 'myuna-dot heart-beat';
      innerDot.style.backgroundColor = '#FF69B4';
      innerDot.style.borderRadius = '50%';
      innerDot.style.width = '100%';
      innerDot.style.height = '100%';
      innerDot.style.filter = 'blur(2px)';
      innerDot.style.boxShadow = '0 0 20px 5px rgba(169, 169, 169, 0.6), 0 0 15px 3px rgba(255, 105, 180, 0.4)';
      el.appendChild(innerDot);
    } else {
      el.style.backgroundColor = entity.color || '#cccccc';
    }

    // Name tooltip
    if (entity.name) {
      el.title = entity.name;
      el.setAttribute('data-name', entity.name);
    }

    // Add click handler for debugging
    el.addEventListener('click', (e: MouseEvent) => {
      e.stopPropagation();
      console.log('Entity clicked:', entity);
      this.showEntityInfo(entity);
    });

    // Update position and size
    this.updateEntityElement(el, entity, camera);

    return el;
  }

  private updateEntityShape(el: HTMLDivElement, entity: EntityState): void {
    const shape = entity.shape ? entity.shape.toLowerCase() : 'rectangle';

    if (['ellipse', 'round', 'circle'].includes(shape)) {
      el.style.borderRadius = '50%';
    } else {
      el.style.borderRadius = '0';
    }

    const indicator = el.querySelector('.direction-indicator');
    if (indicator) indicator.remove();
  }

  /**
   * Update an existing entity DOM element.
   */
  private updateEntityElement(el: HTMLDivElement, entity: EntityState, camera: Camera): void {
    // Skip if entity doesn't have position
    if (!entity.position || !Array.isArray(entity.position) || entity.position.length < 3) {
      return;
    }

    // Update Shape if changed (though usually static, but good to be safe)
    this.updateEntityShape(el, entity);
    if (entity.color) el.style.backgroundColor = entity.color;

    // Entity position is center position (x, y, z)
    const [centerX, centerY, centerZ] = entity.position;

    // Size - check for Myuna or zero-size entities
    const isMyuna = el.hasAttribute('data-myuna');
    let width, height;
    if (isMyuna || (entity.size && entity.size.every(s => s === 0))) {
      width = 16;
      height = 16;
    } else if (entity.size && Array.isArray(entity.size) && entity.size.length >= 2) {
      width = entity.size[0];
      height = entity.size[1];
    } else {
      width = 10;
      height = 10;
    }
    const zoom = camera.getState().zoom;
    const scaledWidth = width * zoom;
    const scaledHeight = height * zoom;

    // Orientation is in radians (single number)
    const angle = entity.orientation || 0;

    // Convert world position to screen position using camera
    const screenPos = camera.worldToScreen(centerX, centerY, window.innerWidth, window.innerHeight);

    // CSS positions from top-left, so subtract half size to center element
    const left = screenPos.x - scaledWidth / 2;
    const top = screenPos.y - scaledHeight / 2;

    // Update styles efficiently
    el.style.left = `${left}px`;
    el.style.top = `${top}px`;
    el.style.width = `${scaledWidth}px`;
    el.style.height = `${scaledHeight}px`;

    // Orientation (rotation) - convert radians to degrees for CSS
    const angleDeg = angle * (180 / Math.PI);
    el.style.transform = `rotate(${angleDeg}deg)`;

    // Use z-coordinate for layer ordering (z-index)
    // Higher z values appear on top (add base offset to ensure positive values)
    // Myuna is always on top
    if (isMyuna) {
      el.style.zIndex = '999999';
    } else {
      el.style.zIndex = `${Math.floor(centerZ * 100) + 1000}`;
    }
  }

  /**
   * Show entity info in alert.
   */
  private showEntityInfo(entity: EntityState): void {
    const info = {
      id: entity.id,
      name: entity.name || 'Unnamed',
      position: entity.position,
      size: entity.size,
      shape: entity.shape,
      color: entity.color,
      orientation: entity.orientation
    };
    console.log('Entity Info:', info);
    alert(JSON.stringify(info, null, 2));
  }

  /**
   * Fix z-index to ensure proper layering.
   */
  fixZIndex(): void {
    const gridContainer = document.getElementById('grid-container');
    const mapContainer = document.getElementById('map-container');

    if (gridContainer) {
      gridContainer.style.zIndex = '1';
    }

    if (mapContainer) {
      mapContainer.style.zIndex = '10';
    }
  }

  getContainer(): HTMLElement | null {
    return this.container;
  }

  /**
   * Destroy renderer and cleanup.
   */
  destroy(): void {
    if (this.container) {
      this.entityElements.forEach(el => el.remove());
    }
    this.entityElements.clear();
  }
}
