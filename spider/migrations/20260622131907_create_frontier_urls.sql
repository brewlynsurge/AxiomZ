-- Add migration script here
CREATE TABLE frontier_urls (
    id BIGSERIAL PRIMARY KEY,
    url TEXT NOT NULL UNIQUE
);