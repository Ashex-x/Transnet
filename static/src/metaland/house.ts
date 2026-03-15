/**
 * Route page for `/metaland/house`.
 *
 * This page creates the house scene shell, mounts the shared Metaland
 * background, and boots the ECS-driven house visualization.
 */

import { House as HouseECS } from './zones/house';
import { Background } from './background';
import { router } from '../router';

export class House {
  private container: HTMLElement;
  private houseECS: HouseECS | null;
  private background: Background | null;

  constructor(container: HTMLElement) {
    this.container = container;
    this.houseECS = null;
    this.background = null;
  }

  /**
   * Render the page shell, background, and house ECS scene.
   */
  async render(): Promise<void> {
    this.createHousePage();
    this.background = new Background(this.container);
    this.background.render();

    this.houseECS = new HouseECS();
    await this.houseECS.init();
  }

  /**
   * Build the route markup and its back-navigation control.
   */
  private createHousePage(): void {
    const house = document.createElement('div');
    house.id = 'house';
    house.className = 'page active';

    const mapContainer = document.createElement('div');
    mapContainer.id = 'map-container';

    const backButton = document.createElement('button');
    backButton.id = 'back-button';
    backButton.className = 'back-button';
    backButton.textContent = '← Back';
    backButton.addEventListener('click', () => {
      router.navigate('/metaland');
    });

    house.appendChild(mapContainer);
    house.appendChild(backButton);
    this.container.appendChild(house);
  }

  /**
   * Tear down the ECS scene, background, and DOM nodes for the route.
   */
  destroy(): void {
    if (this.houseECS) {
      this.houseECS.destroy();
    }
    
    if (this.background) {
      this.background.destroy();
    }

    const house = document.getElementById('house');
    if (house) {
      house.remove();
    }
  }
}