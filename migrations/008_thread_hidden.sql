alter table threads
ADD COLUMN hidden_at TIMESTAMPTZ,
ADD COLUMN closed_at TIMESTAMPTZ;

alter table posts
DROP COLUMN hidden_at;
