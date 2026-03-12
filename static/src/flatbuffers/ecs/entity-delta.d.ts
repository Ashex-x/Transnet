import * as flatbuffers from 'flatbuffers';
export declare class EntityDelta {
    bb: flatbuffers.ByteBuffer | null;
    bb_pos: number;
    __init(i: number, bb: flatbuffers.ByteBuffer): EntityDelta;
    static getRootAsEntityDelta(bb: flatbuffers.ByteBuffer, obj?: EntityDelta): EntityDelta;
    static getSizePrefixedRootAsEntityDelta(bb: flatbuffers.ByteBuffer, obj?: EntityDelta): EntityDelta;
    id(): string | null;
    id(optionalEncoding: flatbuffers.Encoding): string | Uint8Array | null;
    bitmask(): number;
    position(index: number): number | null;
    positionLength(): number;
    positionArray(): Float32Array | null;
    orientation(): number;
    color(): string | null;
    color(optionalEncoding: flatbuffers.Encoding): string | Uint8Array | null;
    shape(): string | null;
    shape(optionalEncoding: flatbuffers.Encoding): string | Uint8Array | null;
    size(index: number): number | null;
    sizeLength(): number;
    sizeArray(): Float32Array | null;
    name(): string | null;
    name(optionalEncoding: flatbuffers.Encoding): string | Uint8Array | null;
    materialType(): string | null;
    materialType(optionalEncoding: flatbuffers.Encoding): string | Uint8Array | null;
    components(): string | null;
    components(optionalEncoding: flatbuffers.Encoding): string | Uint8Array | null;
    volume(): number;
    toxicity(): number;
    degradability(): boolean;
    fat(): number;
    protein(): number;
    carbohydrate(): number;
    waterContent(): number;
    fiberContent(): number;
    vitamins(): number;
    traceElements(): number;
    caloricValue(): number;
    hunger(): number;
    water(): number;
    static startEntityDelta(builder: flatbuffers.Builder): void;
    static addId(builder: flatbuffers.Builder, idOffset: flatbuffers.Offset): void;
    static addBitmask(builder: flatbuffers.Builder, bitmask: number): void;
    static addPosition(builder: flatbuffers.Builder, positionOffset: flatbuffers.Offset): void;
    static createPositionVector(builder: flatbuffers.Builder, data: number[] | Float32Array): flatbuffers.Offset;
    /**
     * @deprecated This Uint8Array overload will be removed in the future.
     */
    static createPositionVector(builder: flatbuffers.Builder, data: number[] | Uint8Array): flatbuffers.Offset;
    static startPositionVector(builder: flatbuffers.Builder, numElems: number): void;
    static addOrientation(builder: flatbuffers.Builder, orientation: number): void;
    static addColor(builder: flatbuffers.Builder, colorOffset: flatbuffers.Offset): void;
    static addShape(builder: flatbuffers.Builder, shapeOffset: flatbuffers.Offset): void;
    static addSize(builder: flatbuffers.Builder, sizeOffset: flatbuffers.Offset): void;
    static createSizeVector(builder: flatbuffers.Builder, data: number[] | Float32Array): flatbuffers.Offset;
    /**
     * @deprecated This Uint8Array overload will be removed in the future.
     */
    static createSizeVector(builder: flatbuffers.Builder, data: number[] | Uint8Array): flatbuffers.Offset;
    static startSizeVector(builder: flatbuffers.Builder, numElems: number): void;
    static addName(builder: flatbuffers.Builder, nameOffset: flatbuffers.Offset): void;
    static addMaterialType(builder: flatbuffers.Builder, materialTypeOffset: flatbuffers.Offset): void;
    static addComponents(builder: flatbuffers.Builder, componentsOffset: flatbuffers.Offset): void;
    static addVolume(builder: flatbuffers.Builder, volume: number): void;
    static addToxicity(builder: flatbuffers.Builder, toxicity: number): void;
    static addDegradability(builder: flatbuffers.Builder, degradability: boolean): void;
    static addFat(builder: flatbuffers.Builder, fat: number): void;
    static addProtein(builder: flatbuffers.Builder, protein: number): void;
    static addCarbohydrate(builder: flatbuffers.Builder, carbohydrate: number): void;
    static addWaterContent(builder: flatbuffers.Builder, waterContent: number): void;
    static addFiberContent(builder: flatbuffers.Builder, fiberContent: number): void;
    static addVitamins(builder: flatbuffers.Builder, vitamins: number): void;
    static addTraceElements(builder: flatbuffers.Builder, traceElements: number): void;
    static addCaloricValue(builder: flatbuffers.Builder, caloricValue: number): void;
    static addHunger(builder: flatbuffers.Builder, hunger: number): void;
    static addWater(builder: flatbuffers.Builder, water: number): void;
    static endEntityDelta(builder: flatbuffers.Builder): flatbuffers.Offset;
    static createEntityDelta(builder: flatbuffers.Builder, idOffset: flatbuffers.Offset, bitmask: number, positionOffset: flatbuffers.Offset, orientation: number, colorOffset: flatbuffers.Offset, shapeOffset: flatbuffers.Offset, sizeOffset: flatbuffers.Offset, nameOffset: flatbuffers.Offset, materialTypeOffset: flatbuffers.Offset, componentsOffset: flatbuffers.Offset, volume: number, toxicity: number, degradability: boolean, fat: number, protein: number, carbohydrate: number, waterContent: number, fiberContent: number, vitamins: number, traceElements: number, caloricValue: number, hunger: number, water: number): flatbuffers.Offset;
}
//# sourceMappingURL=entity-delta.d.ts.map