import * as flatbuffers from 'flatbuffers';
import { EntityDelta } from '../ecs/entity-delta';
export declare class Snapshot {
    bb: flatbuffers.ByteBuffer | null;
    bb_pos: number;
    __init(i: number, bb: flatbuffers.ByteBuffer): Snapshot;
    static getRootAsSnapshot(bb: flatbuffers.ByteBuffer, obj?: Snapshot): Snapshot;
    static getSizePrefixedRootAsSnapshot(bb: flatbuffers.ByteBuffer, obj?: Snapshot): Snapshot;
    entities(index: number, obj?: EntityDelta): EntityDelta | null;
    entitiesLength(): number;
    tick(): bigint;
    static startSnapshot(builder: flatbuffers.Builder): void;
    static addEntities(builder: flatbuffers.Builder, entitiesOffset: flatbuffers.Offset): void;
    static createEntitiesVector(builder: flatbuffers.Builder, data: flatbuffers.Offset[]): flatbuffers.Offset;
    static startEntitiesVector(builder: flatbuffers.Builder, numElems: number): void;
    static addTick(builder: flatbuffers.Builder, tick: bigint): void;
    static endSnapshot(builder: flatbuffers.Builder): flatbuffers.Offset;
    static finishSnapshotBuffer(builder: flatbuffers.Builder, offset: flatbuffers.Offset): void;
    static finishSizePrefixedSnapshotBuffer(builder: flatbuffers.Builder, offset: flatbuffers.Offset): void;
    static createSnapshot(builder: flatbuffers.Builder, entitiesOffset: flatbuffers.Offset, tick: bigint): flatbuffers.Offset;
}
//# sourceMappingURL=snapshot.d.ts.map