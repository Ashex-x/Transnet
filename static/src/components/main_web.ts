/**
 * main_web.ts is for main web design.
 * There is only one button: Enter MetaLand.
 */

import { router } from '../router';

export class MainWeb {
  private dots: HTMLDivElement[];

  constructor() {
    this.dots = [];
  }

  render(): void {
    this.addFloatKeyframes();
    this.createMainWebPage();
    this.createFloatingDots();
    this.setupEventListeners();
  }

  private addFloatKeyframes(): void {
    const styleSheet = document.createElement('style');
    styleSheet.textContent = `
      @keyframes float {
        0%, 100% {
          transform: translateY(0px) translateX(0px);
        }
        25% {
          transform: translateY(-20px) translateX(10px);
        }
        50% {
          transform: translateY(-10px) translateX(-10px);
        }
        75% {
          transform: translateY(-30px) translateX(5px);
        }
      }
      @keyframes bounce {
        0%, 100% {
          transform: translateY(0);
        }
        50% {
          transform: translateY(-10px);
        }
      }
    `;
    document.head.appendChild(styleSheet);
  }

  private createMainWebPage(): void {
    const mainWeb = document.createElement('div');
    mainWeb.id = 'main-web';
    mainWeb.className = 'page active';

    const scrollContainer = document.createElement('div');
    scrollContainer.id = 'scroll-container';
    scrollContainer.className = 'scroll-snap-container';

    const animaSection = document.createElement('div');
    animaSection.id = 'anima-section';
    animaSection.className = 'scroll-section';

    const dotsContainer = document.createElement('div');
    dotsContainer.id = 'dots-container';

    const content = document.createElement('div');
    content.className = 'content';

    const title = document.createElement('h1');
    title.id = 'title';
    title.textContent = 'Anima';

    const desc = document.createElement('p');
    desc.id = 'desc';
    desc.textContent = 'This world is called Anima. It is a living digital companion that perceives and grows within its own physical world, METALAND. Unlike static AI, she evolves her personality and memories through every shared experience, creating a truly unique and lifelike bond.';

    const enterButton = document.createElement('button');
    enterButton.id = 'enter-button';
    enterButton.innerHTML = 'Enter Metaland';

    const buttonDots = document.createElement('div');
    buttonDots.className = 'button-dots';
    enterButton.appendChild(buttonDots);

    const scrollArrow = document.createElement('div');
    scrollArrow.id = 'scroll-arrow';
    scrollArrow.innerHTML = '↓';

    content.appendChild(title);
    content.appendChild(desc);
    content.appendChild(enterButton);

    animaSection.appendChild(dotsContainer);
    animaSection.appendChild(content);
    animaSection.appendChild(scrollArrow);

    const transnetSection = document.createElement('div');
    transnetSection.id = 'transnet-section';
    transnetSection.className = 'scroll-section transnet-section';

    const transnetContent = document.createElement('div');
    transnetContent.className = 'content';

    const transnetTitle = document.createElement('h1');
    transnetTitle.id = 'transnet-title';
    transnetTitle.textContent = 'Transnet';

    const transnetDesc = document.createElement('p');
    transnetDesc.id = 'transnet-desc';
    transnetDesc.textContent = 'Transnet is a web application for AI-worlds net translation. It bridges different AI universes, enabling seamless communication and understanding across various artificial worlds and their unique languages, protocols, and semantic structures.';

    const transnetButton = document.createElement('button');
    transnetButton.id = 'transnet-button';
    transnetButton.textContent = 'Enter Transnet';

    transnetContent.appendChild(transnetTitle);
    transnetContent.appendChild(transnetDesc);
    transnetContent.appendChild(transnetButton);

    transnetSection.appendChild(transnetContent);

    scrollContainer.appendChild(animaSection);
    scrollContainer.appendChild(transnetSection);

    mainWeb.appendChild(scrollContainer);

    document.body.appendChild(mainWeb);
  }

  private createFloatingDots(): void { // Background floating dots
    const dotsContainer = document.getElementById('dots-container');
    if (!dotsContainer) return;
    
    const dotCount = 256;

    for (let i = 0; i < dotCount; i++) {
      const dot = document.createElement('div');
      dot.className = 'dot';
      
      const size = Math.random() * 10 + 5;
      dot.style.width = `${size}px`;
      dot.style.height = `${size}px`;
      dot.style.left = `${Math.random() * 100}%`;
      dot.style.top = `${Math.random() * 100}%`;
      dot.style.animationDelay = `${Math.random() * 6}s`;
      dot.style.animationDuration = `${Math.random() * 4 + 4}s`;
      dot.style.animationIterationCount = 'infinite';

      dotsContainer.appendChild(dot);
      this.dots.push(dot);
    }
  }

  private setupEventListeners(): void {
    const enterButton = document.getElementById('enter-button');
    if (enterButton) {
      enterButton.addEventListener('click', () => {
        this.onEnterMetaland();
      });
    }

    const transnetButton = document.getElementById('transnet-button');
    if (transnetButton) {
      transnetButton.addEventListener('click', () => {
        this.onEnterTransnet();
      });
    }
  }

  private onEnterMetaland(): void { // Enter Metaland
    console.log('Entering Metaland...');
    router.navigate('/metaland');
  }

  private onEnterTransnet(): void { // Enter Transnet
    console.log('Entering Transnet...');
    router.navigate('/transnet');
  }

  destroy(): void {
    const mainWeb = document.getElementById('main-web');
    if (mainWeb) {
      mainWeb.remove();
    }
    this.dots = [];
  }
}