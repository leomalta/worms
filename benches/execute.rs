use criterion::{
    criterion_group, criterion_main, measurement::WallTime, AxisScale, BenchmarkGroup, BenchmarkId,
    Criterion, PlotConfiguration,
};
use egui::pos2;
use std::time::Duration;
use worms::{
    gui::SimInterface,
    scene::{Scene, SceneParameters},
};

fn get_bench_group<'a>(
    c: &'a mut Criterion,
    name: &'a str,
    size: usize,
) -> BenchmarkGroup<'a, WallTime> {
    let mut group = c.benchmark_group(name);
    group
        .significance_level(0.1)
        .sample_size(size)
        .measurement_time(Duration::from_secs(10))
        // .warm_up_time(Duration::from_nanos(1))
        .plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
    group
}

fn get_scene_2000() -> Scene {
    Scene::new(
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
    )
}

pub fn execute_2000(c: &mut Criterion) {
    let mut group = get_bench_group(c, "execute_2000", 500);
    for i in 0..10 {
        let mut scene = get_scene_2000();
        group.bench_function(BenchmarkId::from_parameter(i), |b| {
            b.iter(|| scene.execute())
        });
    }
    group.finish();
}

pub fn print_2000(c: &mut Criterion) {
    let mut group = get_bench_group(c, "print_2000", 500);
    for i in 0..10 {
        let interface = SimInterface::from(get_scene_2000());
        let pos = pos2(50. * i as f32, -50. * i as f32);
        group.bench_with_input(BenchmarkId::from_parameter(i), &pos, |b, &pos| {
            b.iter(|| interface.get_shapes(pos))
        });
    }
    group.finish();
}

criterion_group!(benches, execute_2000, print_2000);
criterion_main!(benches);
