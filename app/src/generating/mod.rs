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
//! **Three vendors, said apart.** Shots are priced by Veo's table, stills by
//! Gemini's and lines by ElevenLabs', and they are orders of magnitude apart —
//! ninety-six cents against about two. So they are counted, subtotalled and
//! reported as separate groups: a single figure covering all of them is
//! arithmetic nobody can act on, because it does not say which part to go and
//! change.
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
use scorsese_core::{AssetKind, Project};
use scorsese_providers::credentials::Settings;
use scorsese_providers::prices::dollars;
use scorsese_providers::quote::Item;

use crate::project::Open;
use quote::Quote;
use worker::{Pass, Working};

/// How often to ask after work in flight.
///
/// Ten seconds, matching the library's own poll interval: what is being waited
/// on takes minutes, and a tighter loop would be a request a second at a vendor
/// to learn the same thing.
const SWEEP_EVERY: Duration = Duration::from_secs(10);

/// How tall the list of briefs grows before it scrolls, in points.
const LIST_HEIGHT: f32 = 360.0;

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
    /// The last quote, and the document it was read from.
    ///
    /// Kept rather than read on every repaint, because the library's quote
    /// reads every still a brief names off disk to hash it. Read again when
    /// the document changes, when the dialog opens, and when a pass reports —
    /// the three moments what is in `generated/` can have moved.
    quoted: Option<(Project, Quote)>,
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
        self.quoted = None;
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
            self.quoted = None;
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
        let quote = self.quote(open, settings);

        if let Some(why) = quote.unquotable() {
            ui.label(RichText::new(why).color(ui.visuals().warn_fg_color));
        } else if quote.free() {
            ui.label("Nothing in this project would be paid for.");
        }
        // Every generated asset has a row, made ones included, so a long
        // project scrolls here rather than pushing the total off the screen.
        let project = &open.project;
        egui::ScrollArea::vertical()
            .max_height(LIST_HEIGHT)
            .show(ui, |ui| {
                group(
                    ui,
                    "SHOTS",
                    &quote.of_kind(project, AssetKind::GeneratedVideo),
                );
                group(
                    ui,
                    "STILLS",
                    &quote.of_kind(project, AssetKind::GeneratedImage),
                );
                group(
                    ui,
                    "NARRATION",
                    &quote.of_kind(project, AssetKind::GeneratedAudio),
                );
            });

        if !quote.free() {
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

    /// The quote for the document as it stands, against the ceiling in
    /// `settings` — see [`Generating::quoted`] for when it is read again.
    fn quote(&mut self, open: &Open, settings: &Settings) -> Quote {
        let current = self
            .quoted
            .as_ref()
            .filter(|(read, _)| *read == open.project)
            .map(|(_, quote)| quote.clone());
        let mut quote = current.unwrap_or_else(|| {
            let quote = Quote::of(&open.project, &open.root, settings);
            self.quoted = Some((open.project.clone(), quote.clone()));
            quote
        });
        quote.ceiling_cents = settings.budget_cents;
        quote
    }

    /// The buttons, and what each of them is allowed to do.
    fn actions(&mut self, ui: &mut Ui, open: &mut Open, quote: &Quote) {
        let busy = self.working.is_some();
        ui.horizontal(|ui| {
            // Refused here as well as in the library, so the number and the
            // button agree: a run the library would refuse must not look
            // available.
            let allowed = !busy && !quote.free() && !quote.over_budget();
            if ui
                .add_enabled(allowed, egui::Button::new("Generate"))
                .on_disabled_hover_text(if quote.over_budget() {
                    "This run would cross the ceiling in settings"
                } else if busy {
                    "Already working"
                } else {
                    "Nothing to pay for"
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
/// **Groups rather than one list**, because the vendors' prices are orders of
/// magnitude apart: a 96¢ shot and a 2¢ line summed into a single figure is a
/// number nobody can act on, and it is not obvious from the rows which part of
/// it a person should go and edit. A heading, how many would be paid for and a
/// subtotal answer that before the grand total is read.
///
/// Each row is the library's own sentence — the price leads it when there is
/// one, and otherwise it says why nothing is paid — so this window and
/// `scorsese generate` describe a brief in the same words.
fn group(ui: &mut Ui, heading: &str, items: &[&Item]) {
    if items.is_empty() {
        return;
    }
    let charged: Vec<u64> = items
        .iter()
        .filter_map(|item| Some(item.charge.as_ref()?.cents))
        .collect();
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(crate::theme::marks::subheading(heading));
        ui.label(
            RichText::new(format!(
                "{} to pay for · {}",
                charged.len(),
                dollars(charged.iter().sum())
            ))
            .weak()
            .small(),
        );
    });
    for item in items {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&item.subject).strong());
            // The library names the asset because a terminal prints one line
            // per brief; this row already has, and saying it twice stutters.
            let named = format!("— {} ", item.subject);
            let says = RichText::new(item.says.replace(&named, "— ")).small();
            ui.label(if item.charge.is_some() {
                says
            } else {
                says.weak()
            });
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
