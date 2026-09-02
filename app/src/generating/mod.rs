//! Generating from the window: what it will cost, the confirmation, and what
//! is still going.
//!
//! **The exception that earns its place.** Everything else this window offers
//! is an editing operation — scrub, nudge, change a plain value — and anything
//! with structure to it is a sentence to an assistant rather than a menu. This
//! is not an editing operation. It is the moment money is spent, and a number
//! on screen before a confirmation is the difference between a decision and a
//! surprise. A spend figure only a terminal can see is not visible to the
//! person paying for it.
//!
//! Nothing here is a new capability: it is `scorsese generate`, given a
//! surface, calling the same functions with the same rate tables and the same
//! ceiling.
//!
//! **Two vendors, said apart.** Shots are priced by Veo's table and lines by
//! ElevenLabs', and the two are two orders of magnitude apart — ninety-six
//! cents against about two. So they are counted, subtotalled and reported as
//! separate groups: a single figure covering both is arithmetic nobody can act
//! on, because it does not say which half to go and change.
//!
//! **What is drawn and what is spent read the ceiling in different places, and
//! that is deliberate.** Everything here draws the [`Settings`] the window was
//! handed, so what appears on screen is a stated machine rather than whichever
//! one happens to be running — which is what a snapshot needs to mean anything.
//! [`worker`] asks the machine itself, on its own thread, at the moment it
//! submits: that is when the money leaves, and a ceiling is worth what it says
//! then rather than what it said when the window opened.

mod quote;
mod worker;

use std::time::{Duration, Instant};

use egui::{Context, RichText, Ui, Window};
use scorsese_providers::credentials::Settings;
use scorsese_providers::prices::dollars;

use crate::project::Open;
use quote::Quote;
use worker::{Pass, Working};

/// How often to ask after work in flight.
///
/// Ten seconds, matching the library's own poll interval: what is being waited
/// on takes minutes, and a tighter loop would be a request a second at a vendor
/// to learn the same thing.
const SWEEP_EVERY: Duration = Duration::from_secs(10);

/// The dialog, and whatever is running behind it.
#[derive(Default)]
pub(crate) struct Generating {
    /// Whether the dialog is open.
    open: bool,
    /// The pass in flight, when there is one.
    working: Option<Working>,
    /// What the last pass said.
    said: Option<String>,
    /// How many shots are still going, from the last report.
    in_flight: usize,
    /// When the last sweep went out, so the next one is on a timer rather than
    /// on every repaint.
    swept: Option<Instant>,
    /// True once a pass failed for a reason repeating will not fix — no key,
    /// over the ceiling. Sweeping stops rather than saying it every ten
    /// seconds.
    stopped: bool,
}

impl Generating {
    /// Forgets everything, for when a different project is opened.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Opens the dialog.
    pub(crate) fn open(&mut self) {
        self.open = true;
        self.stopped = false;
    }

    /// How many shots are in flight, for the bar to say so without opening
    /// anything.
    pub(crate) fn in_flight(&self) -> usize {
        self.in_flight
    }

    /// Polls the worker and keeps the sweep going. Called every repaint; never
    /// blocks.
    pub(crate) fn poll(&mut self, open: &mut Open) {
        if let Some(working) = self.working.as_mut()
            && let Some(report) = working.finished()
        {
            self.working = None;
            self.in_flight = report.in_flight;
            self.stopped = report.failed;
            self.said = Some(report.said);
            self.swept = Some(Instant::now());
        }
        // A sweep spends nothing by construction, which is the whole reason it
        // is safe to run on a timer at all.
        let due = self.swept.is_none_or(|last| last.elapsed() >= SWEEP_EVERY);
        if self.working.is_none() && self.in_flight > 0 && !self.stopped && due {
            self.working = Some(Working::start(Pass::Sweep, &open.project, &open.root));
        }
    }

    /// Draws the dialog, when it is open.
    ///
    /// `settings` is the window's copy of what this machine knows — see
    /// [`Scorsese`](crate::Scorsese) for why it is passed in rather than read
    /// here. Taken by `&mut` because the ceiling field below sets it.
    pub(crate) fn show(&mut self, ctx: &Context, open: &mut Open, settings: &mut Settings) {
        if !self.open {
            return;
        }
        let mut showing = self.open;
        Window::new("Generate")
            .open(&mut showing)
            .resizable(false)
            .show(ctx, |ui| self.body(ui, open, settings));
        self.open = showing;
    }

