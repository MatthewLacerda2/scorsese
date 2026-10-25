//! A sound's peaks kept in step with the file (#1572): a truncated file is
//! read again rather than served to be kept, and a deleted sound takes its
//! peaks with it.

use scorsese_core::hash_bytes;
use sqlx::postgres::PgPool;

use super::peaks::sound;
use crate::common::{self, request};
use crate::{member, upload};

#[sqlx::test]
async fn peaks_shorter_than_their_rate_are_never_served(pool: PgPool) {
    let files = common::files("peaks-short");
    let storage = files.storage.clone();
    let bytes = sound(&common::scratch::dir("peaks-short-media"));
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (user, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "line.wav", &bytes).await;
    let kept = storage.peaks(user, &hash_bytes(&bytes));
    std::fs::create_dir_all(kept.parent().unwrap()).unwrap();
    std::fs::write(&kept, [0, 0]).unwrap();

    let asked = request(
        address,
        "GET",
        &format!("/api/library/{id}/peaks"),
        &[&ana],
        None,
    )
    .await;
    assert_eq!(
        (asked.status, &asked.json()["pending"]),
        (404, &true.into()),
        "a truncated file is read again, not kept for a year"
    );
}

#[sqlx::test]
async fn a_deleted_sound_takes_its_peaks_with_it(pool: PgPool) {
    let files = common::files("peaks-delete");
    let storage = files.storage.clone();
    let bytes = sound(&common::scratch::dir("peaks-delete-media"));
    let (address, _) = common::serve_with(pool.clone(), files).await;
    let (user, ana) = member(&pool, "ana@example.com").await;
    let id = upload(address, &ana, "line.wav", &bytes).await;
    let kept = storage.peaks(user, &hash_bytes(&bytes));
    std::fs::create_dir_all(kept.parent().unwrap()).unwrap();
    std::fs::write(&kept, 40.0_f32.to_le_bytes()).unwrap();

    let path = format!("/api/library/{id}");
    assert_eq!(
        request(address, "DELETE", &path, &[&ana], None)
            .await
            .status,
        204
    );
    assert!(!kept.exists(), "the peaks went with the file");
}
