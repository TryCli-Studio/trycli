-- Add up migration script here
CREATE TABLE IF NOT EXISTS projects (
    owner_username TEXT NOT NULL, 
    slug TEXT NOT NULL,
    image_tag TEXT NOT NULL, 
    markdown TEXT NOT NULL,
    shell TEXT NOT NULL DEFAULT '/bin/bash', 
    owner_id BIGINT, 
    PRIMARY KEY (owner_username, slug)
);