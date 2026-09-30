//! Keeping a template's ids where they are free, and suffixing them where not.

/// `wanted` if nothing has it, or `wanted-2`, `wanted-3`… — the first that is
/// free. The rule an imported asset's id and a placed clip's already follow, so
/// an inserted clip reads like any other.
///
/// Counted to a bound, as a track's number is: `taken` can only hold so many
/// ids, so one of the first `limit + 1` suffixes is free and the search is a
/// proof rather than a loop that could spin.
pub(crate) fn free(wanted: &str, limit: usize, taken: impl Fn(&str) -> bool) -> String {
    if !taken(wanted) {
        return wanted.to_owned();
    }
    (2..=limit + 2)
        .map(|suffix| format!("{wanted}-{suffix}"))
        .find(|candidate| !taken(candidate))
        .expect("`limit` ids cannot take every one of `limit + 1` suffixes")
}

#[cfg(test)]
mod tests {
    use super::free;

    #[test]
    fn a_free_id_is_kept_and_a_taken_one_is_suffixed_until_free() {
        let taken = ["title", "title-2"];
        let is_taken = |id: &str| taken.contains(&id);
        assert_eq!(free("logo", taken.len(), is_taken), "logo");
        assert_eq!(free("title", taken.len(), is_taken), "title-3");
    }
}
