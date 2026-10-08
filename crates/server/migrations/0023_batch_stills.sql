-- Stills ordered in a half-price batch (#947, the web side of #894). A still
-- drawn now comes back on the call and never had a ticket (0012); one ordered
-- in a batch is in flight for up to a day, and the batch job's name is the only
-- record that it will be billed -- so it is kept here the moment Google
-- accepts, as 0004 keeps a shot's. `batch` says which way a row was ordered,
-- for the history and for a dispute: its estimate is the half price.
ALTER TABLE image_generations
    ADD COLUMN batch  BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN ticket TEXT;
