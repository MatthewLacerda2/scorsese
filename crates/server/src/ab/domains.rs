//! Company or personal account, from the email's domain (#1553).
//!
//! An address at a free mail provider is a person's own; any other domain is
//! taken to be a company's. The list is the providers Brazilian and
//! international visitors sign in with; a domain missing from it reads as a
//! company, so the report's company share errs high, never hides one.

/// Domains whose addresses belong to people, not companies.
const PERSONAL: &[&str] = &[
    "aol.com",
    "bol.com.br",
    "gmail.com",
    "gmx.com",
    "googlemail.com",
    "hotmail.com",
    "hotmail.com.br",
    "icloud.com",
    "ig.com.br",
    "live.com",
    "mac.com",
    "mail.com",
    "me.com",
    "msn.com",
    "outlook.com",
    "outlook.com.br",
    "pm.me",
    "proton.me",
    "protonmail.com",
    "terra.com.br",
    "uol.com.br",
    "yahoo.com",
    "yahoo.com.br",
    "yandex.com",
    "zoho.com",
];

/// Whether `email` is at a company's domain rather than a free provider's.
pub fn is_company(email: &str) -> bool {
    let Some((_, domain)) = email.rsplit_once('@') else {
        return false;
    };
    let domain = domain.trim().to_ascii_lowercase();
    !domain.is_empty() && !PERSONAL.contains(&domain.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_providers_are_personal() {
        for email in [
            "ana@gmail.com",
            "ana@HOTMAIL.com",
            "ana@outlook.com",
            "ana@live.com",
            "ana@yahoo.com.br",
            "ana@icloud.com",
            "ana@proton.me",
            "ana@bol.com.br",
            "ana@uol.com.br",
            "ana@terra.com.br",
        ] {
            assert!(!is_company(email), "{email}");
        }
    }

    #[test]
    fn any_other_domain_is_a_company() {
        assert!(is_company("ana@padaria-sao-jorge.com.br"));
        assert!(is_company("ana@mail.gmail.com.example"));
        assert!(!is_company("not an address"));
    }

    #[test]
    fn the_list_is_sorted_lowercase_and_unique() {
        let mut sorted = PERSONAL.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, PERSONAL);
        assert!(
            PERSONAL
                .iter()
                .all(|domain| *domain == domain.to_ascii_lowercase())
        );
    }
}
