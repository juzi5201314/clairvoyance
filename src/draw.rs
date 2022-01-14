use byte_unit::{Byte, ByteUnit};
use plotters::prelude::{
    AsRelative, ChartBuilder, Color, IntoDrawingArea, LabelAreaPosition, LineSeries, RGBColor,
    Rectangle, SVGBackend, BLACK, BLUE, GREEN, MAGENTA, RED, WHITE, YELLOW,
};
use std::path::Path;

use crate::data::Data;

pub fn render_memory<P>(data: &[Data], output: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let to_mb = |b: u64| Byte::from(b).get_adjusted_unit(ByteUnit::MB).get_value() as u64;

    let x_len = data.len();
    let y_len = [
        data.iter()
            .map(|data| data.memory.vms)
            .map(to_mb)
            .max()
            .unwrap_or(0),
        data.iter()
            .map(|data| data.memory.rss)
            .map(to_mb)
            .max()
            .unwrap_or(0),
        data.iter()
            .filter_map(|data| data.memory.data)
            .map(to_mb)
            .max()
            .unwrap_or(0),
        data.iter()
            .filter_map(|data| data.memory.text)
            .map(to_mb)
            .max()
            .unwrap_or(0),
        data.iter()
            .filter_map(|data| data.memory.shared)
            .map(to_mb)
            .max()
            .unwrap_or(0),
    ]
    .iter()
    .max()
    .cloned()
    .unwrap_or(0);
    let root = SVGBackend::new(&output, (1920, 1080)).into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, (8).percent())
        .set_label_area_size(LabelAreaPosition::Bottom, (4).percent())
        .caption("Memory Usage", ("sans-serif", (5).percent_height()))
        .margin((1).percent())
        .build_cartesian_2d(0..x_len, 0..y_len)?;

    chart.configure_mesh().y_desc("MB").draw()?;

    let elems: Vec<(&'static str, RGBColor, Box<dyn Fn(&Data) -> u64>)> = vec![
        ("vms", GREEN, box |d: &Data| d.memory.vms),
        ("rss", RED, box |d: &Data| d.memory.rss),
        ("shared", YELLOW, box |d: &Data| {
            d.memory.shared.unwrap_or(0)
        }),
        ("text", BLUE, box |d: &Data| d.memory.text.unwrap_or(0)),
        ("data", MAGENTA, box |d: &Data| d.memory.data.unwrap_or(0)),
    ];

    for (label, color, elem) in elems {
        chart
            .draw_series(LineSeries::new(
                data.iter()
                    .enumerate()
                    .map(|(x, data)| (x, to_mb(elem(data)))),
                color.stroke_width(3),
            ))?
            .label(label)
            .legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 10, y + 5)], color.filled()));
    }

    chart.configure_series_labels().border_style(BLACK).draw()?;

    root.present()?;

    drop(chart);
    drop(root);

    Ok(())
}

pub fn render_cpu_time<P>(data: &[Data], output: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let x_len = data.len();
    let y_len = [
        data.iter()
            .map(|data| data.cpu_time.system as u64)
            .max()
            .unwrap_or(0),
        data.iter()
            .map(|data| data.cpu_time.user as u64)
            .max()
            .unwrap_or(0),
    ]
    .iter()
    .max()
    .cloned()
    .unwrap_or(0);
    let root = SVGBackend::new(&output, (1920, 1080)).into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, (8).percent())
        .set_label_area_size(LabelAreaPosition::Bottom, (4).percent())
        .caption("Cpu Time", ("sans-serif", (5).percent_height()))
        .margin((1).percent())
        .build_cartesian_2d(0..x_len, 0..y_len)?;

    chart.configure_mesh().y_desc("us").draw()?;

    let elems: Vec<(&'static str, RGBColor, Box<dyn Fn(&Data) -> u64>)> = vec![
        ("system", GREEN, box |d: &Data| d.cpu_time.system as u64),
        ("user", RED, box |d: &Data| d.cpu_time.user as u64),
    ];

    for (label, color, elem) in elems {
        chart
            .draw_series(LineSeries::new(
                data.iter().enumerate().map(|(x, data)| (x, elem(data))),
                color.stroke_width(3),
            ))?
            .label(label)
            .legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 10, y + 5)], color.filled()));
    }

    chart.configure_series_labels().border_style(BLACK).draw()?;

    root.present()?;

    drop(chart);
    drop(root);

    Ok(())
}

