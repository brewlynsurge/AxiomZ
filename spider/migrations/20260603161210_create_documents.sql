-- Add migration script here
CREATE TABLE documents (
    doc_id BIGSERIAL PRIMARY KEY,
    url TEXT UNIQUE NOT NULL,
    url_hash BIGINT NOT NULL,
    title TEXT,
    description TEXT DEFAULT NULL,
    UNIQUE(url_hash, url)
);

CREATE INDEX documents_url_hash_idx
ON documents(url_hash);