    /// What the dialog says and offers.
    fn body(&mut self, ui: &mut Ui, open: &mut Open, settings: &mut Settings) {
        let quote = Quote::of(&open.project, settings);

        if quote.empty() && quote.in_flight == 0 {
            ui.label("Every shot and every line in this project is already made.");
        }
        group(ui, "SHOTS", &quote.shots);
        group(ui, "NARRATION", &quote.lines);
        if quote.in_flight > 0 {
            ui.label(
                RichText::new(format!(
                    "{} already generating — not sent again",
                    quote.in_flight
                ))
                .weak()
                .small(),
            );
        }

        if !quote.empty() {
            ui.separator();
            ui.label(RichText::new(format!("About {}", quote.total())).strong());
            // Said every time, not once in a tooltip. No provider reports what
            // a generation cost, so a total on screen is our arithmetic — and
            // this is where somebody reads it before spending more.
            ui.label(
                RichText::new("our calculation from published rates, never a bill")
                    .weak()
                    .small(),
            );
        }

        budget(ui, &quote, settings);
        ui.separator();
        self.actions(ui, open, &quote);

        if let Some(said) = &self.said {
            ui.add_space(6.0);
            ui.label(RichText::new(said).small());
        }
    }

    /// The buttons, and what each of them is allowed to do.
    fn actions(&mut self, ui: &mut Ui, open: &mut Open, quote: &Quote) {
        let busy = self.working.is_some();
        ui.horizontal(|ui| {
            // Refused here as well as in the library, so the number and the
            // button agree: a run the library would refuse must not look
            // available.
            let allowed = !busy && !quote.empty() && !quote.over_budget();
            if ui
                .add_enabled(allowed, egui::Button::new("Generate"))
                .on_disabled_hover_text(if quote.over_budget() {
                    "This run would cross the ceiling in settings"
                } else if busy {
                    "Already working"
                } else {
                    "Nothing to generate"
                })
                .clicked()
            {
                self.said = Some(String::from("handing the briefs over…"));
                self.working = Some(Working::start(Pass::Submit, &open.project, &open.root));
            }

            if ui
                .add_enabled(!busy, egui::Button::new("Check for finished"))
                .on_hover_text("Asks after what is already generating. Spends nothing.")
                .clicked()
            {
                self.stopped = false;
                self.working = Some(Working::start(Pass::Sweep, &open.project, &open.root));
            }
        });
        if busy {
            ui.spinner();
        }
    }
}

/// One kind of generation, under a heading that counts and totals it.
///
/// **Two groups rather than one list**, because the two vendors' prices are two
/// orders of magnitude apart: a 96¢ shot and a 2¢ line summed into a single
/// figure is a number nobody can act on, and it is not obvious from the rows
/// which half of it a person should go and edit. A heading, a count and a
/// subtotal answer that before the grand total is read.
fn group(ui: &mut Ui, heading: &str, priced: &[quote::Priced]) {
    if priced.is_empty() {
        return;
    }
    let subtotal: u64 = priced.iter().map(|one| one.cents).sum();
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(crate::theme::marks::subheading(heading));
        ui.label(
            RichText::new(format!("{} · {}", priced.len(), dollars(subtotal)))
                .weak()
                .small(),
        );
    });
    for one in priced {
        ui.horizontal(|ui| {
            ui.label(RichText::new(one.asset.as_str()).strong());
            ui.label(RichText::new(&one.shape).weak().small());
            ui.label(dollars(one.cents));
        });
    }
}

/// The ceiling, the spend, and the field that sets one.
///
/// Here rather than behind a settings menu of its own, because this is the
/// moment the number matters: a person deciding whether to spend is the person
/// who would change the limit.
fn budget(ui: &mut Ui, quote: &Quote, settings: &mut Settings) {
    ui.add_space(4.0);
    match quote.ceiling() {
        Some(line) => ui.label(RichText::new(line).weak().small()),
        None => ui.label(
            RichText::new(format!(
                "{} spent so far · no ceiling set",
                dollars(quote.spent_cents)
            ))
            .weak()
            .small(),
        ),
    };
    if quote.over_budget() {
        ui.label(
            RichText::new("This run would cross the ceiling.")
                .color(ui.visuals().warn_fg_color)
                .small(),
        );
    }

    let mut ceiling = settings.budget_cents.unwrap_or(0) / 100;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Ceiling $").small());
        if ui
            .add(egui::DragValue::new(&mut ceiling).range(0..=10_000))
            .on_hover_text("0 means no ceiling. A limit no flag can wave through.")
            .changed()
        {
            settings.budget_cents = (ceiling > 0).then_some(ceiling * 100);
            // Per machine, not per project — a key and a limit belong to
            // whoever is paying, and a project directory travels.
            let _ = settings.save();
        }
    });
}
