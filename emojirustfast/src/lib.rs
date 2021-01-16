use cpython::{py_fn, py_module_initializer, PyResult, Python};
use itertools::Itertools;

py_module_initializer!(emojirustfast, |py, m| {
    m.add(py, "__doc__", "This module is implemented in Rust.")?;
    m.add(py, "clean_emoji", py_fn!(py, clean_emoji_py(emoji: &str)))?;
    Ok(())
});

pub fn clean_emoji(emoji: &str) -> Option<String> {
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

pub fn clean_emoji_py(_: Python, emoji: &str) -> PyResult<Option<String>> {
    Ok(clean_emoji(emoji))
}

pub fn is_char_interesting(char: &char) -> bool {
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
