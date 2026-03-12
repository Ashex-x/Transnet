/**
 * metaland.ts handles the Metaland page with background and map.
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

  render(): void {
    this.createMetalandPage();
    this.background = new Background(this.container);
    this.background.render();
  }

  private createMetalandPage(): void {
    const metaland = document.createElement('div');
    metaland.id = 'metaland';
    metaland.className = 'page active';

    const mapContainer = document.createElement('div');
    mapContainer.id = 'map-container';

    // Add house button
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