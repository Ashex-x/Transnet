/**
 * house.ts renders ECS (Entity Component System) data for house zone.
 * It uses ECSClient to stream real-time entity updates.
 */

import { ECSClient } from '../ecs';

export class House {
  private ecsClient: ECSClient;

  constructor() {
    this.ecsClient = new ECSClient();
  }

  async init(): Promise<void> {
    await this.ecsClient.init();
  }

  destroy(): void {
    this.ecsClient.destroy();
  }

}
