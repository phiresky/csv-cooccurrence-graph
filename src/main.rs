use anyhow::{bail, Context, Result};
use csv::StringRecord;
use fnv::{FnvBuildHasher as Fnv, FnvHashMap};
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressBarWrap, ProgressStyle};
use itertools::Itertools;
use std::fs::File;
use std::{collections::HashMap, io::BufReader};
use structopt::StructOpt;

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

#[derive(Clone, PartialEq, Eq, Hash)]
struct Tag {
    text: String,
    expression_type: TagType,
}

fn debugemoji(str: &str) {
    println!("debug emoji {} ({:?}):", str, str);
    for char in str.chars() {
        println!(
            "debug emoji part: {:?}: {:?}",
            char,
            unic_ucd::Name::of(char)
        );
    }
}

fn load_emoticon_emoji_mapping() -> HashMap<String, String> {
    let emoticon_emoji_map = include_str!("../emoticon_emoji_mapping.json");
    let emoticon_emoji_map =
        serde_json::from_str(emoticon_emoji_map).expect("Parsing of map json failed");

    return emoticon_emoji_map;
}

fn replace_emoticon_by_emoji(
    emoticon: &str,
    emoticon_emoji_map: &HashMap<String, String>,
) -> Option<Tag> {
    return match emoticon_emoji_map.get(emoticon) {
        Some(emoji) => Some(Tag {
            text: emoji.to_owned(),
            expression_type: TagType::Emoji,
        }),
        None => {
            println!("Emoticon {} has no corresponding emoji", emoticon);
            None
        }
    };
}

fn is_char_interesting(char: &char) -> bool {
    let info = unic_ucd::GeneralCategory::of(*char);
    use unic_ucd::GeneralCategory::*;
    /* if char == '\u{200d}' {
        println!("right of zws: {:?}", emoji.chars().skip(i).join(""));
    }*/

    match info {
        /*Format if char == '\u{200d}' => {},
        Format  => {
            println!("format {} emojis char {:?} unknown:", emoji, char);
            debugemoji(&emoji);
        }*/
        OtherSymbol | OtherPunctuation | MathSymbol | DashPunctuation | LowercaseLetter
        | Format => true,
        NonspacingMark | ModifierSymbol | ModifierLetter | EnclosingMark | SpacingMark
        | Unassigned | OtherLetter => {
            // non spacing marks and modifier symbols are normal modifiers like skin tone
            // 🦳 = unassigned
            false
        }
        _ => {
            /*println!(
                "info of {}th char of emoji '{}': {:?}: {:?}",
                i, emoji, char, info
            );*/
            // debugemoji(&emoji);
            panic!("what dis {:?}: {:?}", char, info);
        }
    }
}

fn clean_emoji(emoji: &str) -> Option<String> {
    if emoji.len() == 0 {
        return None;
    }
    let emoji = emoji
        .replace("\u{200d}♂\u{fe0f}", "")
        .replace("\u{200d}♀\u{fe0f}", "");
    // println!("chars in emoji {}:", emoji);
    let emoji = emoji.chars().filter(is_char_interesting).join("");
    if emoji.len() == 0 {
        None
    } else {
        Some(emoji.to_string())
    }
}

