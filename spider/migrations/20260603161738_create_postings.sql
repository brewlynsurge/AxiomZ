-- Add migration script here
CREATE TABLE postings (
    term_id BIGINT REFERENCES terms(id) ON DELETE CASCADE,
    doc_id BIGINT REFERENCES documents(id) ON DELETE CASCADE,
    tf SMALLINT NOT NULL,

    PRIMARY KEY(term_id, doc_id)
);

CREATE INDEX postings_doc_id_idx
ON postings(doc_id);

-- ----------------------------------------------
-- Later add, field_mask SMALLINT DEFAULT 0,
-- For distinguish between title, url, body, etc

-- For field mask
-- TITLE       = 0001
-- DESCRIPTION = 0010
-- BODY        = 0100
-- URL         = 1000