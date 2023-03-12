#[cfg(feature = "color")]
use colored::*;
use similar::{Change, ChangeTag, TextDiff};

pub fn compare(expected: &str, actual: &str) -> String {
    let mut result = String::new();

    let clean_expected = expected.replace("\r\n", "\n");
    let clean_actual = actual.replace("\r\n", "\n");

    let mut last: Option<Change<_>> = None;
    for diff in TextDiff::from_lines(&clean_expected, &clean_actual).iter_all_changes() {
        let x = diff.value();
        match diff.tag() {
            ChangeTag::Equal => {
                result.push_str(x);
            }
            ChangeTag::Insert => {
                if let Some((y, ChangeTag::Delete)) = last.map(|d| (d.value(), d.tag())) {
                    for change in TextDiff::from_words(y, x).iter_all_changes() {
                        match change.tag() {
                            ChangeTag::Equal => {
                                let z = change.value();
                                #[cfg(feature = "color")]
                                #[allow(clippy::unnecessary_to_owned)]
                                result.push_str(&z.green().to_string());
                                #[cfg(not(feature = "color"))]
                                result.push_str(z);
                            }
                            ChangeTag::Insert => {
                                let z = change.value();
                                #[cfg(feature = "color")]
                                #[allow(clippy::unnecessary_to_owned)]
                                result.push_str(&z.black().on_green().to_string());
                                #[cfg(not(feature = "color"))]
                                result.push_str(z);
                            }
                            _ => (),
                        }
                    }
                } else {
                    #[cfg(feature = "color")]
                    #[allow(clippy::unnecessary_to_owned)]
                    result.push_str(&x.bright_green().to_string());
                    #[cfg(not(feature = "color"))]
                    result.push_str(x);
                }
            }
            ChangeTag::Delete => {
                #[cfg(feature = "color")]
                #[allow(clippy::unnecessary_to_owned)]
                result.push_str(&x.red().to_string());
                #[cfg(not(feature = "color"))]
                result.push_str(x);
            }
        }

        last = Some(diff);
    }

    result.push('\n');

    result
}
