/**
 * Shared animated particle background used across multiple pages.
 *
 * The background owns a full-screen canvas and keeps its own animation loop,
 * so individual pages only need to create and destroy the instance.
 */

export class ParticleBackground {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private particles: Particle[] = [];
  private animationId: number | null = null;
  private mouse = { x: 0, y: 0 };

  private readonly config = {
    particleCount: 60,
    connectionDistance: 150,
    maxConnections: 3,
    particleSpeed: 0.3,
    particleRadius: { min: 1, max: 2.5 },
    colors: {
      particle: 'rgba(0, 240, 255, 0.6)',
      particleDim: 'rgba(0, 240, 255, 0.3)',
      line: 'rgba(0, 240, 255, 0.15)',
    },
  };

  constructor(container: HTMLElement = document.body) {
    this.canvas = document.createElement('canvas');
    this.canvas.id = 'particle-canvas';

    const ctx = this.canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Could not get canvas context');
    }

    this.ctx = ctx;
    this.setupCanvas(container);
    this.initParticles();
    this.bindEvents();
    this.start();
  }

  /**
   * Attach the background canvas and size it to the viewport.
   */
  private setupCanvas(container: HTMLElement): void {
    this.canvas.style.cssText = `
      position: fixed;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      z-index: -1;
      pointer-events: none;
    `;

    container.appendChild(this.canvas);
    this.resize();
  }

  /**
   * Resize the canvas for the current viewport and device pixel ratio.
   */
  private resize(): void {
    const dpr = window.devicePixelRatio || 1;
    const width = window.innerWidth;
    const height = window.innerHeight;

    this.canvas.width = width * dpr;
    this.canvas.height = height * dpr;
    this.canvas.style.width = `${width}px`;
    this.canvas.style.height = `${height}px`;
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  /**
   * Seed the animated particle field for the current viewport size.
   */
  private initParticles(): void {
    const width = window.innerWidth;
    const height = window.innerHeight;

    this.particles = [];
    for (let index = 0; index < this.config.particleCount; index++) {
      this.particles.push(
        new Particle(
          Math.random() * width,
          Math.random() * height,
          (Math.random() - 0.5) * this.config.particleSpeed,
          (Math.random() - 0.5) * this.config.particleSpeed,
          this.config.particleRadius.min +
            Math.random() * (this.config.particleRadius.max - this.config.particleRadius.min),
          this.config.colors.particle,
          this.config.colors.particleDim
        )
      );
    }
  }

  /**
   * Track viewport and pointer movement used by the visual effect.
   */
  private bindEvents(): void {
    window.addEventListener('resize', () => {
      this.resize();
      this.initParticles();
    });

    window.addEventListener('mousemove', (event) => {
      this.mouse.x = event.clientX;
      this.mouse.y = event.clientY;
    });
  }

  /**
   * Draw the full particle scene for the current animation frame.
   */
  private draw(): void {
    const width = window.innerWidth;
    const height = window.innerHeight;

    this.ctx.clearRect(0, 0, width, height);

    this.particles.forEach((particle) => {
      particle.update(width, height);
      particle.draw(this.ctx);
    });

    this.drawConnections();
    this.drawMouseConnections();
  }

  /**
   * Connect nearby particles with subtle lines for the network effect.
   */
  private drawConnections(): void {
    for (let i = 0; i < this.particles.length; i++) {
      let connections = 0;

      for (let j = i + 1; j < this.particles.length; j++) {
        if (connections >= this.config.maxConnections) {
          break;
        }

        const dx = this.particles[i].x - this.particles[j].x;
        const dy = this.particles[i].y - this.particles[j].y;
        const distance = Math.sqrt(dx * dx + dy * dy);

        if (distance < this.config.connectionDistance) {
          const opacity = (1 - distance / this.config.connectionDistance) * 0.15;

          this.ctx.beginPath();
          this.ctx.strokeStyle = `rgba(0, 240, 255, ${opacity})`;
          this.ctx.lineWidth = 0.5;
          this.ctx.moveTo(this.particles[i].x, this.particles[i].y);
          this.ctx.lineTo(this.particles[j].x, this.particles[j].y);
          this.ctx.stroke();

          connections++;
        }
      }
    }
  }

  /**
   * Add extra interaction between the pointer and nearby particles.
   */
  private drawMouseConnections(): void {
    this.particles.forEach((particle) => {
      const dx = particle.x - this.mouse.x;
      const dy = particle.y - this.mouse.y;
      const distance = Math.sqrt(dx * dx + dy * dy);

      if (distance < this.config.connectionDistance * 1.5) {
        const opacity = (1 - distance / (this.config.connectionDistance * 1.5)) * 0.2;

        this.ctx.beginPath();
        this.ctx.strokeStyle = `rgba(184, 41, 247, ${opacity})`;
        this.ctx.lineWidth = 0.5;
        this.ctx.moveTo(particle.x, particle.y);
        this.ctx.lineTo(this.mouse.x, this.mouse.y);
        this.ctx.stroke();

        const force = (this.config.connectionDistance * 1.5 - distance) / (this.config.connectionDistance * 1.5);
        particle.vx += (dx / distance) * force * 0.01;
        particle.vy += (dy / distance) * force * 0.01;
      }
    });
  }

  private animate = (): void => {
    this.draw();
    this.animationId = requestAnimationFrame(this.animate);
  };

  /**
   * Start the background animation loop if it is not already running.
   */
  start(): void {
    if (!this.animationId) {
      this.animate();
    }
  }

  /**
   * Stop the animation loop so the page can be safely destroyed.
   */
  stop(): void {
    if (this.animationId) {
      cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }
  }

  /**
   * Stop animating and remove the canvas from the document.
   */
  destroy(): void {
    this.stop();
    this.canvas.remove();
  }
}

class Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  radius: number;
  color: string;
  colorDim: string;
  pulsePhase: number;

  constructor(
    x: number,
    y: number,
    vx: number,
    vy: number,
    radius: number,
    color: string,
    colorDim: string
  ) {
    this.x = x;
    this.y = y;
    this.vx = vx;
    this.vy = vy;
    this.radius = radius;
    this.color = color;
    this.colorDim = colorDim;
    this.pulsePhase = Math.random() * Math.PI * 2;
  }

  /**
   * Advance the particle by one frame and keep it inside the viewport.
   */
  update(width: number, height: number): void {
    this.x += this.vx;
    this.y += this.vy;

    if (this.x < 0 || this.x > width) {
      this.vx *= -1;
    }
    if (this.y < 0 || this.y > height) {
      this.vy *= -1;
    }

    this.x = Math.max(0, Math.min(width, this.x));
    this.y = Math.max(0, Math.min(height, this.y));
    this.pulsePhase += 0.02;
  }

  /**
   * Draw the particle glow and its bright core.
   */
  draw(ctx: CanvasRenderingContext2D): void {
    const pulse = Math.sin(this.pulsePhase) * 0.3 + 0.7;
    const radius = this.radius * pulse;

    const gradient = ctx.createRadialGradient(this.x, this.y, 0, this.x, this.y, radius * 3);
    gradient.addColorStop(0, this.color);
    gradient.addColorStop(0.4, this.colorDim);
    gradient.addColorStop(1, 'transparent');

    ctx.beginPath();
    ctx.arc(this.x, this.y, radius * 3, 0, Math.PI * 2);
    ctx.fillStyle = gradient;
    ctx.fill();

    ctx.beginPath();
    ctx.arc(this.x, this.y, radius, 0, Math.PI * 2);
    ctx.fillStyle = this.color;
    ctx.fill();
  }
}
