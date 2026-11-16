//! The report's layout, from visitors made here.

use super::*;

fn seen(version: &str, hour: i32, prompted: bool, email: Option<&str>) -> Seen {
    Seen {
        test: "heading".into(),
        version: version.into(),
        hour,
        place: Some("BR-SP".into()),
        person: true,
        ended: true,
        prompted,
        seconds: 30,
        email: email.map(Into::into),
        paid: false,
    }
}

#[test]
fn the_report_names_each_version_its_shares_and_the_verdict() {
    let mut all = vec![
        seen("a", 9, true, Some("ana@padaria.com.br")),
        seen("a", 20, false, Some("bia@gmail.com")),
    ];
    all.extend((0..48).map(|_| seen("a", 9, false, None)));
    all.extend((0..50).map(|_| seen("b", 9, false, None)));
    let report = render(&all);
    assert!(report.starts_with("test heading\n"), "{report}");
    let a = report
        .lines()
        .find(|line| line.trim_start().starts_with("a "))
        .unwrap();
    let words: Vec<&str> = a.split_whitespace().collect();
    assert_eq!(
        words,
        ["a", "50", "100%", "30", "2%", "4%", "0%", "50%"],
        "{a}"
    );
    assert!(report.contains("verdict: not ready: every version needs 50 more visitors"));
    assert!(report.contains("07:00-18:00  a 49 visitors"), "{report}");
    assert!(report.contains("18:00-07:00  a 1 visitors, prompt 0%, last section 100% (thin)"));
}

#[test]
fn a_load_no_person_used_is_left_out_and_counted_apart() {
    let mut bot = seen("a", 9, true, None);
    bot.person = false;
    let report = render(&[bot.clone(), bot.clone(), seen("b", 9, false, None)]);
    assert!(
        report.starts_with("test heading\n  left out: 2 loads the page never saw a person use\n"),
        "{report}"
    );
    assert!(!report.contains("\n  a "), "{report}");
    assert!(report.contains("needs 49 more visitors"), "{report}");
    assert_eq!(
        render(&[bot]),
        "test heading\n  left out: 1 load the page never saw a person use\n  no visitor counted yet\n"
    );
}

#[test]
fn nothing_seen_says_so() {
    assert_eq!(render(&[]), "no landing test has had a visitor yet\n");
}

#[test]
fn a_median_takes_the_lower_middle() {
    assert_eq!(median(vec![5, 1, 3]), "3");
    assert_eq!(median(vec![4, 1]), "1");
    assert_eq!(median(vec![]), "-");
}
