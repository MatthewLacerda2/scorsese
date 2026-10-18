//! Why a page shows its slug card, as said by whoever captured it elsewhere.

use std::collections::HashMap;

use scorsese_core::Project;

use crate::report::Note;

/// What a renderer that does not capture noted ([`crate::Renderer::without_capturing`]),
/// as its caller is told it: every note once, with a page that showed its card
/// saying why the capture made ahead of it failed, by the page's path in
/// `failed`, rather than the renderer's "not captured yet". The web app's
/// renders and stills both capture through another container (#778, #1434),
/// and only it knows why a page is not there.
pub fn reasoned(notes: &[Note], project: &Project, failed: &HashMap<String, String>) -> Vec<Note> {
    let mut told: Vec<Note> = Vec::new();
    for note in notes {
        let note = match note {
            Note::PageNotCaptured { clip, asset, .. } => {
                let page = project
                    .assets
                    .iter()
                    .find(|a| a.id.as_str() == asset)
                    .and_then(|a| a.path.as_ref());
                match page.and_then(|path| failed.get(path.as_str())) {
                    Some(why) => Note::PageNotCaptured {
                        clip: clip.clone(),
                        asset: asset.clone(),
                        reason: why.clone(),
                    },
                    None => note.clone(),
                }
            }
            other => other.clone(),
        };
        if !told.contains(&note) {
            told.push(note);
        }
    }
    told
}

#[cfg(test)]
mod tests {
    use scorsese_core::{Asset, AssetId, AssetKind, ProjectPath};

    use super::*;

    #[test]
    fn a_page_left_as_its_card_says_why_it_was_not_captured() {
        let mut project = Project::new("p", scorsese_core::Fps::THIRTY);
        project.assets.push(Asset::imported(
            AssetId::new("title"),
            AssetKind::Html,
            ProjectPath::new("pages/title.html"),
        ));
        let card = |reason: &str| Note::PageNotCaptured {
            clip: "c1".into(),
            asset: "title".into(),
            reason: reason.into(),
        };
        let warned = Note::PageWarning {
            asset: "title".into(),
            warning: "it asked for the internet".into(),
        };
        let failed = HashMap::from([("pages/title.html".to_owned(), "stopped".to_owned())]);
        let notes = [card("not captured yet"), warned.clone(), warned.clone()];

        let told = reasoned(&notes, &project, &failed);

        assert_eq!(told, [card("stopped"), warned]);
        let none = reasoned(&[card("not captured yet")], &project, &HashMap::new());
        assert!(
            none[0].to_string().ends_with("not captured yet"),
            "{none:?}"
        );
    }
}
