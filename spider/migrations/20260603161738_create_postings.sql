-- Add migration script here
CREATE TABLE postings (
    term_id BIGINT REFERENCES terms(term_id),
    doc_id BIGINT REFERENCES documents(doc_id),

    term_freq INTEGER NOT NULL,

    -- Store compressed positional data later
    positions BYTEA,

    PRIMARY KEY(term_id, doc_id)
);

CREATE INDEX postings_term_doc_idx
ON postings(term_id, doc_id);

CREATE INDEX postings_doc_term_idx
ON postings(doc_id, term_id);