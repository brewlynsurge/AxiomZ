-- Add migration script here
CREATE TABLE terms (
    term_id BIGSERIAL PRIMARY KEY,
    term TEXT UNIQUE NOT NULL,
    doc_freq INTEGER DEFAULT 0
);