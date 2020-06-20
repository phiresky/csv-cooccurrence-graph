use anyhow::{Context, Result};
use csv::StringRecord;
use fnv::{FnvBuildHasher as Fnv, FnvHashMap};
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressBarWrap, ProgressStyle};
use itertools::Itertools;
use std::fs::File;
use std::{collections::HashMap, io::BufReader};

type CsvReader = csv::Reader<BufReader<ProgressBarWrap<File>>>;

fn open_csv(path: &str) -> Result<CsvReader> {
    let inp = File::open(path)?;
    let progress_style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta_precise})");
    let bar = ProgressBar::new(inp.metadata()?.len()).with_style(progress_style);
    Ok(csv::Reader::from_reader(BufReader::with_capacity(
        10_000_000, // 10MB buffer so progress bar doesn't update as often
        bar.wrap_read(inp),
    )))
}

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
        .filter(|e| e.len() > 0) // filter out empty strings
}

const min_to_retain_nodes: i64 = 10;
const min_to_retain_edges: i64 = 3;

fn read_nodes(mut csv: CsvReader) -> Result<IndexMap<String, i64, Fnv>> {
    let mut nodes = IndexMap::with_capacity_and_hasher(1000_000, Fnv::default());

    let mut record = csv::StringRecord::new();
    while csv.read_record(&mut record)? {
        for mex in get_metaexpressions(&record) {
            match nodes.get_mut(mex) {
                Some(x) => *x += 1,
                None => {
                    nodes.insert(mex.to_string(), 1);
                }
            }
        }
    }
    Ok(nodes)
}

fn read_edges(
    mut csv: CsvReader,
    nodes: &IndexMap<String, i64, Fnv>,
) -> Result<FnvHashMap<(usize, usize), i64>> {
    let mut edges = HashMap::with_capacity_and_hasher(1000_000, Fnv::default());
    let mut record = csv::StringRecord::new();
    while csv.read_record(&mut record)? {
        let mexes: Vec<_> = get_metaexpressions(&record)
            .filter_map(|mex| nodes.get_index_of(mex))
            .collect();

        for (mut a, mut b) in mexes.iter().tuple_combinations() {
            if a > b { // ensure same edge order
                std::mem::swap(&mut a, &mut b);
            }
            *(edges.entry((*a, *b)).or_default()) += 1;
        }
    }
    Ok(edges)
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .next()
        .context("Supply csv as first argument")?;
    let mut nodes = read_nodes(open_csv(&path)?)?;
    eprintln!("");
    eprintln!("total nodes: {}", nodes.len());
    nodes.retain(|_, v| *v >= min_to_retain_nodes);
    eprintln!("total nodes after filtering: {}", nodes.len());

    {
        // convert nodes to vec, then sort, then write nodes to file
        let mut nodes_sorted = nodes.iter().collect_vec();
        nodes_sorted.sort_by(|(_, v1), (_, v2)| v2.cmp(v1));
        let mut f = csv::Writer::from_path("nodes.csv")?;
        f.write_record(&["node", "weight"])?;
        for (k, v) in nodes_sorted {
            f.write_record(&[k, &v.to_string()])?;
        }
    }

    let mut edges = read_edges(open_csv(&path)?, &nodes)?;

    eprintln!("total edges: {}", edges.len());
    edges.retain(|_, v| *v >= min_to_retain_edges);
    eprintln!("total edges after filtering: {}", edges.len());

    {
        // convert edges to vec, then sort, then write to file
        let mut edges_sorted = edges.into_iter().collect_vec();
        edges_sorted.sort_by(|(_, v1), (_, v2)| v2.cmp(v1));

        let mut f = csv::Writer::from_path("edges.csv")?;
        f.write_record(&["node_1", "node_2", "weight"])?;
        for ((inx_1, inx_2), v) in edges_sorted {
            f.write_record(&[
                nodes.get_index(inx_1).unwrap().0,
                nodes.get_index(inx_2).unwrap().0,
                &v.to_string(),
            ])?;
        }
    }

    Ok(())
}
