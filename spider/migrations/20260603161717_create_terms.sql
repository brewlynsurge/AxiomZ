-- Add migration script here
CREATE TABLE terms (
    id BIGSERIAL PRIMARY KEY,
    term TEXT UNIQUE NOT NULL,
    document_freq INTEGER NOT NULL DEFAULT 0
);
