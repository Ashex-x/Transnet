class Router {
  private routes: Map<string, () => void>;

  constructor() {
    this.routes = new Map();
    this.setupEventListeners();
  }

  register(path: string, handler: () => void): void {
    this.routes.set(path, handler);
  }

  handleRoute(): void {
    const path = window.location.pathname;
    const handler = this.routes.get(path);

    if (handler) {
      handler();
    } else {
      this.routes.get('/')?.();
    }
  }

  navigate(path: string): void {
    window.history.pushState({}, '', path);
    this.handleRoute();
  }

  private setupEventListeners(): void {
    window.addEventListener('popstate', () => {
      this.handleRoute();
    });
  }
}

const router = new Router();
export { router };
