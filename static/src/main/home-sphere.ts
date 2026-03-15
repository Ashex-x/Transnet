/**
 * Interactive landing-page sphere used on `/home`.
 *
 * The sphere is a canvas-based illusion: nodes are distributed across a virtual
 * 3D surface, projected onto 2D, and connected when they are close in space.
 */

import { router } from '../router';

interface SphereNode {
  kind: 'normal' | 'metaland' | 'transnet';
  baseTheta: number;
  basePhi: number;
  driftPhase: number;
  driftSpeed: number;
  driftThetaAmplitude: number;
  driftPhiAmplitude: number;
  radius: number;
  label?: string;
  description?: string;
  href?: string;
}

interface ProjectedNode extends SphereNode {
  x: number;
  y: number;
  z: number;
  screenX: number;
  screenY: number;
  alpha: number;
  scale: number;
  renderRadius: number;
}

export class HomeSphere {
  private host: HTMLDivElement;
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private nodes: SphereNode[];
  private projectedNodes: ProjectedNode[] = [];
  private animationId: number | null = null;
  private rotationX = -0.28;
  private rotationY = 0.52;
  private velocityX = 0.0015;
  private velocityY = -0.0022;
  private isDragging = false;
  private lastPointer = { x: 0, y: 0 };
  private hoveredNode: ProjectedNode | null = null;
  private size = { width: 0, height: 0, radius: 0 };
  private readonly connectionThreshold = 140;
  private readonly perspective = 900;
  private readonly resizeHandler = () => this.resize();
  private readonly contextMenuHandler = (event: MouseEvent) => {
    event.preventDefault();
  };
  private readonly pointerDownHandler = (event: PointerEvent) => this.handlePointerDown(event);
  private readonly pointerMoveHandler = (event: PointerEvent) => this.handlePointerMove(event);
  private readonly pointerUpHandler = () => this.handlePointerUp();
  private readonly clickHandler = () => this.handleClick();
  private readonly leaveHandler = () => {
    this.hoveredNode = null;
    this.canvas.style.cursor = 'grab';
  };

  constructor() {
    this.host = document.createElement('div');
    this.host.className = 'home-sphere';

    this.canvas = document.createElement('canvas');
    this.canvas.className = 'home-sphere__canvas';

    const ctx = this.canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Could not get home sphere canvas context');
    }

