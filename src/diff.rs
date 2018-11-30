use difference::{Difference, Changeset};
use colored::*;

pub fn compare(expected: &str, actual: &str) -> String {
    let mut result = String::new();

    let clean_expected = expected.replace("\r\n", "\n");
    let clean_actual = actual.replace("\r\n", "\n");

    let Changeset { diffs, .. } = Changeset::new(&clean_expected, &clean_actual, "\n");

    for i in 0..diffs.len() {
        match diffs[i] {
            Difference::Same(ref x) => {
                result.push_str(x);
                result.push('\n');
            },
            Difference::Add(ref x) => {
                if let Difference::Rem(ref y) = diffs[i - 1] {
                    let Changeset { diffs, .. } = Changeset::new(y, x, " ");
                    for (i, change) in diffs.iter().enumerate() {
                        match change {
                            Difference::Same(ref z) => {
                                result.push_str(&z.green().to_string());
                                if i < diffs.len() - 1 { result.push(' '); }
                            }
                            Difference::Add(ref z) => {
                                result.push_str(&z.white().on_green().to_string());
                                if i < diffs.len() - 1 { result.push(' '); }
                            }
                            _ => (),
                        }
                    }
                    result.push('\n');
                } else {
                    result.push_str(&x.bright_green().to_string());
                    result.push('\n');
                }
            },
            Difference::Rem(ref x) => {
                result.push_str(&x.red().to_string());
                result.push('\n');
            },
        }
    }

    result
}
