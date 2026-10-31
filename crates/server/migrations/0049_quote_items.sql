-- A quote's token keeps every brief it would pay for, as the confirmation
-- box lists them: [{subject, says, micros, brief, description}], written when
-- the token is issued from the quote it is bound to. Their micros add up to
-- the row's. A row issued before this has none, and its box lists no items.
ALTER TABLE quotes ADD COLUMN items JSONB NOT NULL DEFAULT '[]';
