alter table posts
ADD COLUMN removed_at TIMESTAMPTZ;

alter table attachments
ADD COLUMN removed_at TIMESTAMPTZ;
