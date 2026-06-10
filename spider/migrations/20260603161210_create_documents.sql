-- Add migration script here
CREATE TABLE documents (
    id BIGSERIAL PRIMARY KEY,
    
    url TEXT NOT NULL,
    url_hash BIGINT NOT NULL,
    
    title TEXT,
    description TEXT DEFAULT NULL,

    document_len INTEGER NOT NULL,
    pagerank REAL NOT NULL DEFAULT 0,
    
    UNIQUE(url_hash)
);

CREATE INDEX documents_url_hash_idx
ON documents(url_hash);