//! How much one account may keep (`Storage::quota`): its library, its kept
//! renders and the uploads it has announced, checked when an upload is
//! announced and again as its bytes arrive — and abandoned uploads swept for
//! everybody, not only for whoever announces next.

use scorsese_core::Project;
use scorsese_server::db::UserId;
use scorsese_server::projects;
use scorsese_server::storage::DEFAULT_QUOTA;
use sqlx::postgres::PgPool;

use crate::common::{self, request, send};
use crate::library::{announce, announce_as, member, patch, video};

/// The largest file an upload may be: 20 GB.
const LARGEST: usize = 20_000_000_000;

/// A hash made of one repeated hex digit.
fn hash(digit: char) -> String {
    digit.to_string().repeat(64)
}

/// Put a kept render of `size` bytes in `user`'s name, in a project of its
/// own.
async fn render_of(pool: &PgPool, user: UserId, size: i64) {
    let document = Project::new("Intro", Default::default());
    let project = projects::create(pool, user, &document)
        .await
        .expect("the test setup holds")
        .id;
    sqlx::query(
        "INSERT INTO renders (user_id, project_id, key, settings, path, size)
         VALUES ($1, $2, $3, '{}', 'renders/kept.mp4', $4)",
    )
    .bind(user.get())
    .bind(project)
    .bind(hash('e'))
    .bind(size)
    .execute(pool)
    .await
    .expect("the test setup holds");
}

#[sqlx::test]
async fn an_announcement_past_the_quota_is_refused_counting_what_is_under_way(pool: PgPool) {
    let (address, _) = common::serve_with(pool.clone(), common::files("quota-announce")).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let quota = usize::try_from(DEFAULT_QUOTA.get()).unwrap();
    let mut started = Vec::new();
    let mut announced = 0;
    for digit in ['a', 'b', 'c', 'd'] {
        let length = LARGEST.min(quota - announced);
        if length == 0 {
            break;
        }
        let created = announce_as(address, &who, "long.mp4", &hash(digit), length).await;
        assert_eq!(created.status, 201, "{}", created.body);
        started.push(created.header("location").unwrap().to_owned());
        announced += length;
    }

    // Nothing has arrived, and still the quota is spoken for.
    let refused = announce_as(address, &who, "more.mp4", &hash('f'), 1).await;
    assert_eq!(refused.status, 413, "{}", refused.body);
    assert!(
        refused.body.contains("your storage is full"),
        "{}",
        refused.body
    );

    // Somebody else's quota is their own.
    let (_, bia) = member(&pool, "bia@example.com").await;
    let hers = announce_as(address, &bia, "long.mp4", &hash('a'), LARGEST).await;
    assert_eq!(hers.status, 201, "{}", hers.body);

    // Abandoning an upload gives its room back; a kept render takes some.
    let cancelled = send(
        address,
        "DELETE",
        &started[0],
        &[&who, "Tus-Resumable: 1.0.0"],
        &[],
    )
    .await;
    assert_eq!(cancelled.status, 204, "{}", cancelled.body);
    render_of(&pool, ana, 1_000).await;
    let room = LARGEST.min(quota) - 1_000;
    let fits = announce_as(address, &who, "more.mp4", &hash('f'), room).await;
    assert_eq!(fits.status, 201, "{}", fits.body);
    let over = announce_as(address, &who, "one more.mp4", &hash('9'), 1).await;
    assert_eq!(over.status, 413, "{}", over.body);
}

#[sqlx::test]
async fn bytes_arriving_past_the_quota_are_refused(pool: PgPool) {
    let bytes = video(&common::scratch::dir("quota-arrive-media"));
    let (address, _) = common::serve_with(pool.clone(), common::files("quota-arrive")).await;
    let (ana, who) = member(&pool, "ana@example.com").await;
    let created = announce(address, &who, "intro.mp4", &bytes).await;
    assert_eq!(created.status, 201, "{}", created.body);
    let location = created.header("location").unwrap().to_owned();

    // Meanwhile something else filled the account: a generated file, say.
    let full = i64::try_from(DEFAULT_QUOTA.get()).unwrap() - i64::try_from(bytes.len()).unwrap();
    sqlx::query(
        "INSERT INTO library_items (user_id, sha256, name, kind, extension, size_bytes, media)
         VALUES ($1, $2, 'made.mp4', 'video', 'mp4', $3, '{}')",
    )
    .bind(ana.get())
    .bind(hash('c'))
    .bind(full + 1)
    .execute(&pool)
    .await
    .unwrap();

    let refused = patch(address, &who, &location, 0, &bytes).await;
    assert_eq!(refused.status, 413, "{}", refused.body);
    assert!(
        refused.body.contains("your storage is full"),
        "{}",
        refused.body
    );
    let listed = request(address, "GET", "/api/library", &[&who], None).await;
    assert_eq!(
        listed.json().as_array().unwrap().len(),
        1,
        "only the made file"
    );
}

#[sqlx::test]
async fn abandoned_uploads_are_swept_for_everybody(pool: PgPool) {
    let bytes = video(&common::scratch::dir("quota-sweep-media"));
    let files = common::files("quota-sweep");
    let storage = files.storage.clone();
    let (address, state) = common::serve_with(pool.clone(), files).await;
    let (ana, ana_who) = member(&pool, "ana@example.com").await;
    let (_, bia_who) = member(&pool, "bia@example.com").await;

    let old = announce(address, &ana_who, "old.mp4", &bytes).await;
    let old = old.header("location").unwrap().to_owned();
    let half = bytes.len() / 2;
    let first = patch(address, &ana_who, &old, 0, &bytes[..half]).await;
    assert_eq!(first.status, 204, "{}", first.body);
    let id: i64 = old.rsplit('/').next().unwrap().parse().unwrap();
    sqlx::query("UPDATE uploads SET touched_at = now() - interval '2 days' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let file = storage.upload_file(ana, id, "mp4");
    assert!(file.exists());
    let fresh = announce(address, &bia_who, "fresh.mp4", &bytes).await;
    let fresh = fresh.header("location").unwrap().to_owned();

    assert_eq!(state.library.sweep_uploads().await.unwrap(), 1);
    assert!(!file.exists(), "what arrived of it is gone");
    let head = |who: String, location: String| async move {
        send(
            address,
            "HEAD",
            &location,
            &[&who, "Tus-Resumable: 1.0.0"],
            &[],
        )
        .await
    };
    assert_eq!(head(ana_who, old).await.status, 404);
    assert_eq!(head(bia_who, fresh).await.status, 200, "a fresh one stays");
}
