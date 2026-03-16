
export interface EntityState {
  id: string;
  position: [number, number, number];
  orientation: number;
  color?: string;
  shape?: string;
  size?: [number, number, number];
  name?: string;
  material_type?: string;
  components?: string;
  volume?: number;
  toxicity?: number;
  degradability?: boolean;
  fat?: number;
  protein?: number;
  carbohydrate?: number;
  water_content?: number;
  fiber_content?: number;
  vitamins?: number;
  trace_elements?: number;
  caloric_value?: number;
  hunger?: number;
  water?: number;
}

export interface JSONSnapshot {
  tick: number;
  entities: Record<string, EntityState>;
}
