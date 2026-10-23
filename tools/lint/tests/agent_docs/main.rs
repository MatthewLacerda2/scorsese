//! What an agent loads: `CLAUDE.md` and skills held to their caps, a skill's
//! frontmatter to what Claude loads, and both to citing no history.

mod caps;
mod frontmatter;
mod history;
mod skill;
mod tree;

use scorsese_lint::Finding;

/// The rules of the findings, in order.
fn rules(findings: &[Finding]) -> Vec<&'static str> {
    findings.iter().map(|finding| finding.rule).collect()
}

/// `n` lines of plain text.
fn lines(n: usize) -> String {
    "a rule\n".repeat(n)
}

/// A `SKILL.md` with this frontmatter block and a short body.
fn skill_md(frontmatter: &str) -> String {
    format!("---\n{frontmatter}\n---\n\n# A skill\n")
}
