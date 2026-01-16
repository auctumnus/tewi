-- stored as:
-- $ATTACHMENTS_FOLDER/xx/yyyy-yyyyy....
-- where xx are the first two hex digits of the UUID,
-- and yyyyy... is the full uuid
-- thumbnails are stored similarly in $THUMBNAILS_FOLDER
create table attachments (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    mime_type VARCHAR NOT NULL,
    size INT NOT NULL, -- in bytes
    width INT,  -- null if not an image
    height INT, -- null if not an image

    thumbnail_width INT,  -- null if no thumbnail
    thumbnail_height INT, -- null if no thumbnail

    original_filename VARCHAR NOT NULL,
    spoilered BOOLEAN NOT NULL DEFAULT FALSE
);

create table threads (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    board_id UUID REFERENCES boards(id) ON DELETE CASCADE,
    op_post UUID NOT NULL,

    last_post_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_sticky BOOLEAN NOT NULL DEFAULT FALSE
);

create table posts (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    thread_id UUID REFERENCES threads(id) ON DELETE CASCADE,
    ip_id UUID REFERENCES ips(id) ON DELETE SET NULL,
    associated_ban_id UUID REFERENCES bans(id) ON DELETE SET NULL,
    attachment_id UUID REFERENCES attachments(id) ON DELETE SET NULL,

    post_number INT NOT NULL,
    title VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    content TEXT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    hidden_at TIMESTAMPTZ
);