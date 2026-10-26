//! A loudness target (#990): what the web's render takes in place of the
//! CLI's `--loudness`, and the key a render asked for at one is kept under.

use scorsese_server::renders::{Ask, Settings, key};

use super::card;

#[test]
fn a_loudness_target_is_kept_in_the_key_and_none_is_not() {
    let project = card(|_| {});
    let balanced = Settings::from_ask(&Ask::default()).unwrap();
    assert_eq!(balanced.loudness, None);
    // Absent from the stored form: every render asked for before the choice
    // existed keeps its key.
    let stored = serde_json::to_value(&balanced).unwrap();
    assert!(stored.get("loudness").is_none(), "{stored}");

    let loud = |lufs| {
        Settings::from_ask(&Ask {
            loudness: Some(lufs),
            ..Ask::default()
        })
    };
    let fourteen = loud(-14.0).unwrap();
    assert_eq!(fourteen.loudness, Some(-14.0));
    assert!(fourteen.render(&project).is_ok());
    let base = key(&project, &balanced).unwrap();
    assert_ne!(key(&project, &fourteen).unwrap(), base);
    assert_ne!(
        key(&project, &loud(-16.0).unwrap()).unwrap(),
        key(&project, &fourteen).unwrap()
    );
    let back: Settings = serde_json::from_value(serde_json::to_value(&fourteen).unwrap()).unwrap();
    assert_eq!(back, fourteen, "a stored row reads back as it was asked");

    // Refused in the CLI's and the stdio tool's words, and a sound-only file
    // takes a target as well as a picture does.
    assert!(loud(14.0).unwrap_err().contains("did you mean -14"));
    assert!(loud(-50.0).unwrap_err().contains("outside -40 to -5 LUFS"));
    let sound = Settings::from_ask(&Ask {
        loudness: Some(-16.0),
        container: Some("wav".into()),
        ..Ask::default()
    });
    assert_eq!(sound.unwrap().loudness, Some(-16.0));
}
