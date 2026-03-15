/**
 * Entry page for the `/metaland` route.
 *
 * The page mounts the Metaland background and exposes the first zone shortcut
 * that leads deeper into the Metaland experience.
 */

import { Background } from './background';
import { router } from '../router';

export class Metaland {
  private container: HTMLElement;
  private background: Background | null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.background = null;
  }

  /**
   * Render the route shell and attach the interactive background layer.
   */
  render(): void {
    this.createMetalandPage();
    this.background = new Background(this.container);
    this.background.render();
  }

  /**
   * Build the Metaland route markup and its house-entry shortcut.
   */
  private createMetalandPage(): void {
    const metaland = document.createElement('div');
    metaland.id = 'metaland';
    metaland.className = 'page active';

    const mapContainer = document.createElement('div');
    mapContainer.id = 'map-container';

    const houseButton = document.createElement('button');
    houseButton.id = 'house-button';
    houseButton.className = 'zone-button';
    houseButton.textContent = 'House';
    houseButton.addEventListener('click', () => {
      router.navigate('/metaland/house');
    });

    metaland.appendChild(mapContainer);
    metaland.appendChild(houseButton);
    this.container.appendChild(metaland);
  }

  /**
   * Remove the background and route markup from the DOM.
   */
  destroy(): void {
    if (this.background) {
      this.background.destroy();
    }

    const metaland = document.getElementById('metaland');
    if (metaland) {
      metaland.remove();
    }
  }
}