pub fn render_cpu_usage<P>(data: &[Data], output: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let x_len = data.len();
    let y_len = data
        .iter()
        .map(|data| data.cpu_usage.0.round() as u64)
        .max()
        .unwrap_or(0);
    let root = SVGBackend::new(&output, (1920, 1080)).into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, (8).percent())
        .set_label_area_size(LabelAreaPosition::Bottom, (4).percent())
        .caption("Cpu Usage", ("sans-serif", (5).percent_height()))
        .margin((1).percent())
        .build_cartesian_2d(0..x_len, 0..y_len)?;

    chart.configure_mesh().y_desc("%").draw()?;

    let elems: Vec<(&'static str, RGBColor, Box<dyn Fn(&Data) -> u64>)> =
        vec![("usage", GREEN, box |d: &Data| d.cpu_usage.0.ceil() as u64)];

    for (label, color, elem) in elems {
        chart
            .draw_series(LineSeries::new(
                data.iter().enumerate().map(|(x, data)| (x, elem(data))),
                color.stroke_width(3),
            ))?
            .label(label)
            .legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 10, y + 5)], color.filled()));
    }

    chart.configure_series_labels().border_style(BLACK).draw()?;

    root.present()?;

    drop(chart);
    drop(root);

    Ok(())
}

pub fn render_io<P>(data: &[Data], output: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let to_mb = |b: u64| Byte::from(b).get_adjusted_unit(ByteUnit::MB).get_value() as u64;

    let x_len = data.len();
    let y_len = [
        data.iter()
            .map(|data| data.io.bytes_written)
            .map(to_mb)
            .max()
            .unwrap_or(0),
        data.iter()
            .map(|data| data.io.bytes_read)
            .map(to_mb)
            .max()
            .unwrap_or(0),
        data.iter()
            .filter_map(|data| data.io.disk_written)
            .map(to_mb)
            .max()
            .unwrap_or(0),
        data.iter()
            .filter_map(|data| data.io.disk_read)
            .map(to_mb)
            .max()
            .unwrap_or(0),
        data.iter()
            .filter_map(|data| data.io.syscall_written)
            .map(to_mb)
            .max()
            .unwrap_or(0),
        data.iter()
            .filter_map(|data| data.io.syscall_read)
            .map(to_mb)
            .max()
            .unwrap_or(0),
    ]
    .iter()
    .max()
    .cloned()
    .unwrap_or(0);
    let root = SVGBackend::new(&output, (1920, 1080)).into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, (8).percent())
        .set_label_area_size(LabelAreaPosition::Bottom, (4).percent())
        .caption("I/O", ("sans-serif", (5).percent_height()))
        .margin((1).percent())
        .build_cartesian_2d(0..x_len, 0..y_len)?;

    chart.configure_mesh().y_desc("MB").draw()?;

    let elems: Vec<(&'static str, RGBColor, Box<dyn Fn(&Data) -> u64>)> = vec![
        ("bytes_written", GREEN, box |d: &Data| d.io.bytes_written),
        ("bytes_read", GREEN, box |d: &Data| d.io.bytes_read),
        ("disk_written", GREEN, box |d: &Data| {
            d.io.disk_written.unwrap_or_default()
        }),
        ("disk_read", GREEN, box |d: &Data| {
            d.io.disk_read.unwrap_or_default()
        }),
        ("syscall_written", GREEN, box |d: &Data| {
            d.io.syscall_written.unwrap_or_default()
        }),
        ("syscall_read", GREEN, box |d: &Data| {
            d.io.syscall_read.unwrap_or_default()
        }),
    ];

    for (label, color, elem) in elems {
        chart
            .draw_series(LineSeries::new(
                data.iter()
                    .enumerate()
                    .map(|(x, data)| (x, to_mb(elem(data)))),
                color.stroke_width(3),
            ))?
            .label(label)
            .legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 10, y + 5)], color.filled()));
    }

    chart.configure_series_labels().border_style(BLACK).draw()?;

    root.present()?;

    drop(chart);
    drop(root);

    Ok(())
}
