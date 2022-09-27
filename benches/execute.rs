use criterion::{criterion_group, criterion_main, Criterion};
use worms::scene::{Scene, SceneParameters};

pub fn execute_2000(c: &mut Criterion) {
    let mut pit = Scene::new(
            1000,
             1000,
             SceneParameters {
                worm_size: 8,
                starvation: 5000,
                expiration: 1000,
                body_size: 2.0,
            },
             2000,
             200,
        );

    c.bench_function("execute_2000", |b| b.iter(|| pit.execute()));
}

pub fn execute_200(c: &mut Criterion) {
    let mut pit = Scene::new(
            1000,
             1000,
             SceneParameters {
                worm_size: 8,
                starvation: 5000,
                expiration: 1000,
                body_size: 2.0,
            },
             200,
             20,
        );

    c.bench_function("execute_200", |b| b.iter(|| pit.execute()));
}

criterion_group!(benches, execute_2000, execute_200);
criterion_main!(benches);