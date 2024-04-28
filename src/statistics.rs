use std::{collections::HashMap, fs, time::Duration};

use bevy::{app::AppExit, prelude::*};
use itertools::Itertools;

pub struct StatisticsPlugin;

impl Plugin for StatisticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Statistics>()
            .add_systems(Last, write_statistics.run_if(is_exiting));
    }
}

#[derive(Resource, Default)]
pub struct Statistics(pub HashMap<&'static str, Vec<Duration>>);

impl Statistics {
    pub fn add(&mut self, name: &'static str, duration: Duration) {
        self.0.entry(name).or_default().push(duration);
    }

    pub fn last_mut(&mut self, name: &'static str) -> Option<&mut Duration> {
        self.0.get_mut(name)?.last_mut()
    }
}

pub fn is_exiting(mut exit: EventReader<AppExit>) -> bool {
    exit.read().next().is_some()
}

pub fn mean(a: &[Duration]) -> Duration {
    a.iter().sum::<Duration>() / a.len() as u32
}

pub fn median(a: &[Duration]) -> Duration {
    let mut a = a.to_vec();
    a.sort();
    if a.len() % 2 == 0 {
        (a[a.len() / 2] + a[a.len() / 2 - 1]) / 2
    } else {
        a[a.len() / 2]
    }
}

pub fn std(a: &[Duration]) -> Duration {
    let mean = mean(a).as_secs_f64();
    let variance = a
        .iter()
        .map(|d| (d.as_secs_f64() - mean).powi(2))
        .sum::<f64>()
        / a.len() as f64;
    Duration::from_secs_f64(variance.sqrt())
}

pub fn as_ms(d: Duration) -> String {
    format!("{:.5} ms", d.as_secs_f64() * 1000.)
}

fn print_stats(stats: Res<Statistics>) {
    for k in stats.0.keys().sorted() {
        let v = &stats.0[k];
        println!(
            "{:16 }: mean {: <11}, std {: <11}, median {: <11}",
            k,
            as_ms(mean(v)),
            as_ms(std(v)),
            as_ms(median(v)),
        );
    }
}

fn write_statistics(stats: Res<Statistics>) {
    let stats_f64 = stats
        .0
        .iter()
        .map(|(k, v)| (*k, v.iter().map(|d| d.as_secs_f64()).collect_vec()))
        .collect::<HashMap<_, _>>();

    fs::write(
        "statistics.json",
        serde_json::to_string(&stats_f64).unwrap(),
    )
    .unwrap();

    print_stats(stats);

    plot_stats(stats_f64).unwrap();
}

use plotters::{prelude::*, style::Color};

const COLORS: [RGBColor; 6] = [
    RGBColor(11, 132, 165),
    RGBColor(111, 78, 124),
    RGBColor(157, 216, 102),
    RGBColor(202, 71, 47),
    RGBColor(255, 160, 86),
    RGBColor(141, 221, 208),
];

fn plot_stats(stats: HashMap<&str, Vec<f64>>) -> anyhow::Result<()> {
    let root = BitMapBackend::new("plot.png", (800, 480)).into_drawing_area();
    root.fill(&WHITE)?;

    let width = stats.iter().next().unwrap().1.len();

    let max = *stats
        .values()
        .map(|v| v.iter().max_by(|a, b| a.total_cmp(b)).unwrap())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();

    let min = *stats
        .values()
        .map(|v| v.iter().min_by(|a, b| a.total_cmp(b)).unwrap())
        .min_by(|a, b| a.total_cmp(b))
        .unwrap();

    let mut chart = ChartBuilder::on(&root)
        .caption("Stats", ("sans-serif", 20).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0..width, min..max)?;

    chart.configure_mesh().draw()?;

    for (i, (name, values)) in stats.iter().sorted_by_key(|(k, _)| *k).enumerate() {
        let color = COLORS[i];
        chart
            .draw_series(LineSeries::new(
                values.iter().enumerate().map(|(i, v)| (i, *v)),
                color,
            ))?
            .label(*name)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));
    }

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}
