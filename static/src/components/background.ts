/**
 * background.ts handles the Metaland background with grid dots
 * and interactive dot motion near mouse cursor.
 */

interface GridDot {
  element: HTMLDivElement;
  originalX: number;
  originalY: number;
  currentX: number;
  currentY: number;
}

export class Background {
  private container: HTMLElement;
  private gridContainer: HTMLDivElement | null;
  private gridDots: GridDot[];
  private mouseX: number;
  private mouseY: number;

  constructor(container: HTMLElement) {
    this.container = container;
    this.gridContainer = null;
    this.gridDots = [];
    this.mouseX = 0;
    this.mouseY = 0;
  }

  render(): void {
    this.createGridContainer();
    this.createGridDots();
    this.setupEventListeners();
  }

  private createGridContainer(): void {
    this.gridContainer = document.createElement('div');
    this.gridContainer.id = 'grid-container';
    this.container.appendChild(this.gridContainer);
  }

  private createGridDots(): void {
    const spacing = 30;
    const width = window.innerWidth;
    const height = window.innerHeight;

    for (let x = 0; x <= width; x += spacing) {
      for (let y = 0; y <= height; y += spacing) {
        const dot = document.createElement('div');
        dot.className = 'grid-dot';
        dot.style.left = `${x}px`;
        dot.style.top = `${y}px`;
        dot.style.width = '3px';
        dot.style.height = '3px';
        
        if (this.gridContainer) {
          this.gridContainer.appendChild(dot);
          this.gridDots.push({
            element: dot,
            originalX: x,
            originalY: y,
            currentX: x,
            currentY: y
          });
        }
      }
    }
  }

  private setupEventListeners(): void {
    document.addEventListener('mousemove', (e: MouseEvent) => {
      this.mouseX = e.clientX;
      this.mouseY = e.clientY;
    });

    document.addEventListener('click', () => {
      this.scatterDots();
    });

    window.addEventListener('resize', () => {
      this.handleResize();
    });
  }

  private scatterDots(): void {
    const scatterRadius = 100;
    const scatterForce = 50;

    this.gridDots.forEach(dot => {
      const dx = dot.currentX - this.mouseX;
      const dy = dot.currentY - this.mouseY;
      const distance = Math.sqrt(dx * dx + dy * dy);

      if (distance < scatterRadius) {
        const angle = Math.atan2(dy, dx);
        const force = (scatterRadius - distance) / scatterRadius * scatterForce;
        
        dot.currentX += Math.cos(angle) * force;
        dot.currentY += Math.sin(angle) * force;

        dot.element.style.transform = `translate(${dot.currentX - dot.originalX}px, ${dot.currentY - dot.originalY}px)`;

        setTimeout(() => {
          this.returnDot(dot);
        }, 300);
      }
    });
  }

  private returnDot(dot: GridDot): void {
    dot.currentX = dot.originalX;
    dot.currentY = dot.originalY;
    dot.element.style.transform = `translate(0px, 0px)`;
  }

  private handleResize(): void {
    if (this.gridContainer) {
      this.gridContainer.innerHTML = '';
    }
    this.gridDots = [];
    this.createGridDots();
  }

  setDarkMode(enabled: boolean): void {
    const container = document.getElementById('metaland');
    if (container) {
      if (enabled) {
        container.classList.add('dark-mode');
      } else {
        container.classList.remove('dark-mode');
      }
    }
  }

  destroy(): void {
    if (this.gridContainer) {
      this.gridContainer.remove();
    }
    this.gridDots = [];
  }
}