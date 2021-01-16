use cpython::{py_fn, py_module_initializer, PyResult, Python, PyErr};

py_module_initializer!(emojirustfast, |py, m| {
    m.add(py, "__doc__", "This module is implemented in Rust.")?;
    m.add(py, "clean_emoji", py_fn!(py, clean_emoji_py(emoji: &str)))?;
    Ok(())
});

pub fn clean_emoji(emoji: &str) -> Result<Option<String>, String> {
    if emoji.len() == 0 {
        return Ok(None);
    }
    let emoji = emoji
        .replace("\u{200d}♂\u{fe0f}", "")
        .replace("\u{200d}♀\u{fe0f}", "");
    // println!("chars in emoji {}:", emoji);
    let emoji = emoji.chars().filter_map(|char| match is_char_interesting(&char) {
        Ok(true) => Some(Ok(char)),
        Ok(false) => None,
        Err(e) => Some(Err(e))
    }).collect::<Result<String, _>>()?;
    if emoji.len() == 0 {
        Ok(None)
    } else {
        Ok(Some(emoji.to_string()))
    }
}

pub fn clean_emoji_py(py: Python, emoji: &str) -> PyResult<Option<String>> {
    clean_emoji(emoji).map_err(|s| PyErr::new::<cpython::exc::TypeError, _>(py, s))
}

pub fn is_char_interesting(char: &char) -> Result<bool, String> {
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
        | Format => Ok(true),
        NonspacingMark | ModifierSymbol | ModifierLetter | EnclosingMark | SpacingMark
        | Unassigned | OtherLetter | PrivateUse | SpaceSeparator | UppercaseLetter | DecimalNumber => {
            // non spacing marks and modifier symbols are normal modifiers like skin tone
            // 🦳 = unassigned
            Ok(false)
        }
        _ => {
            /*println!(
                "info of {}th char of emoji '{}': {:?}: {:?}",
                i, emoji, char, info
            );*/
            // debugemoji(&emoji);
            eprintln!("what dis {:?}: {:?}", char, info);
            Ok(false)
            //Err(format!("what dis {:?}: {:?}", char, info))
        }
    }
}
