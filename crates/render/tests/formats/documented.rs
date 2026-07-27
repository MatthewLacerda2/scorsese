//! The page and the code, held to each other.
//!
//! `docs/output-formats.md` is where the accepted set is decided, and an agent
//! reading it is deciding what to render into. A row it promises that the code
//! refuses is a capability nobody can reach; a combination the code writes that
//! the page never mentions is one nobody knows about. Neither fails anything
//! else, which is why it is checked here.

use scorsese_render::Container;

const PAGE: &str = include_str!("../../../../docs/output-formats.md");

/// Matched literally: renaming the header fails this test rather than quietly
/// finding no rows and passing.
const TABLE: &str = "| container | extension | video | audio |";

/// One row of the page's table, cells already stripped of their backticks.
struct Row {
    container: String,
    extension: String,
    video: Vec<String>,
    audio: Vec<String>,
}

fn rows() -> Vec<Row> {
    let table = PAGE
        .split_once(TABLE)
        .unwrap_or_else(|| panic!("no output-format table (`{TABLE}`) on the page"))
        .1;
    table
        .lines()
        .skip(1)
        .skip_while(|line| line.starts_with("| ---"))
        .take_while(|line| line.starts_with('|'))
        .map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let names = |cell: &str| {
                cell.split(',')
                    .map(|name| name.trim().trim_matches('`').to_owned())
                    .collect()
            };
            Row {
                container: cells[1].trim_matches('`').to_owned(),
                extension: cells[2].trim_matches('`').to_owned(),
                video: names(cells[3]),
                audio: names(cells[4]),
            }
        })
        .collect()
}

fn listed<T: ToString>(codecs: &[T]) -> Vec<String> {
    codecs.iter().map(ToString::to_string).collect()
}

#[test]
fn the_page_lists_every_container_and_no_others() {
    let documented: Vec<String> = rows().into_iter().map(|row| row.container).collect();
    let written: Vec<String> = Container::ALL
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        documented, written,
        "the page's containers, in order, are not what the code writes"
    );
}

/// Order included, because the page tells a reader the first codec listed is
/// the default. A row in the wrong order is a wrong promise, not a formatting
/// nit.
#[test]
fn every_row_says_what_that_container_is_written_with() {
    for row in rows() {
        let container: Container = row
            .container
            .parse()
            .unwrap_or_else(|_| panic!("`{}` is not a container", row.container));
        assert_eq!(
            row.extension,
            format!(".{container}"),
            "{container}'s extension"
        );
        assert_eq!(
            row.video,
            listed(container.video_codecs()),
            "{container}'s video codecs, default first"
        );
        assert_eq!(
            row.audio,
            listed(container.audio_codecs()),
            "{container}'s audio codecs, default first"
        );
    }
}