fn get_tags<'a>(
    record: &'a StringRecord,
    emoticon_emoji_map: &HashMap<String, String>,
    replace_emoticons_and_ignore_hashtags: bool,
) -> impl Iterator<Item = Tag> + 'a {
    let tweet_year = &record[0];

    // let mex_num = &record[1];
    let emojis = &record[2];
    let emoticons = &record[3];
    let hashtags = &record[4];

    let emoji_tags = emojis
        .split(" ")
        .filter_map(clean_emoji)
        .map(move |value| Tag {
            text: value,
            expression_type: TagType::Emoji,
        });

    let emoticons_tags = emoticons
        .split(" ")
        .filter(|x| !x.is_empty())
        .map(move |value| Tag {
            text: value.to_owned(),
            expression_type: TagType::Emoticon,
        });

    let emoticons: Vec<Tag> = if replace_emoticons_and_ignore_hashtags {
        emoticons_tags
            .filter_map(|x| replace_emoticon_by_emoji(&x.text, &emoticon_emoji_map))
            .collect()
    } else {
        emoticons_tags.collect()
    };

    let hashtags_tags: Vec<Tag> = if replace_emoticons_and_ignore_hashtags {
        hashtags
            .split(" ")
            .map(move |value| Tag {
                text: value.to_owned(),
                expression_type: TagType::Hashtag,
            })
            .collect()
    } else {
        Vec::new()
    };

    emoji_tags
        .chain(emoticons)
        .chain(hashtags_tags)
        .filter(|e| e.text.len() > 0) // filter out empty strings
}

const min_to_retain_nodes: i64 = 7;
const min_to_retain_edges: i64 = 3;

type NodeMap = IndexMap<Tag, i64, Fnv>;

fn read_nodes(
    mut csv: CsvReader,
    emoticon_emoji_map: &HashMap<String, String>,
    replace_emoticons_and_ignore_hashtags: bool,
) -> Result<NodeMap> {
    let mut nodes: IndexMap<Tag, i64, Fnv> =
        IndexMap::with_capacity_and_hasher(1000_000, Fnv::default());

    let mut record = csv::StringRecord::new();

    while csv
        .read_record(&mut record)
        .context("reading record from csv")?
    {
        for mex in get_tags(
            &record,
            &emoticon_emoji_map,
            replace_emoticons_and_ignore_hashtags,
        ) {
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

fn read_edges(
    mut csv: CsvReader,
    nodes: &NodeMap,
    emoticon_emoji_map: &HashMap<String, String>,
    replace_emoticons_and_ignore_hashtags: bool,
) -> Result<FnvHashMap<(usize, usize), i64>> {
    let mut edges = HashMap::with_capacity_and_hasher(1000_000, Fnv::default());
    let mut record = csv::StringRecord::new();
    while csv.read_record(&mut record)? {
        let mexes: Vec<_> = get_tags(
            &record,
            &emoticon_emoji_map,
            replace_emoticons_and_ignore_hashtags,
        )
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

#[derive(StructOpt)]
struct Args {
    /// The input csv path
    path: String,
    #[structopt(short, long)]
    /// If true, replace emoticons with a somewhat corresponding emoji, and remove all hashtags
    replace_emoticons_and_ignore_hashtags: bool,
}

fn main() -> Result<()> {
    let emoticon_emoji_map = load_emoticon_emoji_mapping();

    let args = Args::from_args();

    let nodes = {
        let mut nodes = read_nodes(
            open_csv(&args.path)?,
            &emoticon_emoji_map,
            args.replace_emoticons_and_ignore_hashtags,
        )?;
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
        f.write_record(&["node_id", "node_type", "node", "weight"])?;
        for (node_id, (tag, weight)) in nodes_sorted {
            f.write_record(&[
                (node_id + 1).to_string().as_str(),
                tag.expression_type.as_str(),
                &tag.text,
                &weight.to_string(),
            ])?;
        }
    }

    let mut edges = read_edges(
        open_csv(&args.path)?,
        &nodes,
        &emoticon_emoji_map,
        args.replace_emoticons_and_ignore_hashtags,
    )?;

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
        f.write_record(&["node_1", "node_2", "weight"])?;
        for ((inx_1, inx_2), v) in edges_sorted {
            let (tag, _) = nodes.get_index(inx_1).unwrap();
            let (tag2, _) = nodes.get_index(inx_2).unwrap();

            f.write_record(&[
                &(inx_1 + 1).to_string(),
                &(inx_2 + 1).to_string(),
                &v.to_string(),
            ])?;
        }
    }

    Ok(())
}
