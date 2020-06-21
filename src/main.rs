use anyhow::{bail, Context, Result};
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum TagType {
    Emoji,
    Emoticon,
    Hashtag,
}

impl TagType {
    fn as_str(&self) -> &'static str {
        use TagType::*;
        match self {
            Emoji => "Emoji",
            Emoticon => "Emoticon",
            Hashtag => "Hashtag",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum DatasetType {
    IrisData,
    Auxiliary,
}
impl DatasetType {
    fn as_str(&self) -> &'static str {
        match self {
            IrisData => "0",
            Auxiliary => "1",
        }
    }
}

use DatasetType::*;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Tag {
    text: String,
    expression_type: TagType,
    dataset_flag: DatasetType,
}

fn get_tags<'a>(record: &'a StringRecord) -> impl Iterator<Item = Tag> + 'a {
    let tweet_year = &record[0];
    let dataset_flag = if tweet_year.parse::<i32>().unwrap() >= 2018 {
        IrisData
    } else {
        Auxiliary
    };
    // let mex_num = &record[1];
    let emojis = &record[2];
    let emoticons = &record[3];
    let hashtags = &record[4];

    let emoji_tags = emojis.split(" ").map(move |value| Tag {
        text: value.to_owned(),
        expression_type: TagType::Emoji,
        dataset_flag,
    });

    let emoticons_tags = emoticons.split(" ").map(move |value| Tag {
        text: value.to_owned(),
        expression_type: TagType::Emoticon,
        dataset_flag,
    });
    let hashtags_tags = hashtags.split(" ").map(move |value| Tag {
        text: value.to_owned(),
        expression_type: TagType::Hashtag,
        dataset_flag,
    });

    emoji_tags
        .chain(emoticons_tags)
        .chain(hashtags_tags)
        .filter(|e| e.text.len() > 0) // filter out empty strings
}

const min_to_retain_nodes: i64 = 7;
const min_to_retain_edges: i64 = 3;

type NodeMap = IndexMap<Tag, i64, Fnv>;

fn read_nodes(mut csv: CsvReader) -> Result<NodeMap> {
    let mut nodes: IndexMap<Tag, i64, Fnv> =
        IndexMap::with_capacity_and_hasher(1000_000, Fnv::default());

    let mut record = csv::StringRecord::new();

    while csv
        .read_record(&mut record)
        .context("reading record from csv")?
    {
        for mex in get_tags(&record) {
            match nodes.get_mut(&mex) {
                Some(x) => *x += 1,
                None => {
                    nodes.insert(mex, 1);
                }
            }
        }
    }
    Ok(nodes)
}

fn read_edges(mut csv: CsvReader, nodes: &NodeMap) -> Result<FnvHashMap<(usize, usize), i64>> {
    let mut edges = HashMap::with_capacity_and_hasher(1000_000, Fnv::default());
    let mut record = csv::StringRecord::new();
    while csv.read_record(&mut record)? {
        let mexes: Vec<_> = get_tags(&record)
            .filter_map(|mex| nodes.get_index_of(&mex)) // remove nodes that we don't care about
            .collect();

        for (mut a, mut b) in mexes.iter().tuple_combinations() {
            if a > b {
                std::mem::swap(&mut a, &mut b); // ensure same edge order
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

    let nodes = {
        let mut nodes = read_nodes(open_csv(&path)?)?;
        eprintln!("total nodes: {}", nodes.len());

        nodes.retain(|_, v| *v >= min_to_retain_nodes);

        eprintln!("total nodes after filtering: {}", nodes.len());
        nodes
    };

    {
        // convert nodes to vec, then sort, then write nodes to file
        let mut nodes_sorted = nodes.iter().enumerate().collect_vec();

        nodes_sorted.sort_by(|(_, (_, v1)), (_, (_, v2))| v2.cmp(v1));
        let mut f = csv::Writer::from_path("nodes.csv")?;
        // node_id SERIAL PRIMARY KEY,
        // dataset_flag integer NOT NULL,
        // node_type text NOT NULL,
        // node text NOT NULL,
        // weight integer NOT NULL,
        f.write_record(&["node_id", "dataset_flag", "node_type", "node", "weight"])?;
        for (node_id, (tag, weight)) in nodes_sorted {
            f.write_record(&[
                (node_id + 1).to_string().as_str(),
                tag.dataset_flag.as_str(),
                tag.expression_type.as_str(),
                &tag.text,
                &weight.to_string(),
            ])?;
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
        // edge_id BIGSERIAL PRIMARY KEY,
        // dataset_flag integer NOT NULL,
        // node_1 biginteger NOT NULL,
        // node_2 biginteger NOT NULL,
        // weight real NOT NULL,
        f.write_record(&["dataset_flag", "node_1", "node_2", "weight"])?;
        for ((inx_1, inx_2), v) in edges_sorted {
            let (tag, _) = nodes.get_index(inx_1).unwrap();
            let (tag2, _) = nodes.get_index(inx_2).unwrap();

            if tag.dataset_flag != tag2.dataset_flag {
                eprintln!(
                    "dataset_flag is not the same for {} and {}",
                    tag.text, tag2.text
                );
            }

            let dataset_flag = tag.dataset_flag;

            f.write_record(&[
                dataset_flag.as_str(),
                &(inx_1 + 1).to_string(),
                &(inx_2 + 1).to_string(),
                &v.to_string(),
            ])?;
        }
    }

    Ok(())
}
