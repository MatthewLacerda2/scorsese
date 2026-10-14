//! The assistant's vendor-neutral seam (`src/chat`): Claude's reply read as
//! the neutral record, the record written back in the wire, and a history
//! replayed across a change of model — never a real call.

mod anthropic;
mod edges;
mod seam;
