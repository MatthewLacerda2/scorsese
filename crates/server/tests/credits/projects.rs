//! A history row and the project it was spent on (#534).

use scorsese_core::{Fps, Project};
use scorsese_server::credits::generations::Answer;
use scorsese_server::credits::history::{self, Filter};
use scorsese_server::projects::store;
use sqlx::postgres::PgPool;

use super::{SHOT_PRICE, account, finish, fund, shot, start};

/// A row names its project while the project exists, and keeps its id once
/// it is deleted: the money moved either way.
#[sqlx::test]
async fn a_row_names_its_project_and_outlives_it(pool: PgPool) {
    let ana = account(&pool, "ana@example.com").await;
    fund(&pool, ana, 10).await;
    let teaser = Project::new("teaser", Fps::default());
    let project = store::create(&pool, ana, &teaser).await.unwrap().id;
    let paid = start(&pool, ana, &shot(Some(project))).await.unwrap();
    finish(&pool, ana, paid, &Answer::Worked(None))
        .await
        .unwrap();

    let named = history::read(&pool, ana, &Filter::default()).await.unwrap();
    assert_eq!(named.rows[0].project_name.as_deref(), Some("teaser"));

    assert!(store::delete(&pool, ana, project).await.unwrap());
    let orphaned = history::read(&pool, ana, &Filter::default()).await.unwrap();
    assert_eq!(orphaned.rows[0].project_id, Some(project));
    assert_eq!(orphaned.rows[0].project_name, None);
    assert_eq!(orphaned.rows[0].amount_micros, -SHOT_PRICE);
}
