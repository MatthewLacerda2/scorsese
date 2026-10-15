//! Finding an id nothing is using: one search, whatever the id names.
//!
//! Ids are what a person types into `project.json`, so a new one stays legible
//! — a word and a number, never a hash. Each caller keeps its own scheme
//! (`v3`, `sfx-2`, `title-2`, `v-dissolve-1`); what they share is the search,
//! and the bound that makes it a proof rather than a loop that could spin.

/// The first of `{prefix}{first}`, `{prefix}{first + 1}`… that `taken` refuses.
///
/// **Counted to a bound rather than searched until something gives.** `taken`
/// can only hold `limit` ids, so one of the `limit + 1` numbers from `first`
/// is always free and the range is a proof rather than a guess. A bare `loop`
/// would find the same id and would also be a naming rule that *can* spin —
/// the mutation signal hangs on it rather than failing, which is what a real
/// spin would do to a test run.
pub(crate) fn numbered(
    prefix: &str,
    first: usize,
    limit: usize,
    taken: impl Fn(&str) -> bool,
) -> String {
    (first..=first + limit)
        .map(|number| format!("{prefix}{number}"))
        .find(|candidate| !taken(candidate))
        .expect("`limit` ids cannot take every one of `limit + 1` numbers")
}

/// `wanted` if nothing has it, or `wanted-2`, `wanted-3`… — the first that is
/// free. The rule an imported asset's id and a placed clip's already follow, so
/// an inserted clip reads like any other.
///
/// Bounded as [`numbered`] is: `taken` holds at most `limit` ids.
pub(crate) fn free(wanted: &str, limit: usize, taken: impl Fn(&str) -> bool) -> String {
    if !taken(wanted) {
        return wanted.to_owned();
    }
    numbered(&format!("{wanted}-"), 2, limit, taken)
}

#[cfg(test)]
mod tests {
    use super::{free, numbered};

    #[test]
    fn a_free_id_is_kept_and_a_taken_one_is_suffixed_until_free() {
        let taken = ["title", "title-2"];
        let is_taken = |id: &str| taken.contains(&id);
        assert_eq!(free("logo", taken.len(), is_taken), "logo");
        assert_eq!(free("title", taken.len(), is_taken), "title-3");
    }

    #[test]
    fn a_numbered_id_is_the_lowest_free_number_from_the_first() {
        let taken = ["v1", "v2", "v4"];
        let is_taken = |id: &str| taken.contains(&id);
        assert_eq!(numbered("v", 1, taken.len(), is_taken), "v3");
        assert_eq!(numbered("v", 4, taken.len(), is_taken), "v5");
    }

    #[test]
    fn every_number_taken_but_the_last_still_finds_the_last() {
        let taken = ["a-2", "a-3"];
        let is_taken = |id: &str| taken.contains(&id);
        assert_eq!(numbered("a-", 2, taken.len(), is_taken), "a-4");
    }
}
