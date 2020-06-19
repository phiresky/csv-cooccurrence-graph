use anyhow::Result;
use csv::StringRecord;
use fnv::FnvHashMap;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::{collections::HashMap, fs::File};

/*#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;*/

fn get_metaexpressions<'a>(record: &'a StringRecord) -> impl Iterator<Item = &'a str> {
    // let tweet_year = &record[0];
    // let mex_num = &record[1];
    let emojis = &record[2];
    let emoticons = &record[3];
    let hashtags = &record[4];
    emojis
        .split(" ")
        .chain(emoticons.split(" "))
        .chain(hashtags.split(" "))
}

const total: i64 = 100_000_000;
const prog_chunk: i64 = 1000_000;

fn read_nodes(path: &str, style: &ProgressStyle) -> Result<FnvHashMap<String, i64>> {
    let mut nodes: HashMap<String, i64, _> =
        FnvHashMap::with_capacity_and_hasher(1000_000, Default::default());

    let mut reader = csv::Reader::from_path(path)?;
    let bar = ProgressBar::new((total / prog_chunk) as u64).with_style(style.clone());
    let mut record = csv::StringRecord::new();
    let mut c: i64 = 0;
    while reader.read_record(&mut record)? {
        c += 1;
        let mexes = get_metaexpressions(&record);
        for mex in mexes {
            let r = nodes.get_mut(mex);
            if let Some(x) = r {
                *x += 1;
            } else {
                nodes.insert(mex.to_string(), 1);
            }
        }

        if c % prog_chunk == 0 {
            bar.inc(1);
        }
    }
    bar.finish();
    Ok(nodes)
}

fn read_edges(
    path: &str,
    nodes: HashMap<String, i64, FnvBuildHasher>,
    style: &ProgressStyle,
) -> Result<FnvHashMap<(String, String), i64>> {
    let mut edges: HashMap<(String, String), i64, _> =
        FnvHashMap::with_capacity_and_hasher(1000_000, Default::default());
    let bar = ProgressBar::new((total / prog_chunk) as u64).with_style(style.clone());
    let mut reader = csv::Reader::from_path(&path)?;
    let mut record = csv::StringRecord::new();
    let mut c: i64 = 0;
    while reader.read_record(&mut record)? {
        c += 1;
        let mexes: Vec<_> = get_metaexpressions(&record)
            .filter(|mex| nodes.contains_key(*mex))
            .collect();
        for (inx, mut a) in mexes.iter().enumerate() {
            for mut b in &mexes[(inx + 1)..] {
                if a > b {
                    std::mem::swap(&mut a, &mut b);
                }
                let k = (a.to_string(), b.to_string());
                let r = edges.get_mut(&k);
                if let Some(x) = r {
                    *x += 1;
                } else {
                    edges.insert(k, 1);
                }
            }
        }

        if c % prog_chunk == 0 {
            bar.inc(1);
        }
    }
    bar.finish();
    Ok(edges)
}

fn main() -> Result<()> {
    let style = ProgressStyle::default_bar().template(
        "[{elapsed_precise} - ETA {eta_precise}] {bar:40.cyan/blue} {pos}/{len} million {msg}",
    );
    let path = "/home/tehdog/tmp/iris/meta-expressions-100m.csv";
    let mut nodes = read_nodes(path, &style)?;
    eprintln!("total nodes: {}", nodes.len());
    nodes.retain(|_, v| *v >= 10);
    eprintln!("total nodes after filtering: {}", nodes.len());

    let mut f = File::create("nodes.txt")?;
    for (k, v) in &nodes {
        writeln!(&mut f, "{}\t{}", k, v)?;
    }

    let mut edges = read_edges(path, nodes, &style)?;

    eprintln!("total edges: {}", edges.len());
    edges.retain(|_, v| *v >= 10);
    eprintln!("total edges after filtering: {}", edges.len());

    let mut f = File::create("edges.txt")?;
    for (k, v) in &edges {
        writeln!(&mut f, "{}\t{}\t{}", k.0, k.1, v)?;
    }

    Ok(())
}