    this.ctx = ctx;
    this.host.appendChild(this.canvas);
    this.nodes = this.createNodes();
  }

  /**
   * Mount the sphere into its page slot and start the render loop.
   */
  mount(parent: HTMLElement): void {
    parent.appendChild(this.host);
    this.bindEvents();
    this.resize();
    this.start();
  }

  /**
   * Remove listeners, stop animation, and detach the canvas tree.
   */
  destroy(): void {
    if (this.animationId !== null) {
      window.cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }

    window.removeEventListener('resize', this.resizeHandler);
    this.canvas.removeEventListener('contextmenu', this.contextMenuHandler);
    this.canvas.removeEventListener('pointerdown', this.pointerDownHandler);
    window.removeEventListener('pointermove', this.pointerMoveHandler);
    window.removeEventListener('pointerup', this.pointerUpHandler);
    this.canvas.removeEventListener('click', this.clickHandler);
    this.canvas.removeEventListener('pointerleave', this.leaveHandler);
    this.host.remove();
  }

  /**
   * Bind pointer and resize handlers used by the sphere interaction model.
   */
  private bindEvents(): void {
    window.addEventListener('resize', this.resizeHandler);
    this.canvas.addEventListener('contextmenu', this.contextMenuHandler);
    this.canvas.addEventListener('pointerdown', this.pointerDownHandler);
    window.addEventListener('pointermove', this.pointerMoveHandler);
    window.addEventListener('pointerup', this.pointerUpHandler);
    this.canvas.addEventListener('click', this.clickHandler);
    this.canvas.addEventListener('pointerleave', this.leaveHandler);
  }

  /**
   * Start the animation loop for the rotating sphere.
   */
  private start(): void {
    const renderFrame = (timestamp: number) => {
      this.update(timestamp);
      this.draw();
      this.animationId = window.requestAnimationFrame(renderFrame);
    };

    this.animationId = window.requestAnimationFrame(renderFrame);
  }

  /**
   * Size the canvas and choose a sphere radius based on the viewport slot.
   */
  private resize(): void {
    const rect = this.host.getBoundingClientRect();
    const width = Math.max(rect.width, 320);
    const height = Math.max(rect.height, 320);
    const dpr = window.devicePixelRatio || 1;

    this.size.width = width;
    this.size.height = height;
    this.size.radius = Math.min(width, height) * 0.28;

    this.canvas.width = Math.floor(width * dpr);
    this.canvas.height = Math.floor(height * dpr);
    this.canvas.style.width = `${width}px`;
    this.canvas.style.height = `${height}px`;
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  /**
   * Advance the sphere rotation and project the 3D nodes for the frame.
   */
  private update(timestamp: number): void {
    if (!this.isDragging) {
      this.rotationX += this.velocityX;
      this.rotationY += this.velocityY;
    }

    this.velocityX *= 0.992;
    this.velocityY *= 0.992;

    if (Math.abs(this.velocityX) < 0.0002) {
      this.velocityX = Math.sign(this.velocityX || 1) * 0.0002;
    }

    if (Math.abs(this.velocityY) < 0.0002) {
      this.velocityY = Math.sign(this.velocityY || -1) * 0.0002;
    }

    this.projectedNodes = this.nodes.map((node, index) => this.projectNode(node, timestamp, index));
  }

  /**
   * Draw the full scene for the current frame.
   */
  private draw(): void {
    this.ctx.clearRect(0, 0, this.size.width, this.size.height);
    this.drawConnections();

    const sortedNodes = [...this.projectedNodes].sort((a, b) => a.z - b.z);
    for (const node of sortedNodes) {
      this.drawNode(node);
    }
  }

  /**
   * Connect nearby nodes after they have been projected into the current frame.
   */
  private drawConnections(): void {
    for (let i = 0; i < this.projectedNodes.length; i++) {
      for (let j = i + 1; j < this.projectedNodes.length; j++) {
        const a = this.projectedNodes[i];
        const b = this.projectedNodes[j];
        const dx = a.x - b.x;
        const dy = a.y - b.y;
        const dz = a.z - b.z;
        const distance = Math.sqrt(dx * dx + dy * dy + dz * dz);

        if (distance > this.connectionThreshold) {
          continue;
        }

        const proximity = 1 - distance / this.connectionThreshold;
        const alpha = Math.min(a.alpha, b.alpha) * proximity * 0.3;
        this.ctx.strokeStyle = `rgba(255, 255, 255, ${alpha})`;
        this.ctx.lineWidth = 0.7 + proximity * 0.7;
        this.ctx.beginPath();
        this.ctx.moveTo(a.screenX, a.screenY);
        this.ctx.lineTo(b.screenX, b.screenY);
        this.ctx.stroke();
      }
    }
  }

  /**
   * Draw a node, and for special nodes, draw the depth-aware label and hint.
   */
  private drawNode(node: ProjectedNode): void {
    const glow = node.kind === 'normal' ? 10 : 18;
    const baseAlpha = node.kind === 'normal' ? node.alpha : Math.min(node.alpha + 0.08, 1);

    this.ctx.save();
    this.ctx.fillStyle = `rgba(255, 255, 255, ${baseAlpha})`;
    this.ctx.shadowColor = `rgba(255, 255, 255, ${baseAlpha})`;
    this.ctx.shadowBlur = glow;
    this.ctx.beginPath();
    this.ctx.arc(node.screenX, node.screenY, node.renderRadius, 0, Math.PI * 2);
    this.ctx.fill();
    this.ctx.restore();

    if (node.kind === 'normal' || !node.label) {
      return;
    }

    const depthRatio = (node.alpha - 0.18) / 0.82;
    const titleSize = 15 + depthRatio * 9;
    const titleAlpha = Math.max(0.15, node.alpha);
    const isHovered = this.hoveredNode?.kind === node.kind;

    this.ctx.save();
    this.ctx.textAlign = 'center';
    this.ctx.font = `600 ${titleSize}px Inter, sans-serif`;
    this.ctx.fillStyle = `rgba(255, 255, 255, ${titleAlpha})`;
    this.ctx.fillText(node.label, node.screenX, node.screenY - node.renderRadius - 16);

    if (isHovered && node.description) {
      const descriptionSize = Math.max(11, titleSize * 0.56);
      this.ctx.font = `400 ${descriptionSize}px Inter, sans-serif`;
      this.ctx.fillStyle = `rgba(255, 255, 255, ${titleAlpha * 0.92})`;
      this.ctx.fillText(node.description, node.screenX, node.screenY + node.renderRadius + 22);
    }

    this.ctx.restore();
  }

  /**
   * Convert a node's spherical position into the current projected screen point.
   */
  private projectNode(node: SphereNode, timestamp: number, index: number): ProjectedNode {
    const time = timestamp * 0.001;
    const theta = node.baseTheta + Math.sin(time * node.driftSpeed + node.driftPhase) * node.driftThetaAmplitude;
    const phi = this.clampPhi(
      node.basePhi + Math.cos(time * (node.driftSpeed * 0.85) + node.driftPhase * 0.9) * node.driftPhiAmplitude
    );

    const radius = this.size.radius;
    const x = radius * Math.sin(phi) * Math.cos(theta);
    const y = radius * Math.cos(phi);
    const z = radius * Math.sin(phi) * Math.sin(theta);

    const rotatedY = this.rotateY(x, y, z, this.rotationY);
    const rotatedX = this.rotateX(rotatedY.x, rotatedY.y, rotatedY.z, this.rotationX);
    const scale = this.perspective / (this.perspective - rotatedX.z);
    const depth = (rotatedX.z / radius + 1) / 2;
    const alpha = 0.14 + depth * 0.86;

    return {
      ...node,
      x: rotatedX.x,
      y: rotatedX.y,
      z: rotatedX.z,
      screenX: this.size.width / 2 + rotatedX.x * scale,
      screenY: this.size.height / 2 + rotatedX.y * scale,
      scale,
      alpha,
      renderRadius: node.radius * (0.8 + scale * 0.34) + (node.kind === 'normal' ? 0 : 1.6) + (index % 2) * 0.05,
    };
  }

  /**
   * Create the regular and special nodes that define the sphere layout.
   */
  private createNodes(): SphereNode[] {
    const specialNodes: SphereNode[] = [
      {
        kind: 'metaland',
        baseTheta: Math.PI * 0.18,
        basePhi: Math.PI * 0.42,
        driftPhase: 0.35,
        driftSpeed: 0.8,
        driftThetaAmplitude: 0.035,
        driftPhiAmplitude: 0.028,
        radius: 4.2,
        label: 'Metaland',
        description: 'Enter the living digital world',
        href: '/metaland',
      },
      {
        kind: 'transnet',
        baseTheta: Math.PI * 1.22,
        basePhi: Math.PI * 0.62,
        driftPhase: 1.9,
        driftSpeed: 0.76,
        driftThetaAmplitude: 0.035,
        driftPhiAmplitude: 0.028,
        radius: 4.2,
        label: 'Transnet',
        description: 'Open the AI translation bridge',
        href: '/transnet',
      },
    ];

    const normals: SphereNode[] = [];
    const count = 26;
    const goldenAngle = Math.PI * (3 - Math.sqrt(5));

    for (let i = 0; i < count; i++) {
      const ratio = (i + 0.5) / count;
      const phi = Math.acos(1 - 2 * ratio);
      const theta = goldenAngle * i;

      normals.push({
        kind: 'normal',
        baseTheta: theta,
        basePhi: phi,
        driftPhase: i * 0.37,
        driftSpeed: 0.55 + (i % 7) * 0.05,
        driftThetaAmplitude: 0.018 + (i % 3) * 0.004,
        driftPhiAmplitude: 0.014 + (i % 4) * 0.003,
        radius: 1.9 + (i % 4) * 0.25,
      });
    }

    return [...normals, ...specialNodes];
  }

  /**
   * Start rotating only on right-click drag, per the interaction spec.
   */
  private handlePointerDown(event: PointerEvent): void {
    if (event.button !== 2) {
      return;
    }

    event.preventDefault();
    this.isDragging = true;
    this.lastPointer = { x: event.clientX, y: event.clientY };
    this.canvas.style.cursor = 'grabbing';
  }

  /**
   * Update drag-based rotation or hover state, depending on pointer mode.
   */
  private handlePointerMove(event: PointerEvent): void {
    const rect = this.canvas.getBoundingClientRect();
    const pointerX = event.clientX - rect.left;
    const pointerY = event.clientY - rect.top;

    if (this.isDragging) {
      const deltaX = this.lastPointer.x - event.clientX;
      const deltaY = this.lastPointer.y - event.clientY;
      this.lastPointer = { x: event.clientX, y: event.clientY };
      this.rotationY += deltaX * 0.008;
      this.rotationX += deltaY * 0.008;
      this.velocityY = deltaX * 0.0004;
      this.velocityX = deltaY * 0.0004;
      return;
    }

    this.hoveredNode = this.findHoveredSpecialNode(pointerX, pointerY);
    this.canvas.style.cursor = this.hoveredNode ? 'pointer' : 'grab';
  }

  /**
   * End dragging while preserving the inertial velocity of the sphere.
   */
  private handlePointerUp(): void {
    this.isDragging = false;
    this.canvas.style.cursor = this.hoveredNode ? 'pointer' : 'grab';
  }

  /**
   * Navigate when a special node is clicked.
   */
  private handleClick(): void {
    if (this.hoveredNode?.href) {
      router.navigate(this.hoveredNode.href);
    }
  }

  /**
   * Detect whether the pointer is over one of the special navigable nodes.
   */
  private findHoveredSpecialNode(pointerX: number, pointerY: number): ProjectedNode | null {
    const candidates = this.projectedNodes
      .filter((node) => node.kind !== 'normal')
      .sort((a, b) => b.z - a.z);

    for (const node of candidates) {
      const dx = pointerX - node.screenX;
      const dy = pointerY - node.screenY;
      const hitRadius = Math.max(18, node.renderRadius + 12);
      if (Math.sqrt(dx * dx + dy * dy) <= hitRadius) {
        return node;
      }
    }

    return null;
  }

  /**
   * Rotate a 3D point around the vertical axis.
   */
  private rotateY(x: number, y: number, z: number, angle: number): { x: number; y: number; z: number } {
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    return {
      x: x * cos - z * sin,
      y,
      z: x * sin + z * cos,
    };
  }

  /**
   * Rotate a 3D point around the horizontal axis.
   */
  private rotateX(x: number, y: number, z: number, angle: number): { x: number; y: number; z: number } {
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    return {
      x,
      y: y * cos - z * sin,
      z: y * sin + z * cos,
    };
  }

  /**
   * Keep spherical coordinates away from the poles to avoid visual clustering.
   */
  private clampPhi(value: number): number {
    const minPhi = 0.12;
    const maxPhi = Math.PI - 0.12;
    return Math.min(maxPhi, Math.max(minPhi, value));
  }
}
