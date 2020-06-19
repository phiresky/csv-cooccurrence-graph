use anyhow::Result;
use csv::StringRecord;
use fnv::FnvHashMap;
use indicatif::{ProgressBar, ProgressBarWrap, ProgressStyle};
use std::io::{BufReader, Write};
use std::{collections::HashMap, fs::File};

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
        .filter(|e| e.len() > 0)
}

const min_to_retain_nodes: i64 = 10;
const min_to_retain_edges: i64 = 3;

type CsvReader = csv::Reader<BufReader<ProgressBarWrap<File>>>;

fn open_csv(path: &str) -> Result<CsvReader> {
    let inp = File::open(path)?;
    let progress_style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta_precise})");
    let bar = ProgressBar::new(inp.metadata()?.len()).with_style(progress_style);
    Ok(csv::Reader::from_reader(BufReader::with_capacity(
        10_000_000,
        bar.wrap_read(inp),
    )))
}

fn read_nodes(mut csv: CsvReader) -> Result<FnvHashMap<String, i64>> {
    let mut nodes: HashMap<String, i64, _> =
        FnvHashMap::with_capacity_and_hasher(1000_000, Default::default());

    //let bar = ProgressBar::new((total / prog_chunk) as u64)
    let mut record = csv::StringRecord::new();
    while csv.read_record(&mut record)? {
        let mexes = get_metaexpressions(&record);
        for mex in mexes {
            let r = nodes.get_mut(mex);
            if let Some(x) = r {
                *x += 1;
            } else {
                nodes.insert(mex.to_string(), 1);
            }
        }
    }
    Ok(nodes)
}

fn read_edges(
    mut csv: CsvReader,
    nodes: FnvHashMap<String, i64>,
) -> Result<FnvHashMap<(String, String), i64>> {
    let mut edges: HashMap<(String, String), i64, _> =
        FnvHashMap::with_capacity_and_hasher(1000_000, Default::default());
    let mut record = csv::StringRecord::new();
    while csv.read_record(&mut record)? {
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
    }
    Ok(edges)
}

fn main() -> Result<()> {
    let path = "/home/tehdog/tmp/iris/meta-expressions-100m.csv";
    let mut nodes = read_nodes(open_csv(&path)?)?;
    eprintln!("total nodes: {}", nodes.len());
    nodes.retain(|_, v| *v >= min_to_retain_nodes);
    eprintln!("total nodes after filtering: {}", nodes.len());

    let mut f = File::create("nodes.tsv")?;
    for (k, v) in &nodes {
        writeln!(&mut f, "{}\t{}", k, v)?;
    }

    let mut edges = read_edges(open_csv(&path)?, nodes)?;

    eprintln!("total edges: {}", edges.len());
    edges.retain(|_, v| *v >= min_to_retain_edges);
    eprintln!("total edges after filtering: {}", edges.len());

    let mut f = File::create("edges.tsv")?;
    for (k, v) in &edges {
        writeln!(&mut f, "{}\t{}\t{}", k.0, k.1, v)?;
    }

    Ok(())
}
