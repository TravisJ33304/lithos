use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use lithos_protocol::types::{EntityId, EntitySnapshot, SnapshotEntityType, Vec2, ZoneId};
use lithos_protocol::{ServerMessage, codec};

fn state_snapshot(entity_count: u64) -> ServerMessage {
    let entities: Vec<EntitySnapshot> = (0..entity_count)
        .map(|i| EntitySnapshot {
            id: EntityId(i),
            position: Vec2::new(i as f32 * 0.1, -(i as f32) * 0.1),
            velocity: Vec2::new(1.0, -1.0),
            zone: ZoneId::Overworld,
            entity_type: SnapshotEntityType::Hostile,
        })
        .collect();
    ServerMessage::StateSnapshot {
        tick: 10_000,
        last_processed_seq: 99,
        entities,
    }
}

fn bench_encode_state_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_state_snapshot_compact");
    for n in [32_u64, 128, 512] {
        let msg = state_snapshot(n);
        group.throughput(Throughput::Elements(n));
        group.bench_with_input(BenchmarkId::from_parameter(n), &msg, |b, m| {
            b.iter(|| {
                let v = codec::encode(black_box(m)).unwrap();
                black_box(v.len())
            });
        });
    }
    group.finish();
}

fn bench_fanout_clone(c: &mut Criterion) {
    let msg = state_snapshot(512);
    let encoded = codec::encode(&msg).unwrap();
    let shared = Bytes::from(encoded.clone());
    let mut group = c.benchmark_group("fanout_100_clients");
    group.throughput(Throughput::Elements(100));
    group.bench_function("vec_clone_per_send", |b| {
        b.iter(|| {
            let mut acc = 0u8;
            for _ in 0..100 {
                let c = encoded.clone();
                acc ^= c.first().copied().unwrap_or(0) ^ c.last().copied().unwrap_or(0);
            }
            black_box(acc);
        });
    });
    group.bench_function("bytes_clone_per_send", |b| {
        b.iter(|| {
            let mut acc = 0u8;
            for _ in 0..100 {
                let c = shared.clone();
                acc ^= c.first().copied().unwrap_or(0) ^ c.last().copied().unwrap_or(0);
            }
            black_box(acc);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_encode_state_snapshot, bench_fanout_clone);
criterion_main!(benches);
