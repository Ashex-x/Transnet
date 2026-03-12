/**
 * app.ts serves as the main entry point for the frontend application.
 * It bootstraps the application and loads the appropriate page components
 * when a user navigates to a specific route.
 * MainWeb is the initial landing page/component presented to the user.
 */

import { MainWeb } from './components/main_web';
import { Metaland } from './components/metaland';
import { House } from './components/house';
import { Transnet } from './components/transnet';
import { router } from './router';

interface Component {
  destroy(): void;
  render(): void | Promise<void>;
}

class App {
  private currentComponent: Component | null;

  constructor() {
    this.currentComponent = null;
  }

  init(): void {
    router.register('/', () => this.loadMainWeb());
    router.register('/metaland', () => this.loadMetaland());
    router.register('/metaland/house', () => this.loadHouse());
    router.register('/transnet', () => this.loadTransnet());
    router.handleRoute();
  }

  loadMainWeb(): void {
    if (this.currentComponent) {
      this.currentComponent.destroy();
    }
    const component = new MainWeb();
    this.currentComponent = component;
    component.render();
  }

  loadMetaland(): void {
    if (this.currentComponent) {
      this.currentComponent.destroy();
    }
    const component = new Metaland(document.body);
    this.currentComponent = component;
    component.render();
  }

  async loadHouse(): Promise<void> {
    if (this.currentComponent) {
      this.currentComponent.destroy();
    }
    const component = new House(document.body);
    this.currentComponent = component;
    await component.render();
  }

  loadTransnet(): void {
    if (this.currentComponent) {
      this.currentComponent.destroy();
    }
    const component = new Transnet(document.body);
    this.currentComponent = component;
    component.render();
  }
}

const app = new App();
document.addEventListener('DOMContentLoaded', () => {
  app.init();
});

export { App, app };